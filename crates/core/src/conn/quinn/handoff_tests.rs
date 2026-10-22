//! End-to-end tests for WebTransport session hand-off over a real local QUIC
//! connection.
//!
//! The server side drives [`Builder::serve_connection`] directly so the tests can
//! observe the `IoResult` it returns (the normal `Server` loop discards it). The
//! client side is the vendored `h3` client over `h3-quinn`: it sends the extended
//! `CONNECT` with `:protocol = webtransport` and then opens WebTransport
//! bidirectional streams by hand on the raw Quinn connection (stream type
//! `0x41` followed by the session id, as in draft-ietf-webtrans-http3).
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use bytes::{Buf, Bytes, BytesMut};
use futures_util::future::poll_fn;
use http::uri::Scheme;
use salvo_http3::ext::Protocol;
use salvo_http3::webtransport::server::AcceptedBi;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_rustls::rustls;

use super::{Builder, QuinnConnection};
use crate::conn::ConnCtrl;
use crate::conn::rustls::{Keycert, RustlsConfig, default_crypto_provider};
use crate::http::Method;
use crate::proto::WebTransportSession;
use crate::{Request, Response, Router, Service, handler};

type Session = WebTransportSession<salvo_http3::quinn::Connection, Bytes>;
type ClientSendRequest = salvo_http3::client::SendRequest<salvo_http3::quinn::OpenStreams, Bytes>;

const STEP: Duration = Duration::from_secs(10);
// Full chain (leaf, intermediate, root) so the client can verify it against the root.
const CERT: &[u8] = include_bytes!("../../../certs/chain.pem");
const KEY: &[u8] = include_bytes!("../../../certs/key.pem");

/// Echo exactly one incoming WebTransport bidirectional stream on `session`.
async fn echo_one_stream(session: &Session) -> Result<(), String> {
    match session.accept_bi().await.map_err(|e| e.to_string())? {
        Some(AcceptedBi::BidiStream(_, mut stream)) => {
            let mut buf = Vec::new();
            stream
                .read_to_end(&mut buf)
                .await
                .map_err(|e| e.to_string())?;
            stream.write_all(&buf).await.map_err(|e| e.to_string())?;
            stream.shutdown().await.map_err(|e| e.to_string())?;
            Ok(())
        }
        Some(AcceptedBi::Request(..)) => Err("unexpected HTTP/3 request".into()),
        None => Err("connection closed before a stream arrived".into()),
    }
}

/// (i) Takes ownership, moves the session into a task and returns at once.
/// The task echoes streams until the connection goes away and reports how
/// many it served.
struct OwnedSpawn {
    served: mpsc::UnboundedSender<usize>,
}
#[handler]
impl OwnedSpawn {
    async fn handle(&self, req: &mut Request) -> Result<(), crate::Error> {
        let session = req.web_transport_owned().await?;
        let served = self.served.clone();
        tokio::spawn(async move {
            let mut count = 0;
            while echo_one_stream(&session).await.is_ok() {
                count += 1;
                let _ = served.send(count);
            }
        });
        Ok(())
    }
}

/// (ii) Takes ownership but keeps the only clone inside the handler, echoes
/// one stream and drops it before returning.
#[handler]
async fn owned_held(req: &mut Request) -> Result<(), crate::Error> {
    let session = req.web_transport_owned().await?;
    echo_one_stream(&session)
        .await
        .map_err(|e| crate::Error::Other(e.into()))?;
    drop(session);
    Ok(())
}

/// (iii) The non-owned path used by `aster-transport-salvo` / portal-agent.
#[handler]
async fn borrowed(req: &mut Request) -> Result<(), crate::Error> {
    let session = req.web_transport_mut().await?;
    echo_one_stream(session)
        .await
        .map_err(|e| crate::Error::Other(e.into()))?;
    Ok(())
}

