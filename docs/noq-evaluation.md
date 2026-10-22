# Noq backend evaluation

This candidate makes Salvo's HTTP/3 and WebTransport server use the same Noq
fork as Aster's Iroh stack. It is based on Salvo `80c472ac9e19bd6bba01e734fa2bf8503d569dd1`
(the 1.0.5 assembly on upstream v1.0.1) and pins Noq 1.4.0 at
`535bd88409193ae577f445a3207351bb83525f4e` from
`https://forge.emrul.dev/Aster/noq.git`.

Branch: `feat/noq-backend-v1.0.1`. This is a source evaluation candidate;
consumer adoption and registry publication are separate steps. Existing crate
versions are deliberately unchanged during evaluation. Do not publish this
tree using those already allocated versions.

## Design and maintenance

The workspace and vendored `h3-quinn` dependency both alias package `noq` as
`quinn`, at the identical Git URL and revision. The `quinn` feature,
`QuinnListener`, `QuinnAcceptor`, `build_quinn_config`, `quinn_connection`,
`salvo::proto::quinn` and `salvo_http3::quinn` names are retained. Their exposed
QUIC types are now Noq types. There is one server backend, with no extra
feature combinations for selecting between Quinn and Noq.

The adapter drops the obsolete `ReadError::IllegalOrderedRead` arm. The
existing hand-off and transport-ACK tests capture the peer address from
`Incoming`, before accepting it. The in-memory handshake tests use Noq's boxed
socket and `UdpSender` API, retaining the original packet-delay/drop schedules.
Noq's non-exhaustive receive metadata is initialized through `Default` and
field assignments.

An independent `wtransport` 0.7 client, using upstream Quinn, is a development
dependency only. Its regression test verifies CONNECT, a bidirectional stream,
datagrams in both directions, the raw connection extension, and termination
of the serving task after client close. It complements the existing Noq-to-Noq
hand-off, FIN acknowledgement, receive cancellation, and stalled-handshake
tests. Keeping Quinn in this test graph is intentional.

On each Salvo/Noq upgrade, review the adapter's error mapping, native stream
polling, FIN acknowledgement semantics, handshake handling and socket test
helpers. Update both Noq dependency declarations together, matching the Aster
consumer's source identity. The release patch ledger remains on Salvo `main`;
add this patch there if the candidate is accepted.

## Local validation

Run from this worktree (prefix with `rtk` where required by local instructions):

```sh
cargo +nightly fmt --all -- --check
cargo test --offline -p salvo_core --features quinn,rustls --lib
cargo check --offline -p salvo --features quinn,rustls,acme,serve-static --all-targets
cargo clippy --offline --workspace --all-targets \
  --features salvo/quinn,salvo/rustls,salvo/acme,salvo/serve-static \
  -- -A clippy::collapsible_if -A clippy::redundant_clone
cargo tree --offline -p salvo --features quinn,rustls,acme,serve-static \
  -e normal --prefix none -i noq
cargo tree --offline -p salvo --features quinn,rustls,acme,serve-static \
  -e normal --prefix none -i quinn
```

The two Clippy allowances are existing Salvo baseline diagnostics. Offline
commands require the pinned sources and dependencies to be cached; omit
`--offline` for an initial fetch.

On Apple Silicon macOS, the core library suite passes 358 tests, including
`quinn_webtransport_client_interoperates_with_noq_server`. The same 358 tests
pass with the default features replaced by
`cookie,fix-http1-request-uri,server,server-handle,http1,http2,test,ring,matched-path,unix,quinn`.
Core doctests pass (59 passed, 30 ignored). Nightly formatting, the Salvo
feature check, a minimal `server,quinn,ring` build with defaults disabled, and
workspace/all-target Clippy with the stated allowances pass. Existing
workspace/vendor warnings remain. The normal dependency graph for the Aster
feature set contains the pinned Noq and has no upstream Quinn dependency.
This is not an Aster consumer, browser, Windows/Linux, or performance validation
result.

## Aster evaluation hand-off

Evaluate in a separate Aster branch/worktree before changing production pins.

1. Point the complete Salvo/H3 source family at this worktree or the candidate
   commit. Aster currently patches `salvo`, `salvo_core`, `salvo_macros`,
   `salvo-http3`, `h3`, `h3-quinn` and `h3-datagram`; retain any additional
   family overrides needed by the selected consumer features. Root Cargo
   patches do not propagate across workspaces.
2. Replace `aster-transport-salvo`'s direct upstream Quinn dependency with
   Noq. Aliasing it as `quinn` minimizes initial edits, but it must use the
   exact Git source/revision above, matching Iroh and the adapter. A crate
   rename preserves spelling, not upstream Quinn type compatibility: request
   extension `TypeId`s must resolve to the same Noq package.
3. Port `aster-transport-salvo/src/listener.rs` explicitly. Noq's
   `Runtime::wrap_udp_socket` and `Endpoint::new_with_abstract_socket` use
   `Box<dyn AsyncUdpSocket>`. `create_sender` replaces `create_io_poller` and
   socket-level `try_send`; `poll_recv` takes `&mut self`; segment limits use
   `NonZeroUsize`. Preserve the tracked socket/sender ownership invariant so
   shutdown waits for the actual final socket reference to be released.
4. Port raw WebTransport diagnostics. Noq's `ConnectionStats` is cloneable,
   not `Copy`, and has aggregate counters rather than Quinn's `stats.path`.
   Obtain RTT, congestion window and MTU from per-path statistics, define
   which path the existing single-path API reports, and preserve a final
   snapshot after close. Resolve peer addresses through Noq's path APIs;
   established `Connection` has no Quinn-style `remote_address()` accessor.
5. Run the Aster checks below, including socket rebind after stop, pending
   accept/read cancellation, peer stop/reset, FIN acknowledgements, datagrams,
   stats snapshots and owned-session shutdown. Evaluate Portal consumers
   using raw transport types or congestion controller callbacks too.

```sh
cargo check --all-targets -p aster-transport-salvo -p aster-expose --features aster-expose/edge
cargo test -p aster-transport-salvo
cargo test -p aster-expose --features edge --lib --tests
```

Retain upstream Quinn for independent clients such as `wtransport` where
needed. Check actual normal/build dependency edges rather than treating
Quinn's presence in `Cargo.lock` as a failed migration.

Acceptance also needs Chrome/Safari WebTransport checks and supported platform
CI. A backend swap does not automatically share the Iroh endpoint, sockets,
NAT traversal, or ALPN dispatch, and this candidate changes no congestion
controller defaults.

After acceptance, allocate fresh registry versions for changed packages and
propagate their requirements through the dependent Salvo family. Update the
native-stack publisher to rewrite Noq dependencies to the Aster registry,
validate the packaged dependency graph, and update Aster's pins and fork
manifest. Only then assemble/tag/publish under the existing fork workflow.
