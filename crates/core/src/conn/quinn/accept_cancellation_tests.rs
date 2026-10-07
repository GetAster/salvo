//! Bounded regressions for cancellation of a real QUIC accept.
use super::*;
use crate::conn::rustls::{Keycert, RustlsConfig, default_crypto_provider};
use futures_util::future::poll_fn;
use std::future::Future;
use std::net::Ipv4Addr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Poll;
use std::time::Duration;
use tokio::time::timeout;
use tokio_rustls::rustls;

const CERT: &[u8] = include_bytes!("../../../certs/chain.pem");
const KEY: &[u8] = include_bytes!("../../../certs/key.pem");
const STEP: Duration = Duration::from_secs(5);

fn endpoints() -> (QuinnAcceptor, ::quinn::Endpoint) {
    let config = RustlsConfig::new(Keycert::new().cert(CERT).key(KEY))
        .build_quinn_config()
        .expect("server config");
    let server = Endpoint::server(config, (Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    let addr = server.local_addr().unwrap();
    let acceptor = QuinnAcceptor::new(server, addr, CancellationToken::new());
    let mut roots = rustls::RootCertStore::empty();
    use rustls::pki_types::pem::PemObject;
    for cert in rustls::pki_types::CertificateDer::pem_slice_iter(CERT) {
        roots.add(cert.unwrap()).unwrap();
    }
    let mut tls = rustls::ClientConfig::builder_with_provider(default_crypto_provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let crypto = ::quinn::crypto::rustls::QuicClientConfig::try_from(tls).unwrap();
    let mut client = ::quinn::Endpoint::client((Ipv4Addr::LOCALHOST, 0).into()).unwrap();
    client.set_default_client_config(::quinn::ClientConfig::new(Arc::new(crypto)));
    (acceptor, client)
}

async fn incoming(acceptor: &mut QuinnAcceptor, client: &::quinn::Endpoint) -> ::quinn::Connecting {
    let connecting = client.connect(acceptor.socket, "localhost").unwrap();
    acceptor.pending = Some(timeout(STEP, acceptor.endpoint.accept()).await.unwrap().unwrap());
    connecting
}

async fn poll_pending<F: Future>(mut future: Pin<&mut F>) {
    assert!(matches!(poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))).await, Poll::Pending), "handshake must be pending");
}

async fn handshake_done(connecting: ::quinn::Connecting) -> ::quinn::Connection {
    let conn = timeout(STEP, connecting).await.unwrap().unwrap();
    timeout(STEP, async {
        while conn.stats().frame_rx.handshake_done == 0 {
            tokio::task::yield_now().await;
        }
    }).await.expect("client never received HANDSHAKE_DONE");
    conn
}

// Current-thread tests give us a deterministic first Pending poll: the server driver
// cannot finish the handshake until this task explicitly yields to it.
#[tokio::test]
async fn post_handshake_cancel_resumes_same_connection_and_original_fuse() {
    let (mut acceptor, client) = endpoints();
    let connecting = incoming(&mut acceptor, &client).await;
    let decisions = Arc::new(AtomicUsize::new(0));
    let count = decisions.clone();
    let config = FuseConfig::disabled().with_request_body_timeout(Duration::from_secs(17));
    let policy: ArcFusePolicy = Arc::new(move |_: &FuseInfo| {
        count.fetch_add(1, Ordering::SeqCst);
        FuseAction::Accept(config)
    });
    let mut accept = Box::pin(acceptor.accept(Some(policy)));
    poll_pending(accept.as_mut()).await;
    let conn = handshake_done(connecting).await;
    drop(accept);
    let changed: ArcFusePolicy = Arc::new(|_: &FuseInfo| {
        panic!("admitted connection must not run a changed policy")
    });
    let accepted = timeout(STEP, acceptor.accept(Some(changed)))
        .await.expect("cancelled accept did not resume the original connection").unwrap();
    assert_eq!(accepted.remote_addr.into_std().unwrap(), client.local_addr().unwrap());
    assert_eq!(accepted.local_addr.into_std().unwrap(), acceptor.socket);
    assert_eq!(accepted.fuse_config, Some(config));
    assert_eq!(decisions.load(Ordering::SeqCst), 1);
    assert!(conn.close_reason().is_none(), "peer closed: {:?}", conn.close_reason());
    // Traffic on the returned raw connection proves it is the original peer.
    let mut send = timeout(STEP, conn.open_uni()).await.unwrap().unwrap();
    timeout(STEP, send.write_all(b"resumed")).await.unwrap().unwrap();
    send.finish().unwrap();
    let mut recv = timeout(STEP, accepted.stream.quinn().accept_uni()).await.unwrap().unwrap();
    assert_eq!(timeout(STEP, recv.read_to_end(32)).await.unwrap().unwrap(), b"resumed");
    conn.close(0u32.into(), b"done");
}

