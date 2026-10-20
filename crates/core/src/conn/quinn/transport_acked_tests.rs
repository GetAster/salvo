//! End-to-end tests for `SendStream::transport_acked()` on a WebTransport send
//! stream, over a real local QUIC connection.
//!
//! The server side drives [`Builder::serve_connection`] directly; the handler
//! accepts a WebTransport bidirectional stream, writes on its send half and
//! reports what `transport_acked()` resolved to. The client side is the vendored
//! `h3` client over `h3-quinn` for the extended `CONNECT`, and the raw Quinn
//! connection for the WebTransport stream itself (stream type `0x41` followed by
//! the session id, as in draft-ietf-webtrans-http3). A server-initiated uni
//! stream can't be used here: the h3 client's connection driver consumes every
//! incoming uni stream. Both directions share the `h3_quinn::SendStream` impl.
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use futures_util::future::poll_fn;
use http::uri::Scheme;
use salvo_http3::ext::Protocol;
use salvo_http3::quic::BidiStream as _;
use salvo_http3::webtransport::server::AcceptedBi;
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_rustls::rustls;

use super::{Builder, QuinnConnection};
use crate::conn::ConnCtrl;
use crate::conn::rustls::{Keycert, RustlsConfig, default_crypto_provider};
use crate::http::Method;
use crate::{Request, Router, Service, handler};

type ClientSendRequest = salvo_http3::client::SendRequest<salvo_http3::quinn::OpenStreams, Bytes>;
type ConnectStream =
    salvo_http3::client::RequestStream<salvo_http3::quinn::BidiStream<Bytes>, Bytes>;

const STEP: Duration = Duration::from_secs(10);
const PAYLOAD: &[u8] = b"frame bytes";
// Full chain (leaf, intermediate, root) so the client can verify it against the root.
const CERT: &[u8] = include_bytes!("../../../certs/chain.pem");
const KEY: &[u8] = include_bytes!("../../../certs/key.pem");

/// Accepts a WebTransport bidi stream, writes [`PAYLOAD`] on its send half,
/// optionally finishes it, and reports what `transport_acked()` resolved to
/// (`None` on timeout).
struct SendAndReport {
    finish: bool,
    report: Mutex<Option<oneshot::Sender<Option<bool>>>>,
}
#[handler]
impl SendAndReport {
    async fn handle(&self, req: &mut Request) -> Result<(), crate::Error> {
        let session = req.web_transport_mut().await?;
        let Ok(Some(AcceptedBi::BidiStream(_, stream))) = session.accept_bi().await else {
            return Err(crate::Error::Other("no WebTransport bidi stream".into()));
        };
        let (mut send, _recv) = stream.split();
        send.write_all(PAYLOAD).await?;
        if self.finish {
            send.shutdown().await?;
        }
        // The future owns quinn's notifier, so it may outlive the stream wrapper.
        let acked = send.transport_acked();
        let outcome = timeout(STEP, acked).await.ok();
        if let Some(tx) = self.report.lock().unwrap().take() {
            let _ = tx.send(outcome);
        }
        Ok(())
    }
}

async fn serve_one(router: Router) -> (SocketAddr, JoinHandle<std::io::Result<()>>) {
    let config = RustlsConfig::new(Keycert::new().cert(CERT).key(KEY))
        .build_quinn_config()
        .expect("quinn server config");
    let endpoint = ::quinn::Endpoint::server(config, (Ipv4Addr::LOCALHOST, 0).into())
        .expect("bind quinn endpoint");
    let addr = endpoint.local_addr().expect("local addr");
    let service = Service::new(router);
    let handle = tokio::spawn(async move {
        let incoming = endpoint.accept().await.expect("incoming connection");
        // Upstream 1.0.1 (#1711) hands the connection over before the handshake completes;
        // `serve_connection` awaits it, as the accept loop does.
        let (raw, handshake_done) = incoming
            .accept()
            .expect("accept incoming")
            .into_0rtt()
            .expect("server connections always accept 0.5-RTT");
        let remote = raw.remote_address();
        let hyper_handler = service.hyper_handler(
            crate::conn::SocketAddr::from(addr),
            crate::conn::SocketAddr::from(remote),
            Scheme::HTTPS,
            None,
            ConnCtrl::new(),
            None,
        );
        let result = Builder::new()
            .serve_connection(QuinnConnection::new(raw, handshake_done, None), hyper_handler, None)
            .await;
        drop(endpoint);
        result
    });
    (addr, handle)
}