/// Exercise an independent Quinn-based WebTransport client against Noq. Keep
/// the session alive until the client closes so CONNECT reclamation cannot
/// race the datagram response.
#[handler]
async fn interop_echo(req: &mut Request) -> Result<(), crate::Error> {
    let raw: crate::proto::noq::Connection = req.quinn_connection().expect("Noq connection extension");
    let session = req.web_transport_mut().await?;
    echo_one_stream(session)
        .await
        .map_err(|e| crate::Error::Other(e.into()))?;
    let datagram = session.datagram_reader().read_datagram().await?;
    session
        .datagram_sender()
        .send_datagram(datagram.into_payload())
        .map_err(crate::Error::other)?;
    raw.closed().await;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn quinn_webtransport_client_interoperates_with_noq_server() {
    timeout(STEP, async {
        // Core's tests deliberately link both crypto providers.
        let _ = (*default_crypto_provider()).clone().install_default();
        let router = Router::with_path("wt").goal(interop_echo);
        let (addr, server) = serve_one(router).await;
        let endpoint = wtransport::Endpoint::client(
            wtransport::ClientConfig::builder()
                .with_bind_default()
                .with_no_cert_validation()
                .build(),
        )
        .expect("Quinn client endpoint");
        let connection = endpoint
            .connect(format!("https://{addr}/wt"))
            .await
            .expect("WebTransport CONNECT to Noq");
        let (mut send, mut recv) = connection.open_bi().await.unwrap().await.unwrap();
        send.write_all(b"Quinn to Noq").await.unwrap();
        send.finish().await.unwrap();
        let mut echoed = Vec::new();
        recv.read_to_end(&mut echoed).await.unwrap();
        assert_eq!(echoed, b"Quinn to Noq");

        connection.send_datagram(b"datagram interop").unwrap();
        let datagram = connection.receive_datagram().await.unwrap();
        assert_eq!(&*datagram, b"datagram interop");
        connection.close(0u32.into(), b"done");
        // HTTP/3 shutdown after peer close may return a connection error; the
        // important lifecycle assertion is that the serving task terminates.
        let _ = server.await.expect("serving task should not panic");
        endpoint.wait_idle().await;
    })
    .await
    .expect("Quinn/Noq interoperability timed out");
}

/// (iv) Shares the session *without* `web_transport_owned()` (clones the Arc
/// out of the extensions behind salvo's back) and keeps it past the handler.
/// Upstream's "still shared" error must still be reported for this.
struct SharedWithoutOwned {
    keep: std::sync::Mutex<Option<oneshot::Sender<Arc<Session>>>>,
}
#[handler]
impl SharedWithoutOwned {
    async fn handle(&self, req: &mut Request) -> Result<(), crate::Error> {
        req.web_transport_mut().await?;
        let session = req
            .extensions()
            .get::<Arc<Session>>()
            .cloned()
            .expect("session cached in extensions");
        if let Some(tx) = self.keep.lock().unwrap().take() {
            let _ = tx.send(session);
        }
        Ok(())
    }
}

#[handler]
async fn hello(res: &mut Response) {
    res.render("hello");
}

/// Accept exactly one QUIC connection and serve it with `router`, returning
/// the server address and the `serve_connection` result.
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
        let remote = incoming.remote_address();
        // Upstream 1.0.1 (#1711) hands the connection over before the handshake completes;
        // `serve_connection` awaits it, as the accept loop does.
        let (raw, handshake_done) = incoming
            .accept()
            .expect("accept incoming")
            .into_0rtt()
            .expect("server connections always accept 0.5-RTT");
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
        // Keep the endpoint alive until the connection is fully done with it.
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
    for cert in rustls_pemfile_certs(CERT) {
        roots.add(cert).expect("add test root");
    }
    let mut tls = rustls::ClientConfig::builder_with_provider(default_crypto_provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("tls13")
        .with_root_certificates(roots)
        .with_no_client_auth();
    tls.alpn_protocols = vec![b"h3".to_vec()];
    let crypto =
        ::quinn::crypto::rustls::QuicClientConfig::try_from(tls).expect("quic client crypto");
    let endpoint =
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

fn rustls_pemfile_certs(pem: &[u8]) -> Vec<rustls::pki_types::CertificateDer<'static>> {
    use rustls::pki_types::pem::PemObject;
    rustls::pki_types::CertificateDer::pem_slice_iter(pem)
        .collect::<Result<Vec<_>, _>>()
        .expect("parse test cert")
}

type ConnectStream = salvo_http3::client::RequestStream<salvo_http3::quinn::BidiStream<Bytes>, Bytes>;

impl Client {
    /// Send the WebTransport `CONNECT` and wait for the 200. Returns the
    /// CONNECT stream (dropping it would end the session) and the session id.
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

    /// Open a WebTransport bidi stream on `session_id`, send `payload`, and
    /// return what the server echoes back.
    async fn echo(&self, session_id: u64, payload: &[u8]) -> Vec<u8> {
        let (mut send, mut recv) = self.conn.open_bi().await.expect("open bidi");
        let mut header = BytesMut::new();
        put_varint(&mut header, 0x41);
        put_varint(&mut header, session_id);
        send.write_all(&header).await.expect("write WT stream header");
        send.write_all(payload).await.expect("write payload");
        send.finish().expect("finish");
        timeout(STEP, recv.read_to_end(64 * 1024))
            .await
            .expect("echo timed out")
            .expect("read echo")
    }

    /// Plain HTTP/3 GET on the same connection.
    async fn get(&mut self, path: &str) -> Result<(http::StatusCode, Vec<u8>), String> {
        let req = http::Request::builder()
            .method(Method::GET)
            .uri(format!("https://localhost:{}{path}", self.port))
            .body(())
            .unwrap();
        let mut stream = self
            .send_request
            .send_request(req)
            .await
            .map_err(|e| e.to_string())?;
        stream.finish().await.map_err(|e| e.to_string())?;
        let response = stream.recv_response().await.map_err(|e| e.to_string())?;
        let mut body = Vec::new();
        while let Some(mut chunk) = stream.recv_data().await.map_err(|e| e.to_string())? {
            body.extend_from_slice(&chunk.copy_to_bytes(chunk.remaining()));
        }
        Ok((response.status(), body))
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

/// (i) `web_transport_owned()` + spawn + return: `serve_connection` ends with
/// `Ok(())` (clean hand-off, no "still shared" error) and the spawned task keeps
/// serving the session after the handler has returned.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_session_spawned_task_is_a_clean_handoff() {
    let (served_tx, mut served_rx) = mpsc::unbounded_channel();
    let router = Router::with_path("wt").goal(OwnedSpawn { served: served_tx });
    let (addr, server) = serve_one(router).await;
    let mut client = connect(addr).await;
    let (_connect_stream, session_id) = client.open_session("/wt").await;

    // The handler has returned and the accept loop has finished: hand-off done.
    let result = timeout(STEP, server)
        .await
        .expect("serve_connection did not finish after the hand-off")
        .expect("serve task panicked");
    assert!(result.is_ok(), "hand-off reported an error: {result:?}");

    // The session still works from the spawned task, more than once.
    assert_eq!(client.echo(session_id, b"after handler returned").await, b"after handler returned");
    assert_eq!(client.echo(session_id, b"second stream").await, b"second stream");
    assert_eq!(timeout(STEP, served_rx.recv()).await.unwrap(), Some(1));
    assert_eq!(timeout(STEP, served_rx.recv()).await.unwrap(), Some(2));
    client.close();
}

/// (ii) Ownership taken, but the handler holds the only clone and drops it
/// before returning. The Arc is unique again, so upstream's path applies: the
/// session is split, the CONNECT stream is closed and the HTTP/3 connection is
/// handed back to the accept loop, which keeps serving plain requests on it.
/// No panic, no hang, and `serve_connection` ends cleanly when the client
/// closes.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn owned_session_dropped_in_handler_returns_connection_to_accept_loop() {
    let router = Router::new()
        .push(Router::with_path("wt").goal(owned_held))
        .push(Router::with_path("hello").get(hello));
    let (addr, server) = serve_one(router).await;
    let mut client = connect(addr).await;
    let (_connect_stream, session_id) = client.open_session("/wt").await;
    assert_eq!(client.echo(session_id, b"held").await, b"held");

    let (status, body) = timeout(STEP, client.get("/hello"))
        .await
        .expect("follow-up GET timed out")
        .expect("follow-up GET");
    assert_eq!(status, http::StatusCode::OK);
    assert_eq!(body, b"hello");
    assert!(!server.is_finished(), "accept loop should still be running");

    client.close();
    let result = timeout(STEP, server)
        .await
        .expect("serve_connection hung after client close")
        .expect("serve task panicked");
    assert!(result.is_ok(), "unexpected error: {result:?}");
}

/// (iii) `web_transport_mut()` (non-owned) still works: the stream is echoed
/// inside the handler and the connection is handed back to the accept loop.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn borrowed_session_still_works() {
    let router = Router::new()
        .push(Router::with_path("wt").goal(borrowed))
        .push(Router::with_path("hello").get(hello));
    let (addr, server) = serve_one(router).await;
    let mut client = connect(addr).await;
    let (_connect_stream, session_id) = client.open_session("/wt").await;
    assert_eq!(client.echo(session_id, b"borrowed").await, b"borrowed");

    let (status, body) = timeout(STEP, client.get("/hello"))
        .await
        .expect("follow-up GET timed out")
        .expect("follow-up GET");
    assert_eq!(status, http::StatusCode::OK);
    assert_eq!(body, b"hello");

    client.close();
    let result = timeout(STEP, server)
        .await
        .expect("serve_connection hung after client close")
        .expect("serve task panicked");
    assert!(result.is_ok(), "unexpected error: {result:?}");
}

/// (iv) Sharing the session without going through `web_transport_owned()`
/// keeps upstream's behaviour: `serve_connection` reports the session as
/// still shared.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shared_session_without_owned_marker_keeps_upstream_error() {
    let (keep_tx, keep_rx) = oneshot::channel();
    let router = Router::with_path("wt").goal(SharedWithoutOwned {
        keep: std::sync::Mutex::new(Some(keep_tx)),
    });
    let (addr, server) = serve_one(router).await;
    let mut client = connect(addr).await;
    let (_connect_stream, _session_id) = client.open_session("/wt").await;
    let _kept = keep_rx.await.expect("handler kept a clone");

    let result = timeout(STEP, server)
        .await
        .expect("serve_connection did not finish")
        .expect("serve task panicked");
    let error = result.expect_err("shared session without hand-off should error");
    assert_eq!(error.to_string(), "WebTransport session is still shared");
    client.close();
}
