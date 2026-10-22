# Noq backend evaluation

This candidate makes Salvo's HTTP/3 and WebTransport server use the same Noq
fork as Aster's Iroh stack. It is based on Salvo `80c472ac9e19bd6bba01e734fa2bf8503d569dd1`
(the 1.0.5 assembly on upstream v1.0.1). It was evaluated against Noq 1.4.0 at
`535bd88409193ae577f445a3207351bb83525f4e`; on adoption it pins Noq 1.4.1 at
`13dcb5b7f60240d729580014356fca6ab7f18ce3` (`upgrade/noq-v1.4.0`, which adds the
BBRv3 fixes, receive-buffer compaction, the zero pacing-delay fix and the BBRv3
default) from `https://forge.emrul.dev/Aster/noq.git`.

Branch: `feat/noq-backend-v1.0.1`, cherry-picked onto `upgrade/salvo-v1.0.1`.
Consumer adoption and registry publication are separate steps. On the upgrade
branch the changed family takes fresh versions: the Salvo crates 1.0.6,
`h3-quinn` 0.0.13 and `salvo-http3` 0.7.3 (`h3` 0.0.9 and `h3-datagram` 0.0.3
are unchanged). The evaluation branch itself kept the already allocated
versions and must not be published.

## Why

The benefit is one QUIC implementation across Iroh and HTTP/3: Aster's Noq
patches and fixes also reach the browser-facing transport, there is one
transport implementation to reason about, and upstream Quinn is removed from
normal Salvo server builds. The cost is a permanent Salvo-fork patch to review
on every Salvo or Noq upgrade. This candidate supersedes the 2026-05-22
decision to retain two QUIC stacks: aliasing Noq at the dependency boundary
now addresses the hard-coded Quinn types that blocked that approach.

## Design and maintenance

The workspace and vendored `h3-quinn` dependency both alias package `noq` as
`quinn`, at the identical Git URL and revision. The `quinn` feature,
`QuinnListener`, `QuinnAcceptor`, `build_quinn_config`, `quinn_connection`,
`salvo::proto::quinn` and `salvo_http3::quinn` names are retained. Their exposed
QUIC types are now Noq types. There is one server backend, with no extra
feature combinations for selecting between Quinn and Noq.

Use `salvo::proto::noq` as the explicit transport name; `salvo::proto::quinn`
is a compatibility re-export of the same crate. These types are incompatible
with an independent upstream `quinn` dependency. Compiler diagnostics may
render both as `quinn::Connection` because of the dependency alias; compare
package/source identities when investigating such errors.

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
   Obtain RTT, congestion window and MTU from the active path. A browser
   session has exactly one active path with multipath disabled; use
   `PathId::ZERO` for this non-multipath connection, including after ordinary
   connection migration. Keep byte/packet/loss counters connection-wide and
   preserve both aggregate and active-path snapshots after close. Resolve
   peer addresses through Noq's path APIs;
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

Acceptance also needs the transport-parameter and performance checks below,
and supported platform CI. A backend swap does not automatically share the
Iroh endpoint, sockets, NAT traversal, or ALPN dispatch. Salvo's configuration
code continues to start with the backend's `TransportConfig::default()` plus
5-second keep-alive and a 30-second idle timeout. This does **not** establish
that Noq's defaults match upstream Quinn's defaults.

## Browser-facing defaults and acceptance

These are the defaults of the exact pinned Noq source, before a consumer's
`transport_config_tuner` changes them. Review them again on every pin update.