#[tokio::test]
async fn repeated_cancellation_before_completion_resumes_same_connection() {
    let (mut acceptor, client) = endpoints();
    let connecting = incoming(&mut acceptor, &client).await;
    for _ in 0..3 {
        let mut accept = Box::pin(acceptor.accept(None));
        poll_pending(accept.as_mut()).await;
        drop(accept);
    }
    let conn = handshake_done(connecting).await;
    let accepted = timeout(STEP, acceptor.accept(None)).await.unwrap().unwrap();
    assert_eq!(accepted.remote_addr.into_std().unwrap(), client.local_addr().unwrap());
    assert!(conn.close_reason().is_none());
    conn.close(0u32.into(), b"done");
}

#[tokio::test]
async fn dropping_acceptor_releases_admitted_handshake() {
    let (mut acceptor, client) = endpoints();
    // Retain another endpoint handle to isolate handshake ownership from endpoint drop.
    let _server = acceptor.endpoint.clone();
    let connecting = incoming(&mut acceptor, &client).await;
    let mut accept = Box::pin(acceptor.accept(None));
    poll_pending(accept.as_mut()).await;
    let conn = handshake_done(connecting).await;
    drop(accept);
    drop(acceptor);
    match timeout(STEP, conn.closed()).await.unwrap() {
        ::quinn::ConnectionError::ApplicationClosed(close) => {
            assert_eq!(u64::from(close.error_code), 0);
            assert!(close.reason.is_empty());
        }
        error => panic!("expected implicit close, got {error:?}"),
    }
}

struct SlowPolicy;
#[crate::async_trait]
impl crate::fuse::FusePolicy for SlowPolicy {
    async fn decide(&self, _: &FuseInfo) -> FuseAction {
        std::future::pending().await
    }
}

#[tokio::test]
async fn cancellation_during_admission_keeps_incoming() {
    let (mut acceptor, client) = endpoints();
    let connecting = incoming(&mut acceptor, &client).await;
    let mut accept = Box::pin(acceptor.accept(Some(Arc::new(SlowPolicy))));
    poll_pending(accept.as_mut()).await;
    drop(accept);
    assert!(acceptor.pending.is_some());
    assert!(acceptor.handshaking.is_none());
    let (accepted, conn) = timeout(STEP, async {
        tokio::join!(acceptor.accept(None), connecting)
    }).await.unwrap();
    assert_eq!(accepted.unwrap().remote_addr.into_std().unwrap(), client.local_addr().unwrap());
    conn.unwrap().close(0u32.into(), b"done");
}

#[tokio::test]
async fn rejection_drops_incoming_and_accepts_next_peer() {
    let (mut acceptor, client) = endpoints();
    let _rejected = incoming(&mut acceptor, &client).await;
    let reject: ArcFusePolicy = Arc::new(|_: &FuseInfo| FuseAction::Reject);
    let mut accept = Box::pin(acceptor.accept(Some(reject)));
    poll_pending(accept.as_mut()).await;
    drop(accept);
    assert!(acceptor.pending.is_none());
    assert!(acceptor.handshaking.is_none());
    let connecting = client.connect(acceptor.socket, "localhost").unwrap();
    let (accepted, conn) = timeout(STEP, async {
        tokio::join!(acceptor.accept(None), connecting)
    }).await.unwrap();
    assert_eq!(accepted.unwrap().remote_addr.into_std().unwrap(), client.local_addr().unwrap());
    conn.unwrap().close(0u32.into(), b"done");
}

#[tokio::test]
async fn handshake_deadline_survives_repeated_cancellation() {
    let (mut acceptor, client) = endpoints();
    // Forward only the first client flight. Holding the relay without forwarding the
    // server's reply prevents completion, so expiration does not depend on scheduling.
    let relay = tokio::net::UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let _connecting = client.connect(relay.local_addr().unwrap(), "localhost").unwrap();
    let mut datagram = [0; 65536];
    let (len, _) = timeout(STEP, relay.recv_from(&mut datagram)).await.unwrap().unwrap();
    relay.send_to(&datagram[..len], acceptor.socket).await.unwrap();
    acceptor.pending = Some(timeout(STEP, acceptor.endpoint.accept()).await.unwrap().unwrap());
    let config = FuseConfig::disabled().with_tls_handshake_timeout(Duration::from_millis(50));
    let mut accept = Box::pin(acceptor.accept(Some(Arc::new(config))));
    poll_pending(accept.as_mut()).await;
    drop(accept);
    let deadline = acceptor.handshaking.as_ref().unwrap().deadline.unwrap();
    for _ in 0..3 {
        let mut accept = Box::pin(acceptor.accept(None));
        poll_pending(accept.as_mut()).await;
        drop(accept);
        assert_eq!(acceptor.handshaking.as_ref().unwrap().deadline, Some(deadline));
    }
    tokio::time::sleep_until(deadline).await;
    let mut accept = Box::pin(acceptor.accept(None));
    // The original expired handshake is discarded and accept waits for a fresh incoming.
    poll_pending(accept.as_mut()).await;
    drop(accept);
    assert!(acceptor.handshaking.is_none(), "deadline must not restart on resume");
    assert!(acceptor.pending.is_none());
}