struct Client {
    _endpoint: ::quinn::Endpoint,
    conn: ::quinn::Connection,
    send_request: ClientSendRequest,
    driver: JoinHandle<()>,
    port: u16,
}

async fn connect(addr: SocketAddr) -> Client {
    let mut roots = rustls::RootCertStore::empty();
    {
        use rustls::pki_types::pem::PemObject;
        for cert in rustls::pki_types::CertificateDer::pem_slice_iter(CERT) {
            roots
                .add(cert.expect("parse test cert"))
                .expect("add test root");
        }
    }
    let mut tls = rustls::ClientConfig::builder_with_provider(default_crypto_provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("tls13")
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let crypto =
        ::quinn::crypto::rustls::QuicClientConfig::try_from(tls).expect("quic client crypto");
    let mut endpoint =
        ::quinn::Endpoint::client((Ipv4Addr::LOCALHOST, 0).into()).expect("client endpoint");
    endpoint.set_default_client_config(::quinn::ClientConfig::new(Arc::new(crypto)));
    let conn = endpoint
        .connect(addr, "localhost")
        .expect("connect")
        .await
        .expect("client handshake");
    let (mut driver, send_request) = salvo_http3::client::builder()
        .enable_extended_connect(true)
        .enable_datagram(true)
        .build::<_, _, Bytes>(salvo_http3::quinn::Connection::new(conn.clone()))
        .await
        .expect("h3 client");
    let driver = tokio::spawn(async move {
        let _ = poll_fn(|cx| driver.poll_close(cx)).await;
    });
    Client {
        _endpoint: endpoint,
        conn,
        send_request,
        driver,
        port: addr.port(),
    }
}

impl Client {
    /// Send the WebTransport `CONNECT` and wait for the 200. Dropping the
    /// returned CONNECT stream would end the session.
    async fn open_session(&mut self, path: &str) -> (ConnectStream, u64) {
        let req = http::Request::builder()
            .method(Method::CONNECT)
            .uri(format!("https://localhost:{}{path}", self.port))
            .extension(Protocol::WEB_TRANSPORT)
            .body(())
            .unwrap();
        let mut stream = self.send_request.send_request(req).await.expect("CONNECT");
        let response = timeout(STEP, stream.recv_response())
            .await
            .expect("CONNECT response timed out")
            .expect("CONNECT response");
        assert_eq!(response.status(), http::StatusCode::OK);
        let session_id = stream.id().into_inner();
        (stream, session_id)
    }

    /// Open a WebTransport bidi stream on `session_id` (stream type `0x41`,
    /// then the session id). The send half stays open, so only the server's
    /// half can finish.
    async fn open_wt_bidi(&self, session_id: u64) -> (::quinn::SendStream, ::quinn::RecvStream) {
        let (mut send, recv) = self.conn.open_bi().await.expect("open bidi");
        let mut header = BytesMut::new();
        put_varint(&mut header, 0x41);
        put_varint(&mut header, session_id);
        send.write_all(&header)
            .await
            .expect("write WT stream header");
        (send, recv)
    }

    fn close(self) {
        self.conn.close(0x100u32.into(), b"done");
        self.driver.abort();
    }
}

fn put_varint(buf: &mut BytesMut, value: u64) {
    use bytes::BufMut;
    if value < 1 << 6 {
        buf.put_u8(value as u8);
    } else if value < 1 << 14 {
        buf.put_u16(0x4000 | value as u16);
    } else if value < 1 << 30 {
        buf.put_u32(0x8000_0000 | value as u32);
    } else {
        buf.put_u64(0xc000_0000_0000_0000 | value);
    }
}

fn router(finish: bool) -> (Router, oneshot::Receiver<Option<bool>>) {
    let (tx, rx) = oneshot::channel();
    let router = Router::with_path("wt").goal(SendAndReport {
        finish,
        report: Mutex::new(Some(tx)),
    });
    (router, rx)
}

/// FIN plus every byte acknowledged by the peer resolves `true`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_acked_resolves_true_once_fin_and_data_are_acked() {
    let (router, report) = router(true);
    let (addr, _server) = serve_one(router).await;
    let mut client = connect(addr).await;
    let (_connect_stream, session_id) = client.open_session("/wt").await;

    let (_send, mut recv) = client.open_wt_bidi(session_id).await;
    let body = timeout(STEP, recv.read_to_end(64 * 1024))
        .await
        .expect("read timed out")
        .expect("read payload");
    assert_eq!(body, PAYLOAD);

    let outcome = timeout(STEP, report)
        .await
        .expect("no report")
        .expect("handler dropped");
    assert_eq!(outcome, Some(true));
    client.close();
}