| Area | Pinned default / Salvo behaviour | Acceptance evidence |
| --- | --- | --- |
| Multipath | Disabled (`max_concurrent_multipath_paths = None`). | No multipath negotiation in Chrome/Safari; exactly one active path. |
| n0 address discovery | Sending and receiving observed-address reports both disabled; no address-discovery transport parameter. | Capture both peers' transport parameters and verify absence. |
| n0 NAT traversal and server handshake migration | NAT traversal disabled (`max_remote_nat_traversal_addresses = None`); server handshake migration disabled. | No NAT-traversal extension negotiation or frames on the browser listener. |
| MTU discovery | Enabled when the socket permits it; 1200-byte starting/minimum MTU; default search ceiling 1452 bytes, also constrained by the peer. Reprobe interval 600 seconds; black-hole cooldown 60 seconds. Endpoint receive UDP-payload ceiling is 1472 bytes. | Record each peer's advertised maximum UDP payload, actual path MTU, probes and recovery on a constrained-MTU path. |
| GSO/GRO | Send segmentation offload enabled; actual segment and receive coalescing support comes from the OS/socket. These are local capabilities, not QUIC transport parameters. | Record actual GSO/GRO capabilities and batching on Linux; compare an offload-disabled run. |
| ECN | New paths initially attempt ECN; validation can disable marking if feedback is absent/invalid. ECN is validated through packet markings and ACK feedback, not a negotiated transport parameter. | Record marking/ACK_ECN validation and fallback on an ECN-disabled path. |
| Congestion control and pacing | BBRv3 (Noq `bbr3-default`, from Noq 1.4.1), which supplies its own pacing rate; paced sending enabled. CUBIC and NewReno remain selectable through `transport_config_tuner` / `congestion_controller_factory`. The evaluation runs above used Noq 1.4.0, whose default was CUBIC. | Record the selected controller and pacing behaviour; compare throughput and tail latency. |
| ACK frequency | No custom ACK-frequency configuration; the protocol still advertises `min_ack_delay` at its timer granularity. | Record the peers' advertised support and any actual extension use; do not equate `ack_frequency_config = None` with absent negotiation. |
| Idle/keep-alive | Salvo sets 30-second idle timeout and 5-second keep-alive. | Record the negotiated idle timeout and verify idle-session liveness. |

Source references in the Noq pin: `noq-proto/src/config/transport.rs`
(`TransportConfig::default`, `MtuDiscoveryConfig::default`),
`noq-proto/src/config/mod.rs` (`EndpointConfig::new`),
`noq-proto/src/address_discovery.rs`, `noq-proto/src/connection/paths.rs`,
`noq-proto/src/connection/pacing.rs`, and `noq/src/runtime/tokio.rs`.

For Chrome and Safari, save a qlog or decoded handshake capture showing both
peers' transport parameters, ALPN `h3`, HTTP/3 CONNECT/WebTransport and
datagram SETTINGS, extension negotiation and the active path. A successful
CONNECT alone is insufficient. Check actual negotiated values against the
table and explain any browser/version-specific differences. GSO/GRO, pacing
and ECN require the local/packet-level evidence described above.

## Performance gate before adoption

Compare the original Quinn-backed Salvo assembly `80c472ac` against this Noq
candidate using the same H3/WebTransport harness, application, TLS provider,
configuration and host. Prefer Linux for meaningful GSO/GRO coverage. Reuse
B1's harness when available; it remains a required adoption gate, not a
completed result of this candidate's functional tests.

Run repeated, interleaved trials with warm-up: clean LAN/WAN, constrained
bandwidth, random loss, shallow/deep queues, and small requests alongside a
bulk transfer. Record delivered application throughput, small-request
p50/p95/p99 latency and failures, CPU, loss/retransmissions, and offload
capabilities. First compare both backends using CUBIC, selected explicitly on
the Noq side because Noq's default is now BBRv3; keep any Noq CUBIC-versus-BBRv3
experiment separate so a controller change is not attributed to the backend
migration. Investigate consistent throughput, tail-latency or
CPU regressions before adoption. A simulated Noq-only congestion-control
benchmark cannot satisfy this Quinn-versus-Noq H3/WebTransport gate.

## Proposed ledger entry on acceptance

The canonical ledger is on Salvo `main`; this candidate does not change its
release record. Add a row there when accepting the patch:

| Topic / base | Reason and consumer impact | Upgrade/release notes |
| --- | --- | --- |
| `feat/noq-backend-v1.0.1`, based on the **1.0.5 assembly `80c472ac`**, not a bare upstream tag; stacked because it needs the vendored H3 and WebTransport patches, like the assembled-base exception in iroh-docs row 41. | One QUIC stack and patch set for Iroh and HTTP/3; supersedes the 2026-05-22 two-stack decision. Permanent backend-substitution patch. Existing `quinn` API names expose **Noq types**, incompatible with upstream Quinn; prefer `salvo::proto::noq`. Aster must port socket lifecycle and active-path statistics. | Review defaults and adapter APIs on every Salvo/Noq upgrade; require browser negotiation captures and the H3/WebTransport performance gate. Evaluation keeps existing versions only temporarily: **do not publish under already allocated versions**. Allocate fresh versions and propagate dependency requirements before adoption/publication. |

After acceptance, allocate fresh registry versions for changed packages and
propagate their requirements through the dependent Salvo family. Update the
native-stack publisher to rewrite Noq dependencies to the Aster registry,
validate the packaged dependency graph, and update Aster's pins and fork
manifest. Only then assemble/tag/publish under the existing fork workflow.