#[tokio::test]
async fn completed_handshake_resumed_after_deadline_keeps_tokio_ready_first_semantics() {
    let (mut acceptor, client) = endpoints();
    let connecting = incoming(&mut acceptor, &client).await;
    let config = FuseConfig::disabled().with_tls_handshake_timeout(Duration::from_secs(1));
    let mut accept = Box::pin(acceptor.accept(Some(Arc::new(config))));
    poll_pending(accept.as_mut()).await;
    drop(accept);
    let deadline = acceptor.handshaking.as_ref().unwrap().deadline.unwrap();

    // The independent Quinn driver completes while accept is cancelled. Observe
    // server HANDSHAKE_DONE at the client before the original budget expires.
    // The stored IncomingFuture remains unpolled until the late resume below.
    let conn = timeout(deadline.saturating_duration_since(tokio::time::Instant::now()),
        handshake_done(connecting)).await.expect("handshake must complete within its original budget");
    assert!(tokio::time::Instant::now() < deadline);
    tokio::time::sleep_until(deadline).await;
    assert!(tokio::time::Instant::now() >= deadline);
    assert_eq!(acceptor.handshaking.as_ref().unwrap().deadline, Some(deadline));

    // Tokio Timeout polls the inner future before its elapsed timer. Keep that
    // existing contract rather than rejecting a connection that already completed
    // within budget solely because the next accept call was scheduled late.
    let accepted = timeout(STEP, acceptor.accept(None)).await.unwrap().unwrap();
    assert_eq!(accepted.remote_addr.into_std().unwrap(), client.local_addr().unwrap());
    assert_eq!(accepted.fuse_config, Some(config));
    assert!(conn.close_reason().is_none());
    assert!(acceptor.handshaking.is_none());
    conn.close(0u32.into(), b"done");
}

#[tokio::test]
async fn handshake_error_clears_pending_state() {
    let (mut acceptor, client) = endpoints();
    let invalid = client.connect(acceptor.socket, "not-localhost.invalid").unwrap();
    let (accepted, conn) = timeout(STEP, async {
        tokio::join!(acceptor.accept(None), invalid)
    }).await.unwrap();
    assert!(accepted.is_err());
    assert!(conn.is_err());
    assert!(acceptor.handshaking.is_none());
    let valid = client.connect(acceptor.socket, "localhost").unwrap();
    let (accepted, conn) = timeout(STEP, async {
        tokio::join!(acceptor.accept(None), valid)
    }).await.unwrap();
    assert!(accepted.is_ok());
    conn.unwrap().close(0u32.into(), b"done");
}

// A test-only polling gate keeps the first accept future suspended after its
// initial Pending poll, while the actual Quinn driver completes the handshake.
// This removes select!'s random ready-branch ordering from the TCP-winner test.
struct GatedQuinn {
    inner: QuinnAcceptor,
    resume: Arc<std::sync::atomic::AtomicBool>,
}
impl Acceptor for GatedQuinn {
    type Coupler = QuinnCoupler;
    type Stream = QuinnConnection;
    fn holdings(&self) -> &[Holding] { self.inner.holdings() }
    async fn accept(&mut self, policy: Option<ArcFusePolicy>) -> IoResult<Accepted<Self::Coupler, Self::Stream>> {
        let resume = self.resume.clone();
        let mut first = true;
        let mut accept = Box::pin(self.inner.accept(policy));
        poll_fn(|cx| {
            if first || resume.load(Ordering::SeqCst) {
                first = false;
                accept.as_mut().poll(cx)
            } else {
                Poll::Pending
            }
        }).await
    }
}

#[tokio::test]
async fn joined_tcp_winner_preserves_same_quic_connection() {
    use crate::conn::{JoinedAcceptor, TcpListener};
    use crate::conn::joined::JoinedStream;
    use std::sync::atomic::AtomicBool;
    let (mut quic, client) = endpoints();
    let connecting = incoming(&mut quic, &client).await;
    let tcp = TcpListener::new("127.0.0.1:0").try_bind().await.unwrap();
    let tcp_addr = tcp.local_addr().unwrap();
    let resume = Arc::new(AtomicBool::new(false));
    let mut joined = JoinedAcceptor::new(tcp, GatedQuinn { inner: quic, resume: resume.clone() });
    let mut accept = Box::pin(joined.accept(None));
    poll_pending(accept.as_mut()).await;
    let conn = handshake_done(connecting).await;
    let _tcp_peer = timeout(STEP, tokio::net::TcpStream::connect(tcp_addr)).await.unwrap().unwrap();
    let accepted = timeout(STEP, accept).await.unwrap().unwrap();
    assert!(matches!(accepted.stream, JoinedStream::A(_)), "TCP must win the joined select");
    resume.store(true, Ordering::SeqCst);
    let accepted = timeout(STEP, joined.accept(None)).await.unwrap().unwrap();
    assert!(matches!(accepted.stream, JoinedStream::B(_)), "next accept must resume QUIC");
    assert_eq!(accepted.remote_addr.into_std().unwrap(), client.local_addr().unwrap());
    assert!(conn.close_reason().is_none());
    conn.close(0u32.into(), b"done");
}
