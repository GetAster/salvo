//! QuinnListener and its implementations.
use std::error::Error as StdError;
use std::fmt::{self, Debug, Formatter};
use std::future::IntoFuture;
use std::io::{Error as IoError, ErrorKind, Result as IoResult};
use std::marker::PhantomData;
use std::net::{SocketAddr, ToSocketAddrs};
use std::vec;

use futures_util::stream::{BoxStream, StreamExt};
use http::uri::Scheme;
use salvo_http3::quinn::Endpoint;
use salvo_http3::quinn::quinn::{Incoming, IncomingFuture};
use tokio_util::sync::CancellationToken;

use super::{QuinnConnection, QuinnCoupler};
use crate::Error;
use crate::conn::quinn::ServerConfig;
use crate::conn::{Accepted, Acceptor, Holding, IntoConfigStream, Listener};
use crate::fuse::{ArcFusePolicy, FuseAction, FuseConfig, FuseInfo, TransProto};
use crate::http::Version;

/// A wrapper of `Listener` with quinn.
pub struct QuinnListener<S, C, T, E> {
    config_stream: S,
    local_addr: T,
    _phantom: PhantomData<(C, E)>,
}
impl<S, C, T: Debug, E> Debug for QuinnListener<S, C, T, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuinnListener")
            .field("local_addr", &self.local_addr)
            .finish()
    }
}
impl<S, C, T, E> QuinnListener<S, C, T, E>
where
    S: IntoConfigStream<C> + Send + 'static,
    C: TryInto<ServerConfig, Error = E> + Send + 'static,
    T: ToSocketAddrs + Send,
    E: StdError + Send,
{
    /// Bind to socket address.
    #[inline]
    pub fn new(config_stream: S, local_addr: T) -> Self {
        Self {
            config_stream,
            local_addr,
            _phantom: PhantomData,
        }
    }
}
impl<S, C, T, E> Listener for QuinnListener<S, C, T, E>
where
    S: IntoConfigStream<C> + Send + 'static,
    C: TryInto<ServerConfig, Error = E> + Send + 'static,
    T: ToSocketAddrs + Send + 'static,
    E: StdError + Send + 'static,
{
    type Acceptor = QuinnAcceptor;

    async fn try_bind(self) -> crate::Result<Self::Acceptor> {
        let Self {
            config_stream,
            local_addr,
            ..
        } = self;
        let socket = local_addr
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| IoError::new(ErrorKind::AddrNotAvailable, "No address available"))?;

        let mut config_stream = config_stream.into_stream().boxed();
        let initial = config_stream.next().await.ok_or_else(|| {
            Error::other("quinn: config stream ended before yielding an initial tls config")
        })?;
        let initial = initial
            .try_into()
            .map_err(|err| IoError::other(err.to_string()))?;
        let endpoint = Endpoint::server(initial, socket)?;
        let cancel_reload = CancellationToken::new();

        tracing::info!("quinn config loaded");
        tokio::spawn(reload_configs(
            config_stream,
            endpoint.clone(),
            cancel_reload.clone(),
        ));

        Ok(QuinnAcceptor::new(endpoint, socket, cancel_reload))
    }
}

async fn reload_configs<C, E>(
    mut config_stream: BoxStream<'static, C>,
    endpoint: Endpoint,
    cancel_reload: CancellationToken,
) where
    C: TryInto<ServerConfig, Error = E> + Send + 'static,
    E: StdError + Send + 'static,
{
    loop {
        tokio::select! {
            _ = cancel_reload.cancelled() => break,
            next = config_stream.next() => {
                let Some(config) = next else {
                    break;
                };
                match config.try_into() {
                    Ok(config) => {
                        endpoint.set_server_config(Some(config));
                        tracing::info!("quinn config changed");
                    }
                    Err(err) => {
                        tracing::error!(error = ?err, "quinn: invalid tls config, keeping previous config");
                    }
                }
            }
        }
    }
}

/// A wrapper of `Acceptor` with quinn.
pub struct QuinnAcceptor {
    socket: SocketAddr,
    holdings: Vec<Holding>,
    endpoint: Endpoint,
    cancel_reload: CancellationToken,
    // An accepted QUIC incoming that has passed `endpoint.accept()` but not yet admission.
    // Parking it here keeps it alive if the `accept` future is dropped during async admission
    // (e.g. a `JoinedListener`'s `select!` picking the other listener).
    pending: Option<Incoming>,
    // Once admitted, retain the handshake and its original metadata across cancellation.
    handshaking: Option<PendingHandshake>,
}

