# Aster patch ledger: salvo

This repository is Aster's soft fork of [salvo-rs/salvo](https://github.com/salvo-rs/salvo). Aster carries the small set of patches listed below, which expose lower-level QUIC and WebTransport internals that the Aster HTTP transport and the Portal desktop app need; everything else tracks upstream verbatim. Upstream maintainers are welcome to incorporate any of them, in whole or in part, at their discretion. The fork is meant to shrink: patches are expected to graduate upstream, not accumulate.

## Branch and tag model

- `main` is upstream `main` plus one commit: the Aster README preamble and this ledger. Nothing is built from `main`. It is refreshed with `git merge upstream/main` and is never reset or force-pushed. Ledger edits are commits on `main`, never on `upgrade/*`.
- `upgrade/<repo>-v<upstream version>` (for example `upgrade/iroh-v1.4.0`, `upgrade/iroh-blobs-v0.103.1`) is the patched integration line: the upstream release plus the patches in this ledger. A new upstream base always gets a new branch, cut from the upstream release tag; Aster-only fixes and re-pins append to the current branch. Per-minor names (`upgrade/iroh-blobs-v0.103`, `upgrade/salvo-v1.0`, …) are historical. Consumers pin commits on it. Published `upgrade/*` branches and tags are never rewritten; a failed attempt is superseded by new commits and a new version.
- Topic branches carry one change each, are named for the change (for example `feat/read-into`, `security/plumtree-bounds`), are based on the upstream release tag (or, for a patch that builds on another, on that prerequisite patch's commit), and are cherry-picked with `git cherry-pick -x` onto the upgrade branch, so every upgrade-branch commit names its source. A topic branch that must be re-cut for a new upstream base gets a new name, `<topic>-v<upstream version>` (for example `wt-transport-acked-v1.0.1`); the old branch stays as it was, nothing is force-pushed.
- Version (option E, effective from the first publication after Salvo 1.0.6): `major.minor.P` with `P = 1000 * (upstream_patch + 1) + n`, where `n` (1 to 999) counts Aster's patch releases on that upstream version: upstream v1.0.1 plus our first release is `1.0.2001`, our second `1.0.2002`. Upstream patch numbers stay below 1000, so our numbers never collide with a crates.io release; caret requirements still match. Each vendored crate follows the rule from the crates.io release it was cut from (`h3-quinn` 0.0.10 gives `0.0.11001`, `salvo-http3` 0.7.0 gives `0.7.1001`). `publish-native-stack.py check` in the Aster repository fails a new version outside that space or present on crates.io. Already published versions (1.0.6 and earlier) are grandfathered. A re-pin without a code change keeps the version and takes the tag suffix `-p<n>`.
- Tags: `aster-<repo>-v<published version>` (re-pins `aster-<repo>-v<published version>-p<n>`), created only after the Aster consumer trial passes against those exact commits, because tags cannot be moved.
- The fork manifest is `iroh-fork-manifest.toml` in the Aster repository (the only copy).
- Aster does not submit patches upstream; upstream maintainers are welcome to take any patch here.

Historical and current tag schemes in this repository:

- `aster-salvo-YYYY-MM-DD`, the date of an absorption or release, with `.n` for a second release the same day: `aster-salvo-2026-06-27`, `-2026-07-05`, `-2026-10-03`, `-2026-10-03.2`, `-2026-10-05`, `-2026-10-07`, `-2026-10-11`.
- `aster-salvo-v<published version>`, the current scheme for new tags (first: `aster-salvo-v1.0.5`; then `aster-salvo-v1.0.6` and the re-pin `aster-salvo-v1.0.6-p1`). The two schemes below are legacy.
- `v<upstream version>-aster.<n>`, since 2026-10-03, on the same commit as the dated tag: `v1.0.0-aster.1` … `v1.0.0-aster.5`. `<n>` was bumped for each re-assembly on the same upstream version and reset to 1 for a new upstream release.
- `archive/<branch>`, keeping a deleted or re-cut patch branch's original commit resolvable: `archive/quinn-webtransport-raw-connection`, and `archive/<branch>-pre-1.0` for the six branches re-cut on v1.0.0.

Topic branches in this repository predate the `feat/`/`fix/` naming and are named for the change directly (for example `quinn-default-keepalive`). Each holds exactly one commit; see "Patches". Some are stacked by design rather than based on the upstream tag, because they build on another patch: S4 on S1, S5 on S2. The `-v1.0.1` re-cuts (S3, S7, S9) sit on the v1.0.1 line: S3 on `v1.0.1`, S7 on the assembled S6 (`c524276a`), S9 on the assembled S7 (`ccc57bd7`), because their tests need the patches below them. Check each with `git cherry -v <branch base> <branch>` against its own base, which should list exactly one `+`.

## Current release

| Field | Value |
|---|---|
| Upstream base | `v1.0.1` |
| Integration branch | `upgrade/salvo-v1.0.1` |
| Tag | `aster-salvo-v1.0.2001` (to be created after the Aster release candidate passes; the latest existing tag is `aster-salvo-v1.0.6-p1`) |
| Pinned commit | `02002e3d6520125bb4ed124e62da531d74adabe9` |
| Crate versions | all 22 Salvo crates 1.0.2001; `h3-quinn` 0.0.11001; `salvo-http3` 0.7.1001; `h3` 0.0.9; `h3-datagram` 0.0.3. Noq backend (S11) on noq `=1.4.1001` (`379f7367`, `aster-noq-v1.4.1001` to be created) |
| Status | branch pushed; no `aster-salvo-v1.0.2001` tag yet. 1.0.7 (`46bb5d5f`, noq 1.4.3) was superseded by `02002e3d` before any publication or tag. The registry serves 1.0.6 (h3-quinn 0.0.13, salvo-http3 0.7.3), whose manifests require noq `=1.4.2` and cannot change, so the noq 1.4.1001 re-pin is the new Salvo family 1.0.2001 and publishes from this commit |
| Source repository | `https://forge.emrul.dev/Aster/salvo.git` |

Until this release, salvo's `main` was itself the pinned integration line: upstream merged in, patches cherry-picked on top, never force-pushed. `upgrade/salvo-v1.0` now holds that line at the same commit, and `main` became upstream `main` plus one commit (the README preamble and this ledger).

Remotes: `origin` is `https://emrul@forge.emrul.dev/Aster/salvo.git` (Forge: `main`, integration line, patch branches, tags; push here only); `upstream` is `https://github.com/salvo-rs/salvo` (read-only). The Forge push-mirrors this repository to `github.com/getaster/salvo`; nothing is pushed to GitHub directly. (Earlier checkouts had a `github` remote for `github.com/aster-rpc/salvo`; it is retired.)

Pin block for source-tree builds (`[patch.crates-io]`). The fork patches `h3`, `h3-quinn`, `h3-datagram` and `salvo-http3` internally, and `[patch.crates-io]` does not cross workspaces, so every root re-declares the whole family at the same rev:

```toml
salvo        = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "02002e3d6520125bb4ed124e62da531d74adabe9" }
salvo_core   = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "02002e3d6520125bb4ed124e62da531d74adabe9" }
salvo_macros = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "02002e3d6520125bb4ed124e62da531d74adabe9" }
salvo-http3  = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "02002e3d6520125bb4ed124e62da531d74adabe9" }
h3           = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "02002e3d6520125bb4ed124e62da531d74adabe9" }
h3-quinn     = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "02002e3d6520125bb4ed124e62da531d74adabe9" }
h3-datagram  = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "02002e3d6520125bb4ed124e62da531d74adabe9" }
```

Workspace version spec: `salvo = "=1.0.2001"`. The Aster repository also pins `salvo-http3` directly in `aster-transport-salvo/Cargo.toml` (`version = "=0.7.1001"`, same `rev`); bump it with the patch block. Crates published to the Aster registry: `h3`, `h3-datagram`, `h3-quinn`, `salvo`, `salvo-acme`, `salvo-cache`, `salvo-compression`, `salvo-cors`, `salvo-craft`, `salvo-craft-macros`, `salvo-csrf`, `salvo-flash`, `salvo-http3`, `salvo-jwt-auth`, `salvo-oapi`, `salvo-oapi-macros`, `salvo-otel`, `salvo-proxy`, `salvo-rate-limiter`, `salvo-serde-util`, `salvo-serve-static`, `salvo-session`, `salvo-tus`, `salvo_core`, `salvo_extra`, `salvo_macros`.

### Consumers

Everything in the Aster product family must build one identical Salvo. Mismatched consumer revs mean two different Salvos in the family and subtle WebTransport/H3 breakage.

| Consumer | Pin location |
|---|---|
| The Aster repository | `Cargo.toml` `[patch.crates-io]` (salvo, salvo_core, salvo_macros, salvo-http3, h3, h3-quinn, h3-datagram) |
| portal_desktop | Aster registry, exact `salvo = "=<version>"` (no patch block); on 2026-10-21 `salvo = "=1.0.3"`, `aster = "=0.3.794"` |
| portal-agent | meant to move to the Aster registry, exact `salvo = "=<version>"`; on 2026-10-21 still `salvo = "0.93"` with a git patch block on `github.com/aster-rpc/salvo` `cdfdc90f` (pre-1.0, pre-absorption) |
| Fork manifest (a record, not a build) | `[forks.salvo]` and the `salvo_patch_crates_io` snippet in `iroh-fork-manifest.toml` in the Aster repository (the only copy). |

Since Aster 0.3.355 the whole Salvo family (all 22 crates plus the h3 stack) is published to the Aster registry, so registry consumers can enable any Salvo feature. Keep it that way. The published `salvo` names every sub-crate as an optional dependency, and one left out of the published set keeps its crates.io source: at an Aster-local version that crates.io lacks, publishing fails (v0.3.353 stopped at `salvo-cache ^1.0.1`); at one it has, consumers get a second `salvo_core`. The Aster publisher's `publish-native-stack.py check` rejects a published crate with a path dependency on an unpublished sibling.

## Patches

Each topic branch carries exactly one commit. Keep it that way (one patch, one branch, one commit); if a patch grows a fixup, squash it before re-cherry-picking. Topic branches are not deleted after assembly: they are the canonical source of each patch and are re-cherry-picked after every upstream merge. S3, S7 and S9 were re-cut on v1.0.1 on 2026-10-21 as `<topic>-v1.0.1` branches; the others still apply unchanged from their `v1.0.0` cuts (2026-10-03); check a branch with `git cherry -v <base> <branch>`, which should list exactly one `+`.

**Quinn versus Noq in these rows.** S1–S9 were written against upstream Quinn, and their topic branches still build on it. S11 then swaps the backend: in the assembled line the `quinn` feature, the `conn/quinn` module and the `Quinn*` types keep their names but are Noq. So where S1–S9 say "Quinn", the topic branch means upstream Quinn and the integration line means Noq under the `quinn` names. Their tests pass on Noq (S11's validation lists the hand-off, `transport_acked` and pending-read tests). Names such as `quinn-default-keepalive` and `h3-quinn` are branch and package names and stay as they are.

| Id | Topic branch (base) | What | Why / what in Aster depends on it | Commits (topic / integration) | Status | Conflict or porting notes |
|---|---|---|---|---|---|---|
| S1 | `quinn-default-keepalive` (`v1.0.0`) | Enables QUIC keep-alive for the HTTP/3 (`quinn`) listener: upstream Quinn on the topic branch, Noq in the integration line (S11). | Keeps idle H3 links alive. | `6eef79d8` / `6e5f308a` | carried | Apply first. Lives in `listener.rs`, preserved through the 2026-06-27 raw-quinn conflict resolution. Provides `build_transport_config`, which S4 needs. |
| S2 | `wt-webtransport-stream-control` (`v1.0.0`) | Exposes WebTransport stream control. Adds the vendored h3 stack under `vendor/`. | Per-stream control for WebTransport. | `8d263509` / `4bc3eb0b` | carried | Apply second. S5 edits the `vendor/` tree this patch adds. |
| S3 | `wt-web-transport-owned-v1.0.1` (`v1.0.1`; the `v1.0.0` cut is `wt-web-transport-owned`, `7dc523f3`) | `Request::web_transport_owned()` returns an owned WebTransport session. Since 2026-10-03 it also carries the hand-off fix for upstream #1671/#1672: `web_transport_owned()` records a private `WebTransportOwnedHandOff` marker (forwarded to the response by `service.rs`). If the spawned task still holds the session when the handler returns, `process_web_transport` returns `Ok(None)` (clean hand-off, the accept loop ends, no "WebTransport session is still shared" error). If the Arc is unique again, upstream's split-and-reclaim path runs and the H3 connection keeps serving requests. A session shared without `web_transport_owned()` still gets upstream's error. | A handler can spawn the WebTransport session into its own task; portal_desktop uses only `web_transport_owned()`. | `43a3a7a2` / `5299ac7b` | carried | Interacts with upstream #1671/#1672 `take_unique_arc_extension`. Applies and compiles on bare `v1.0.0`, but its `handoff_tests` (`crates/core/src/conn/quinn/handoff_tests.rs`) pass only with S5 applied, because the vendored h3 test client sends no WebTransport SETTINGS and bare upstream rejects that with `H3_SETTINGS_ERROR`. They pass in the assembled line. Re-cut on v1.0.1 (2026-10-21): the hand-off tests build `QuinnConnection::new(raw, handshake_done, None)`, upstream 1.0.1's constructor (#1711), and accept with `into_0rtt` as the accept loop does. |
| S4 | `quinn-transport-config-tuner` (`quinn-default-keepalive`) | `QuinnListener::transport_config_tuner()` callback, `#[must_use]` since 2026-10-03. | The consumer tunes pacing, windows and congestion control on top of the keep-alive/idle defaults (real-time media). | `f0a7a7ae` / `8568e86e` | carried | Stacked on S1 because it needs `build_transport_config`. |
| S5 | `wt-ratified-webkit-interop` (`wt-webtransport-stream-control`) | WebTransport interop with ratified-era clients (Safari/WebKit Network.framework): advertise `0xc671706a`/`0x14e9cd29`, omit the draft-13 flow-control ids, accept clients that send no WebTransport settings. | Safari 26.4+ (Network.framework) cancels draft-02-only servers. | `690fc04d` / `76d4957e` | carried | Stacked on S2 (edits `vendor/`). `max_webtransport_sessions` must stay 1. |
| S6 | `http-alt-svc-toggle` (`v1.0.0`) | Exposes the `HttpBuilder::auto_alt_svc_header` toggle. | Lets consumers keep page/API traffic on TCP (Safari migrates fetches onto H3 and wedges them); WebTransport dials QUIC directly. | `8b195ee6` / `c524276a` | carried | |
| S7 | `wt-transport-acked-v1.0.1` (`c524276a`, the assembled S6 on v1.0.1; the v1.0.0 cut is `wt-transport-acked` on S5, `be8b54f4`) | `SendStream::transport_acked()` resolves true only when QUIC acked the stream's FIN and all bytes (`stopped() == Ok(None)` on the `quinn` API: upstream Quinn on the topic branch, Noq in the integration line); a peer STOP or error is false. The future owns the QUIC stack's notifier, so the caller can still `reset_with_error_code(0)` a superseded stream. Edits `vendor/` (h3 `BufRecvStream::stream_ref()`, h3-quinn, salvo-http3). | Upstreamed into this fork from portal_desktop's `third_party/aster-wt-ack` (2026-10-03). portal_desktop can drop that directory and its registry `[patch]` once it consumes Aster with Salvo 1.0.1 or later. | `6f1645fd` / `ccc57bd7` | carried | Stacked on the S1–S6 assembly (edits `vendor/`; its tests need S5's ratified SETTINGS). Conflicts trivially with S3 in `crates/core/src/conn/quinn.rs`'s test-module list: keep both `mod` lines. Tests: `crates/core/src/conn/quinn/transport_acked_tests.rs`. The v1.0.1 re-cut adapts the tests to the 3-argument `QuinnConnection::new` (#1711). |
| S9 | `wt-recv-control-cancellation-safe-v1.0.1` (`ccc57bd7`, the assembled S7 on v1.0.1; the v1.0.0-line cut is `wt-recv-control-cancellation-safe` on `0cf945a9`, `9a38a8a9`) | Keeps QUIC receive-stream ownership (Quinn on the topic branch, Noq in the integration line) across pending reads: native `poll_read` with a lazy 64 KiB buffer keeps `STOP_SENDING` and stream identity usable after a cancelled or pending read. | Fixes the pending-read STOP/receive-ID panic. | `b643fdd8` / `c4a3b65c` | carried | Apply after S7. The v1.0.1 re-cut drops its dependency on S8 (gone; see Upstreamed or dropped). Regression checks: a pending read, a stable receive ID, and STOP code 37 observed by the peer. |
| S10 | none (committed on the integration line) | Builds the `salvo-acme` dependency certon without its `rsa-keys` feature. | ACME feature correction, 2026-10-05. | — / `5163264b` (`-x` of `b636f542`) | carried | Not on a topic branch. |
| S11 | `feat/noq-backend-v1.0.1` (`80c472ac`, the assembled 1.0.5 line on v1.0.1, not a bare upstream tag: it needs the vendored H3 and WebTransport patches S2, S5, S7 and S9, like the assembled-base exception in iroh-docs row 41) | Backend substitution: Salvo's HTTP/3 and WebTransport server runs on Aster's Noq fork instead of upstream Quinn. The workspace and the vendored `h3-quinn` alias package `noq` as `quinn` at the identical Git URL and revision; the `quinn` feature, `QuinnListener`, `QuinnAcceptor`, `build_quinn_config`, `quinn_connection()`, `salvo::proto::quinn` and `salvo_http3::quinn` keep their names. `salvo::proto::noq` is the explicit name, and `salvo::proto::quinn` a compatibility re-export of it. The adapter drops h3-quinn's `ReadError::IllegalOrderedRead` arm; tests capture the peer address from `Incoming` and use Noq's boxed socket/`UdpSender` API. An independent `wtransport` 0.7 client (upstream Quinn) is a dev-dependency only. Design notes: `docs/noq-evaluation.md` on the integration line. | One QUIC implementation and patch set across Iroh and HTTP/3: Aster's Noq patches and fixes (including the BBRv3 default from Noq 1.4.1) reach the browser-facing transport, and upstream Quinn leaves normal Salvo server builds. **Supersedes the 2026-05-22 decision to keep two QUIC stacks** (Salvo on stock Quinn, Iroh on Noq): aliasing Noq at the dependency boundary removes the hard-coded Quinn types that blocked it. **Type warning:** the `quinn` API names expose Noq types, incompatible with an upstream `quinn` dependency; an upstream `quinn::Connection` cannot be passed in or retrieved as a request extension, and compiler diagnostics may print both as `quinn::Connection` (compare package/source identities). Aster must port `aster-transport-salvo/src/listener.rs` (Noq's `Box<dyn AsyncUdpSocket>`, `create_sender` instead of `create_io_poller`/`try_send`, `poll_recv(&mut self)`, `NonZeroUsize` segment limits; keep the tracked socket/sender ownership so stop waits for the last socket reference) and the stats conversion in `raw_webtransport.rs` (`ConnectionStats` is `Clone`, not `Copy`; connection-wide byte/packet/loss counters plus the active browser path `PathId::ZERO` with multipath disabled for RTT, cwnd and MTU; keep both snapshots after close; peer address through the path APIs). Its direct `quinn` dependency must become the same Noq source/revision. | `39564f65` + `12904e0e` / `d3fd4766` + `52dbeea4` (`-x`), then the Noq re-pin `6dbf1951` (Noq 1.4.1 at `13dcb5b7f60240d729580014356fca6ab7f18ce3`, replacing the evaluation pin `535bd884`), the version bump `44199ed7` (tag `aster-salvo-v1.0.6`) the Noq 1.4.2 re-pin `a1b1d724` (tag `aster-salvo-v1.0.6-p1`), the Noq 1.4.3 re-pin `b67e38cb` (noq `de89867c`) and its version bump `46bb5d5f` (1.0.7, superseded, never tagged), then `02002e3d`, the noq `=1.4.1001` pin (`379f7367`) under the fork version rule (tag `aster-salvo-v1.0.2001`; 22 Salvo crates 1.0.2001, `h3-quinn` 0.0.11001, `salvo-http3` 0.7.1001 requiring it; `h3` and `h3-datagram` unchanged) on `upgrade/salvo-v1.0.1` | carried (1.0.2001) | Permanent fork patch; the benefit is a common QUIC stack, the cost is rechecking the adapter on every Salvo or Noq upgrade: error mapping, native stream polling, FIN-acknowledgement semantics, handshake handling, socket test helpers, and the browser-facing defaults table in `docs/noq-evaluation.md`. Update both Noq declarations together, matching the Iroh and Aster pin. `vendor/h3-quinn` source differs from the previous line, so a version bump checks `vendor/` against this line, not 1.0.5. Local validation (2026-10-22, Noq via a temporary path override): `salvo_core` lib 358 passed (incl. `quinn_webtransport_client_interoperates_with_noq_server`, the hand-off, `transport_acked`, pending-read and stalled-handshake tests), doctests 59 passed / 30 ignored, `salvo` feature check and minimal `server,quinn,ring` build, workspace Clippy with the two documented allowances, nightly fmt clean; the normal dependency graph has Noq and no upstream Quinn. Still required before tagging: Aster's transport/edge suites and listener rebind-after-stop, Chrome/Safari negotiation captures, and the Quinn-versus-Noq H3/WebTransport performance gate with an explicitly common controller (Noq now defaults to BBRv3). |