/// A peer STOP_SENDING is not delivery: `transport_acked()` resolves `false`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transport_acked_resolves_false_on_peer_stop() {
    // No FIN: the stream can only complete through the peer's STOP_SENDING.
    let (router, report) = router(false);
    let (addr, _server) = serve_one(router).await;
    let mut client = connect(addr).await;
    let (_connect_stream, session_id) = client.open_session("/wt").await;

    let (_send, mut recv) = client.open_wt_bidi(session_id).await;
    // Wait for the server's bytes so the stream exists on its side, then stop it.
    let mut buf = [0u8; PAYLOAD.len()];
    timeout(STEP, recv.read_exact(&mut buf))
        .await
        .expect("read timed out")
        .expect("read payload");
    recv.stop(7u32.into()).expect("stop");

    let outcome = timeout(STEP, report)
        .await
        .expect("no report")
        .expect("handler dropped");
    assert_eq!(outcome, Some(false));
    client.close();
}

/// A cancelled pending read must leave both identity and STOP control usable.
struct PendingReadThenStop {
    report: Mutex<Option<oneshot::Sender<u64>>>,
}
#[handler]
impl PendingReadThenStop {
    async fn handle(&self, req: &mut Request) -> Result<(), crate::Error> {
        use salvo_http3::quic::RecvStream as _;
        use tokio::io::AsyncReadExt;
        let session = req.web_transport_mut().await?;
        let Ok(Some(AcceptedBi::BidiStream(_, stream))) = session.accept_bi().await else {
            return Err(crate::Error::Other("no WebTransport bidi stream".into()));
        };
        let (_send, mut recv) = stream.split();
        let id = recv.recv_id().into_inner();
        let mut byte = [0u8; 1];
        assert!(
            timeout(Duration::from_millis(20), recv.read(&mut byte))
                .await
                .is_err()
        );
        assert_eq!(recv.recv_id().into_inner(), id);
        recv.stop_sending(37);
        if let Some(tx) = self.report.lock().unwrap().take() {
            let _ = tx.send(id);
        }
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_read_preserves_recv_id_and_stop_sending_application_code() {
    let (tx, rx) = oneshot::channel();
    let router = Router::with_path("wt").goal(PendingReadThenStop {
        report: Mutex::new(Some(tx)),
    });
    let (addr, _server) = serve_one(router).await;
    let mut client = connect(addr).await;
    let (_connect_stream, session_id) = client.open_session("/wt").await;
    let (send, _recv) = client.open_wt_bidi(session_id).await;
    let stopped = timeout(STEP, send.stopped())
        .await
        .expect("STOP timed out")
        .expect("connection failed");
    assert_eq!(stopped, Some(37u32.into()));
    assert_eq!(
        timeout(STEP, rx).await.unwrap().unwrap(),
        u64::from(send.id())
    );
    client.close();
}