struct PendingHandshake {
    incoming: IncomingFuture,
    fuse_config: Option<FuseConfig>,
    local_addr: crate::conn::SocketAddr,
    remote_addr: SocketAddr,
    deadline: Option<tokio::time::Instant>,
}

impl Debug for QuinnAcceptor {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("QuinnAcceptor")
            .field("socket", &self.socket)
            .field("holdings", &self.holdings)
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

impl QuinnAcceptor {
    /// Create a new `QuinnAcceptor`.
    #[must_use]
    pub fn new(endpoint: Endpoint, socket: SocketAddr, cancel_reload: CancellationToken) -> Self {
        let holding = Holding {
            local_addr: socket.into(),
            http_versions: vec![Version::HTTP_3],
            http_scheme: Scheme::HTTPS,
        };
        Self {
            socket,
            holdings: vec![holding],
            endpoint,
            cancel_reload,
            pending: None,
            handshaking: None,
        }
    }
}

impl Drop for QuinnAcceptor {
    fn drop(&mut self) {
        self.cancel_reload.cancel();
    }
}

impl Acceptor for QuinnAcceptor {
    type Coupler = QuinnCoupler;
    type Stream = QuinnConnection;

    fn holdings(&self) -> &[Holding] {
        &self.holdings
    }

    async fn accept(
        &mut self,
        fuse_policy: Option<ArcFusePolicy>,
    ) -> IoResult<Accepted<Self::Coupler, Self::Stream>> {
        loop {
            if self.handshaking.is_none() {
                // Admission may be cancelled too: keep the unadmitted incoming parked.
                let new_conn = match self.pending.take() {
                    Some(incoming) => incoming,
                    None => match self.endpoint.accept().await {
                        Some(incoming) => incoming,
                        None => return Err(IoError::other("quinn accept error")),
                    },
                };
                let remote_addr = new_conn.remote_address();
                let local_addr = self.holdings[0].local_addr.clone();
                self.pending = Some(new_conn);
                let fuse_config = match &fuse_policy {
                    Some(policy) => match policy
                        .decide(&FuseInfo {
                            trans_proto: TransProto::Quic,
                            remote_addr: remote_addr.into(),
                            local_addr: local_addr.clone(),
                        })
                        .await
                    {
                        FuseAction::Accept(config) => Some(config),
                        FuseAction::Reject => {
                            self.pending = None;
                            continue;
                        }
                    },
                    None => None,
                };
                // Do not await between consuming the Incoming and storing its future.
                // Subsequent calls resume this handshake without re-running admission or
                // extending the original timeout, even if their policy has changed.
                let deadline = fuse_config
                    .and_then(|config| config.tls_handshake_timeout)
                    .map(|timeout| tokio::time::Instant::now() + timeout);
                self.handshaking = Some(PendingHandshake {
                    incoming: self.pending.take().expect("incoming parked above").into_future(),
                    fuse_config,
                    local_addr,
                    remote_addr,
                    deadline,
                });
            }
            let handshake = self.handshaking.as_mut().expect("handshake parked above");
            // QUIC uses the fuse handshake deadline here and the body timeout in H3.
            // Transport idle/write-stall limits remain governed by Quinn itself.
            // Preserve Tokio's ready-first timeout semantics: a handshake completed
            // while accept was unpolled can still win against an elapsed timer.
            let connected = match handshake.deadline {
                Some(deadline) => {
                    if let Ok(result) = tokio::time::timeout_at(deadline, &mut handshake.incoming).await {
                        result
                    } else {
                        self.handshaking = None;
                        continue;
                    }
                }
                None => (&mut handshake.incoming).await,
            };
            let PendingHandshake {
                fuse_config,
                local_addr,
                remote_addr,
                ..
            } = self.handshaking.take().expect("completed handshake");
            return match connected {
                Ok(conn) => Ok(Accepted {
                    coupler: QuinnCoupler,
                    stream: QuinnConnection::new(conn),
                    fuse_config,
                    conn_ctrl: crate::conn::ConnCtrl::new(),
                    local_addr,
                    remote_addr: remote_addr.into(),
                    http_scheme: self.holdings[0].http_scheme.clone(),
                }),
                Err(e) => Err(IoError::other(e.to_string())),
            };
        }
    }
}

#[cfg(test)]
#[path = "accept_cancellation_tests.rs"]
mod accept_cancellation_tests;