Version-bump commits are not patches; redo them on each absorption so registry versions stay immutable: `d4cdbbd3` (1.0.1 family, h3 0.0.9, h3-quinn 0.0.11, salvo-http3 0.7.1, vendored h3-datagram 0.0.3), `e9b48e52` (1.0.2), `0cf945a9` (1.0.3), `35aa745d` (1.0.4, h3-quinn 0.0.12, salvo-http3 0.7.2), `80c472ac` (1.0.5 on v1.0.1), `44199ed7` (1.0.6 with S11, h3-quinn 0.0.13, salvo-http3 0.7.3; `vendor/h3-quinn` source changed), `46bb5d5f` (1.0.7 on noq 1.4.3, h3-quinn 0.0.14, salvo-http3 0.7.4; superseded, never published), `02002e3d` (1.0.2001 under option E, h3-quinn 0.0.11001, salvo-http3 0.7.1001; only the vendored manifests changed against 1.0.6, which is published and requires noq `=1.4.2`). Each one also carries what no patch commit does: `vendor/h3-datagram`, the vendored crates' versions in `vendor/*/Cargo.toml`, and in the root `Cargo.toml` the vendored versions (`salvo-http3 = "0.7.2"`, `h3-datagram = "0.0.3"`) plus the `h3-datagram = { path = "vendor/h3-datagram" }` `[patch.crates-io]` line. A bump that only sets the family version loses all of that. Take `vendor/` from the previous line (`git checkout <previous line> -- vendor`), carry those root lines over, set `[workspace.package] version`, and confirm `git diff --quiet <previous line> HEAD -- vendor` unless a vendored crate's source changed (then bump that crate only). Salvo has no `Cargo.lock`, so there is no lock to refresh.

`vendor/h3-datagram` is crates.io 0.0.2 (sha256 `9d2c9f77…fdf0f`) with only its version (0.0.3) and its h3 requirement changed. It used to be a registry bridge, and its `^0.0.8` h3 requirement would have pulled in a second h3; the root `[patch.crates-io]` now maps `h3-datagram` too.

### Upstreamed or dropped

| Date | Patch (branch) | Landed upstream as | Status | Consumer impact |
|---|---|---|---|---|
| 2026-06-27 | `quinn-webtransport-raw-connection` (`3c3298b8`, "Expose raw Quinn connection to WebTransport") | [PR #1528](https://github.com/salvo-rs/salvo/pull/1528) "Expose Quinn connection for WebTransport requests" (commit `037a1577`, merged 2026-06-18) | upstreamed | Upstream named the accessor `Request::quinn_connection() -> Option<crate::proto::quinn::Connection>`; Aster had exposed it via the `QuinnConnection::quinn()` field plus a `::quinn::Connection` request extension. The Aster transport (`aster-transport-salvo`) uses only `web_transport_mut()` and portal_desktop only `web_transport_owned()`. Correction (2026-10-03): portal-agent did use the raw handle, reading `req.extensions().get::<quinn::Connection>()` (`transport.rs:6382`); that still works on upstream 1.0, which inserts the same extension, but portal-agent was pinned to `cdfdc90f` (pre-absorption) at the time; migrate it to `req.quinn_connection()` when it is bumped. Branch deleted; the original commit is not an ancestor of the integration line (which carries a cherry-picked copy plus upstream's version), so it is archived as tag `archive/quinn-webtransport-raw-connection`. Do not re-cherry-pick. |
| 2026-10-21 | S8 `quinn-accept-cancellation-safe` (`097f8ac6`; integration `9eeed585` on `upgrade/salvo-v1.0`) | [PR #1711](https://github.com/salvo-rs/salvo/pull/1711) "finish QUIC handshakes on the connection task" (`9c36d16b`, in v1.0.1) | dropped (superseded) | Upstream solves the same problem better: the QUIC handshake finishes on the connection task, so a cancelled accept has nothing to lose, and v1.0.1 ships its own tests (`crates/core/src/test/quinn_accept.rs`: a joined accept keeps the QUIC handshake; a stalled handshake times out, does not block other connections, and does not hold up graceful stop). `git cherry -v v1.0.1` still shows `+`, because the code differs, not because the behaviour is missing. Migration note: no consumer edits. `aster-transport-salvo` (`tls.rs`) and `aster-expose` (`edge.rs`) use only the public `quinn(...).join(tcp).bind()` / `JoinedAcceptor::new` API, which is unchanged, and the Aster bump touched only pins. Code that constructs `QuinnConnection` itself (here only S3/S7's tests) now calls `QuinnConnection::new(raw, handshake_done, None)`. Bump to `aster-salvo-v1.0.5` (`80c472ac`). The branch is kept unchanged, so no archive tag is needed. Do not re-cherry-pick. |

On 2026-10-21 (v1.0.1) S8 was dropped (above); the other eight topic branches are still `+` against `v1.0.1` (S10 has none). On 2026-10-03 (v1.0.0) nothing graduated: all six patches then existing were still `+` against `v1.0.0`.

## Planned: drop S1 and S4 at the next port

Decided 2026-10-25. Upstream `QuinnListener::new` takes a complete `ServerConfig` and keeps its transport settings; S1 overwrites them with a 5 s keep-alive / 30 s idle timeout, and S4 exists only to let callers tune after that overwrite. At the next port, do not re-cherry-pick S1 or S4; move both rows to "Upstreamed or dropped" (dropped, not upstreamed).

Consumers, in the same release:

- **Aster** (`aster-transport-salvo/src/tls.rs`): `serve_https` and `serve_https_with` set the 5 s / 30 s defaults on the `ServerConfig` they build, then run the tuner, instead of calling `transport_config_tuner()`. Their signatures and documented behaviour stay. Check `aster-expose` (`edge.rs`) and any other `QuinnListener` use for reliance on S1's keep-alive.
- **portal_desktop**: no code change (its actor listener sets its own transport config; its WebTransport server goes through `serve_https_with`). Migration note: `docs/20261025_Salvo_S1_S4_Removal_Migration.md` in that repository.

## Porting to a new upstream release

1. Fetch upstream and tags (`git fetch upstream --tags`) and mirror the release tag to the Forge (`git push origin <upstream-tag>`). Prefer a release tag over a moving upstream `main`.
2. **Triage upstream's new commits against every patch: did upstream land an equivalent?** Signals: `git cherry -v <upstream-tag> <branch>` shows `-` (upstream supplied an equivalent; for stacked branches read the result for the branch's own tip commit), or an upstream commit touches the same API surface (for example adds a `web_transport_*` accessor or exposes the raw quinn connection) and covers the use case. Compare against the upstream tag, never against the integration line, which already holds copies of the patches. `git cherry` compares by patch-id, not commit hash, so it detects an equivalent change under a different SHA; the upstream tag answers "did upstream absorb this?", the integration line answers "is this patch in our assembly?".

   ```bash
   for b in quinn-default-keepalive wt-webtransport-stream-control \
            wt-web-transport-owned-v1.0.1 quinn-transport-config-tuner \
            wt-ratified-webkit-interop http-alt-svc-toggle \
            wt-transport-acked-v1.0.1 \
            wt-recv-control-cancellation-safe-v1.0.1; do
     echo "=== $b ==="
     git cherry -v <upstream-tag> "$b"   # '-' = upstream has an equivalent: graduate it
   done
   ```

   For a stacked branch (S4, S5, S7, S9; S7 and S9 sit on assembled integration commits) this lists the commits it sits on as well; read the line for the branch's own tip commit. To confirm a branch still holds exactly one patch, run `git cherry -v <branch base> <branch>` against its own base.

3. Build the new integration branch in a separate worktree cut from the upstream tag, and cherry-pick the patches in stack order S1 → S10: keepalive (S1) → stream-control (S2) → owned (S3) → tuner (S4) → ratified (S5) → alt-svc (S6) → transport-acked (S7) → cancellation-safe receive control (S9) → certon features (S10); then a version-bump commit (S8 was dropped at v1.0.1). A patch that no longer applies is re-cut as a new branch `<topic>-v<upstream version>`, based on the upstream tag or, when it builds on other patches, on the assembled commit it needs; never force-push the old branch.

   ```bash
   git worktree add -b upgrade/salvo-v<upstream version> ../salvo-v<upstream version> <upstream-tag>
   cd ../salvo-v<upstream version>
   # S1 6eef79d8, S2 8d263509, S3 43a3a7a2, S4 f0a7a7ae, S5 690fc04d,
   # S6 8b195ee6, S7 6f1645fd (keep both `mod` lines against S3),
   # S9 b643fdd8, S10 b636f542 (integration-line commit; no topic branch)
   git cherry-pick -x 6eef79d8 8d263509 43a3a7a2 f0a7a7ae 690fc04d \
                      8b195ee6 6f1645fd b643fdd8 b636f542
   # version-bump commit (keeps vendor/ byte-identical; see Patches), then
   git push origin upgrade/salvo-v<upstream version>
   ```

   Version: publish the next patch above the highest Aster-published Salvo version (1.0.4 → 1.0.5 on base v1.0.1). Bump h3/h3-quinn/salvo-http3 only if their src changed; otherwise `git diff --quiet <previous line> <new line> -- vendor` must hold. Bump the Aster repository's direct `salvo-http3` pin (`aster-transport-salvo/Cargo.toml`) with the rest of the family. If upstream itself ships a version Aster has already published (for example a Salvo 1.0.x or h3 0.0.9), take the next patch above both.
4. Completeness check: the assembled stack on the upstream tag must equal the previous integration line's tree apart from upstream's changes. `git cherry -v <integration-line> <branch>` no longer reliably prints `-` for every patch, because the old line carried pre-1.0 copies and split S3/S4 across two commits each, so their patch-ids differ; use tree equality instead.
5. Build and smoke-test the consumers (below), then tag (below).

Retiring a patch that upstream accepted:

1. Accept the upstream version; do not re-cherry-pick ours. Delete the redundant branch only after confirming the upstream API is a true superset. Before deleting, if the branch's commit is not an ancestor of the integration line (it usually is not), archive the original SHA as a tag so the graduated-patches table stays resolvable: `git tag archive/<branch> <sha> && git push origin archive/<branch>`.
2. Check whether consumers must change (upstream's API may differ in name, signature or feature gating). If the Aster transport (`aster-transport-salvo`), `aster-expose` or portal_desktop must change, write the migration note in the patch's "Upstreamed or dropped" row (its Consumer impact column): which patch graduated and the upstream commit/PR that replaced it; our API and upstream's side by side; the exact edits each consumer needs (or that none are needed); and the tag to bump to. Each consumer's own `YYYYMMDD_Aster_Pin_Update.md` repeats the edits it needs.
3. Move the row from "Patches" to "Upstreamed or dropped".

If a patch is partly subsumed, keep the branch but trim its commit to the still-missing delta, then re-cherry-pick the slimmed patch.

Never rewrite published history: consumers pin a git rev, and rewriting would invalidate every pinned rev. Each new integration line only gains commits, and a consumer pinned to an old rev keeps building; bumping is a deliberate edit. A failed attempt is superseded by new commits and a new version; no `--force` push is ever used.

Tagging, only after the Aster consumer trial passes against these exact commits (tags cannot be moved):

```bash
git tag -a aster-salvo-v<published version> -m "Aster salvo v<published version>" <commit>   # e.g. aster-salvo-v1.0.5
git push origin upgrade/salvo-v<upstream> aster-salvo-v<published version>
```

Tags are immutable and point at the fully assembled line. Push to `origin` (Forge) only. Consumers should prefer the tag over raw revs. The dated `aster-salvo-YYYY-MM-DD` and `v<upstream-version>-aster.<n>` tags are legacy and are not created any more.

When the pinned commit moves, bump every consumer's pin to match, and update `[forks.salvo]` plus the `salvo_patch_crates_io` snippet in `iroh-fork-manifest.toml` in the Aster repository (the only copy). Update this ledger on `main` (merge upstream `main` first). Smoke test after a bump:

```bash
# in the Aster repository
cargo check --all-targets -p aster-transport-salvo -p aster-expose --features aster-expose/edge
cargo test -p aster-transport-salvo            # raw_webtransport.rs: >= 5 passing (CI gate)
cargo test -p aster-expose --features edge --lib --tests
# portal_desktop builds its own WebTransport path against web_transport_owned()
```

Validation used for releases: `cargo fmt` check; clippy (workspace, all features, all targets), with the existing `collapsible_if` (cookie extraction) and `redundant_clone` (TCP test) diagnostics allowed, since strict clippy still reports those unrelated warnings; `salvo_core` library tests with `quinn,rustls` (or `quinn ring server-handle`); the vendored crates tested in a temporary workspace; then Aster's transport suite, including the raw WebTransport real-network lifecycle/control tests.

## History

### 2026-10-28: Noq 1.4.1001, 1.0.2001 (current)

- **1.0.2001** (`02002e3d`, tag `aster-salvo-v1.0.2001` after the Aster release candidate passes): the fork version rule's option E puts Aster's first release on upstream v1.0.1 at 1.0.2001. It supersedes **1.0.7** (`46bb5d5f`, noq 1.4.3 at `de89867c`, h3-quinn 0.0.14, salvo-http3 0.7.4), which was never published or tagged; noq 1.4.3 itself became noq 1.4.1001 (`379f7367`). Vendored crates: `h3-quinn` was cut from crates.io 0.0.10 and `salvo-http3` from 0.7.0 (first vendored in S2 `8d263509`; crates.io has nothing newer), so they become `0.0.11001` and `0.7.1001`; neither exists on crates.io. `h3` 0.0.9 and `h3-datagram` 0.0.3 are served and unchanged. Both noq declarations are `=1.4.1001` at `379f73677c057908b7e43a40466e38d2bbe75630`. The history below keeps the 1.0.7 notes for the commits it describes: the Noq 1.4.3 re-pin `b67e38cb` (both declarations to `=1.4.3` at `de89867c975192d69438164bada087506133c5a4`: adaptive reordering thresholds, the BBRv3 undo-state fix, the path's minimum RTT in `PathStats`), then the version bump. The registry already serves Salvo 1.0.6, h3-quinn 0.0.13 and salvo-http3 0.7.3 requiring noq `=1.4.2`, and a published crate cannot change, so the family moves: 22 Salvo crates 1.0.7, `h3-quinn` 0.0.14 (its noq requirement changed), `salvo-http3` 0.7.4 (requires h3-quinn 0.0.14); `h3` 0.0.9 and `h3-datagram` 0.0.3 unchanged. `git diff a1b1d724 46bb5d5f -- vendor` touches only `vendor/h3-quinn/Cargo.toml` and `vendor/salvo-http3/Cargo.toml`. Validation: `salvo_core` lib (quinn, rustls) 358 passed, including `quinn_webtransport_client_interoperates_with_noq_server`, the hand-off, `transport_acked` and pending-read tests; `salvo` minimal `server,quinn,ring` check; workspace clippy (all targets) with the two baseline allowances reports no errors; the normal graph resolves one noq 1.4.3 (`de89867c`) and no upstream Quinn (the lock's quinn 0.11 is the `wtransport` dev-dependency). Aster's transport and edge suites run with the Aster pin. Validation of `02002e3d` (2026-10-28): `salvo_core` lib (quinn, rustls) 358 passed; `salvo` minimal `server,quinn,ring` check; workspace clippy (all targets, all features) with the two baseline allowances reports no errors; the normal graph resolves one noq 1.4.1001 (`379f7367`) and no upstream Quinn.

### 2026-10-22: Noq backend, 1.0.6 and 1.0.6-p1

- S11 (Noq backend substitution) on `upgrade/salvo-v1.0.1`: `d3fd4766` + `52dbeea4` (`-x` of `39564f65` + `12904e0e`), Noq re-pin `6dbf1951` (noq 1.4.1), version commit `44199ed7` (22 Salvo crates 1.0.6, h3-quinn 0.0.13, salvo-http3 0.7.3; tag `aster-salvo-v1.0.6`). Supersedes the 2026-05-22 two-QUIC-stack decision.
- **1.0.6-p1** (`a1b1d724`, tag `aster-salvo-v1.0.6-p1`): the workspace and vendored `h3-quinn` noq aliases move to `=1.4.2` at `a3c70b7a` (incremental BBRv3 packet history). No source change and the versions stay, because 1.0.6 has never been published; the exact noq requirement is in the published manifests, so 1.0.6 publishes from this commit, not from `44199ed7`. Validation: `salvo_core` lib (quinn, rustls) 358 passed, doctests 59 passed (30 ignored); `salvo` feature check and minimal `server,quinn,ring` build; workspace clippy with the two baseline allowances; nightly fmt clean; the normal graph resolves one noq 1.4.2 and no upstream Quinn. Aster's transport and edge suites, the listener rebind test, browser captures and the Quinn-versus-Noq performance gate run with the Aster port.

### 2026-10-21: upstream v1.0.1, 1.0.5

- New integration branch `upgrade/salvo-v1.0.1`, cut from `v1.0.1` (`083a17e5`), tip `80c472ac9e19bd6bba01e734fa2bf8503d569dd1`; tag `aster-salvo-v1.0.5` on that commit once the Aster consumer trial passes. Stack: S1 `6e5f308a`, S2 `4bc3eb0b`, S3 `5299ac7b`, S4 `8568e86e`, S5 `76d4957e`, S6 `c524276a`, S7 `ccc57bd7`, S9 `c4a3b65c`, S10 `5163264b`, version commit `80c472ac`.
- S8 dropped: upstream #1711 (`9c36d16b`) finishes QUIC handshakes on the connection task, which covers it (see Upstreamed or dropped; no consumer edits). S3, S7 and S9 re-cut as `wt-web-transport-owned-v1.0.1` (`43a3a7a2`, on `v1.0.1`), `wt-transport-acked-v1.0.1` (`6f1645fd`, on the assembled S6) and `wt-recv-control-cancellation-safe-v1.0.1` (`b643fdd8`, on the assembled S7): their tests build #1711's three-argument `QuinnConnection::new`, and S9 no longer sits on S8. The `v1.0.0` branches are unchanged. The integration commits carry both `-x` lines (original, then re-cut).
- Versions: all 22 Salvo crates 1.0.5 (the registry serves 1.0.1, 1.0.3 and 1.0.4); h3 0.0.9, h3-quinn 0.0.12, salvo-http3 0.7.2 and h3-datagram 0.0.3 unchanged, `vendor/` byte-identical to `35aa745d`. 26 registry crates in all.
- Validation on `80c472ac`: `salvo_core` 357 library tests passed, including the four hand-off, three transport-ACK/receive-control and upstream's four `quinn_accept` tests; workspace clippy (all targets) passes with the two baseline allowances (`collapsible_if`, `redundant_clone`). Aster's `aster-transport-salvo` suite passed against the pin, including all five raw WebTransport real-network tests.

### 2026-10-11: receive-control release

- Forgejo `main` and immutable tags `v1.0.0-aster.5` / `aster-salvo-2026-10-11` point at `35aa745dfca50c2e5a640c0d0c93153b033fa931`. Patch commit `9a38a8a9` (S9) fixes the pending-read STOP/receive-ID panic; the following version commit keeps registry versions immutable: all 22 Salvo crates 1.0.4, `h3-quinn` 0.0.12, `salvo-http3` 0.7.2; `h3` stays 0.0.9 and `h3-datagram` 0.0.3. The release was assembled in an isolated worktree, leaving the original developer checkout unchanged.
- Validation at this exact release: 362 `salvo_core` library tests passed with `quinn,rustls`, including all 18 QUIC tests. All-target clippy passed with only the two existing baseline diagnostics allowed; strict clippy still reports them. Aster's full transport suite passed against this pin, including all five raw WebTransport real-network lifecycle/control tests. Registry publication and downstream consumer pins remain the consuming workspace's release gate.
- This commit is the tip of `upgrade/salvo-v1.0`, the last release on upstream v1.0.0.

### 2026-10-07: cancellation-safe accept, 1.0.3 (source-published; registry pending)

- Canonical `quinn-accept-cancellation-safe` has exactly one commit, `097f8ac6f88b15a012e39ae184a42e311b4243fe`, directly on `v1.0.0`.
- Local assembly `review/quinn-accept-cancellation-1.0.3-final` is `0cf945a9e9ac94cfe718dbed46ecf21f8a67a492`: cherry-pick `9eeed585` onto the existing line, then a separate Salvo family version bump. Upstream base and H3 versions unchanged. All 22 family 1.0.3 versions were absent from the anonymous Aster sparse index during preparation.
- Source tags `v1.0.0-aster.4` / `aster-salvo-2026-10-07`, the integration line and the canonical patch branch were pushed and are anonymously readable. The 1.0.3 registry wave was pending the user-approved standard Aster verification release workflow, including dependent dry-run and consumer gates.
- Evidence: a pinned baseline witness asserts post-HANDSHAKE_DONE application close 0 and an empty reason; the inverted same-peer regression fails on the baseline. The final assembly passes nine focused cancellation tests, including a ready handshake resumed after its deadline. The final tree equals the accepted `69fe9c62` review. The earlier assembly `d282442d` passed 360 unit tests and 59 doctests (30 ignored), including eight cancellation tests, four WebTransport hand-off and two transport-ACK tests; eight regressions also passed on bare upstream. These full-suite counts carry forward across the test/comment-only delta, rather than claiming a fresh 361-test run. Aster's build and 62 consumer tests also carry from that assembly; the wheel/stubs then present were 0.3.770, not a final-HEAD artifact. Old review refs remain; the prior canonical identity is also archived as `review/archive/quinn-accept-cancellation-safe-8bf6ea5c`.
- Clippy passed with allowances for the existing `collapsible_if` and `redundant_clone`; strict clippy remained blocked by them. Format check passed.
- Aster integration was adopted locally as `93bdeede` then `4eec1c3b` (Aster release-assurance line); the iroh-fork manifest (BOM) commits were `5d5d3ed1e1` then `d5498a0efb`. Both manifest copies matched and recorded the published Salvo line with its registry wave pending. Public consumer instructions kept the available 1.0.1 and named 1.0.3 as the next wave. Local source checks are not a public consumer verdict.

### 2026-10-05: ACME feature correction, 1.0.2

- The line was `e9b48e52`: `b636f542` (S10) drops certon's `rsa-keys` feature, followed by the 1.0.2 family version commit. `v1.0.0-aster.3` / `aster-salvo-2026-10-05` record the 1.0.2 assembly; the anonymous Aster index still served 1.0.1.

### 2026-10-03 (later): `wt-transport-acked`, crates 1.0.1

- New patch S7 (`be8b54f4`, on `wt-ratified-webkit-interop`), cherry-picked as `c2c53442`.
- Registry versions are immutable, so `d4cdbbd3` bumped versions: h3 0.0.9, h3-quinn 0.0.11, salvo-http3 0.7.1; h3-datagram 0.0.3 vendored (see Patches); the Salvo family 1.0.1, because salvo_core's exact h3-datagram requirement moved. `upstream_base` stayed `v1.0.0`.
- Tags `v1.0.0-aster.2` and `aster-salvo-2026-10-03.2` point at `d4cdbbd3`.
- Validation: salvo_core 352 lib tests with `quinn ring server-handle`, including the 4 hand-off and 2 transport-ACK tests. No new clippy warnings; `cargo check --workspace --all-targets` passes.
- portal_desktop can drop `third_party/aster-wt-ack` and its registry `[patch]` once it consumes Aster with Salvo 1.0.1 (see its `docs/20261003_Aster_Pin_Update.md`).

### 2026-10-03: upstream v1.0.0

- The line moved from `0e8477c1` by merging `v1.0.0` (`50035977`). One conflict, in `crates/core/src/conn/quinn/listener.rs`: upstream's rustfmt of the initial `config_stream.next()` call against the Aster `let mut initial: ServerConfig` plus tuner wiring. Took upstream's formatting and kept the Aster lines. The merged tree is identical to the trial merge and to a fresh cherry-pick stack on v1.0.0.
- Graduation check (`git cherry -v v1.0.0 <branch>`): all six patches still `+`; nothing graduated.
- `9ed1e06e` fix(core): treat a still-shared owned WebTransport session as a hand-off (with e2e tests); interacts with upstream #1671/#1672 `take_unique_arc_extension`. `c86d797c` fix(quinn): `#[must_use]` on `transport_config_tuner` (clippy). Both were folded into their patch branches; all six branches were re-cut on `v1.0.0`, old tips archived as `archive/<branch>-pre-1.0`.
- Tags `aster-salvo-2026-10-03` and `v1.0.0-aster.1` point at `c86d797c`.
- `vendor/h3`, `vendor/h3-quinn` and `vendor/salvo-http3` had no source changes since `0e8477c1`; registry versions h3 0.0.8, h3-quinn 0.0.10 and salvo-http3 0.7.0 were reused. The Salvo family (salvo, salvo_core, salvo_macros, salvo-serde-util, salvo-acme, salvo-proxy, salvo-serve-static, salvo-otel) became 1.0.0.
- Validation: fmt clean; clippy (workspace, all features, all targets) passes with only pre-existing upstream/vendor warnings; `salvo_core` 362 unit tests and 61 doctests (30 ignored); the vendored crates in a temporary workspace: h3 227 + 8 doctests (h3-quinn and salvo-http3 have no tests).
- Still needs browser testing: real Safari/Chrome WebTransport sessions against portal_desktop/portal-agent on the new rev, especially the new reclaim path, where a `web_transport_mut()` handler returns, Salvo splits the session and closes the CONNECT stream, and the connection keeps serving requests.

### 2026-06-27: upstream absorption, raw-quinn graduated

- The upstream merge conflicted in `crates/core/src/conn/quinn.rs`, `.../quinn/builder.rs` and `.../quinn/listener.rs` (the Aster `raw_quinn` field/threading versus upstream's `raw`). Resolved by taking upstream's side for the raw-quinn regions (graduated; see Upstreamed or dropped) while preserving the keep-alive patch elsewhere in `listener.rs`. `request.rs` auto-merged: `web_transport_owned()` sits on top of upstream's `quinn_connection()` / `web_transport_mut()`, all on the `crate::proto::` types.
