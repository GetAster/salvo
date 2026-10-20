# Aster patch ledger: salvo

This repository is Aster's soft fork of [salvo-rs/salvo](https://github.com/salvo-rs/salvo). Aster carries the small set of patches listed below, which expose lower-level QUIC and WebTransport internals that the Aster HTTP transport and the Portal desktop app need; everything else tracks upstream verbatim. Upstream maintainers are welcome to incorporate any of them, in whole or in part, at their discretion. The fork is meant to shrink: patches are expected to graduate upstream, not accumulate.

## Branch and tag model

- `main` is upstream `main` plus one commit: the Aster README preamble and this ledger. Nothing is built from `main`. It is refreshed with `git merge upstream/main` and is never reset or force-pushed. Ledger edits are commits on `main`, never on `upgrade/*`.
- `upgrade/<repo>-v<upstream version>` (for example `upgrade/iroh-v1.4.0`, `upgrade/iroh-blobs-v0.103.1`) is the patched integration line: the upstream release plus the patches in this ledger. A new upstream base always gets a new branch, cut from the upstream release tag; Aster-only fixes and re-pins append to the current branch. Per-minor names (`upgrade/iroh-blobs-v0.103`, `upgrade/salvo-v1.0`, …) are historical. Consumers pin commits on it. Published `upgrade/*` branches and tags are never rewritten; a failed attempt is superseded by new commits and a new version.
- Topic branches carry one change each, are named for the change (for example `feat/read-into`, `security/plumtree-bounds`), are based on the upstream release tag, and are cherry-picked with `git cherry-pick -x` onto the upgrade branch, so every upgrade-branch commit names its source.
- Version: the upstream version if Aster's registry has never served that version or a higher one in the series; otherwise the next patch above Aster's highest published version. A re-pin without a code change keeps the version and takes the tag suffix `-p<n>`.
- Tags: `aster-<repo>-v<published version>` (re-pins `aster-<repo>-v<published version>-p<n>`), created only after the Aster consumer trial passes against those exact commits, because tags cannot be moved.
- The fork manifest is `iroh-fork-manifest.toml` in the Aster repository (the only copy).
- Aster does not submit patches upstream; upstream maintainers are welcome to take any patch here.

Historical and current tag schemes in this repository:

- `aster-salvo-YYYY-MM-DD`, the date of an absorption or release, with `.n` for a second release the same day: `aster-salvo-2026-06-27`, `-2026-07-05`, `-2026-10-03`, `-2026-10-03.2`, `-2026-10-05`, `-2026-10-07`, `-2026-10-11`.
- `aster-salvo-v<published version>`, the current scheme for new tags (the next is `aster-salvo-v1.0.5`). The two schemes below are legacy.
- `v<upstream version>-aster.<n>`, since 2026-10-03, on the same commit as the dated tag: `v1.0.0-aster.1` … `v1.0.0-aster.5`. `<n>` was bumped for each re-assembly on the same upstream version and reset to 1 for a new upstream release.
- `archive/<branch>`, keeping a deleted or re-cut patch branch's original commit resolvable: `archive/quinn-webtransport-raw-connection`, and `archive/<branch>-pre-1.0` for the six branches re-cut on v1.0.0.

Topic branches in this repository predate the `feat/`/`fix/` naming and are named for the change directly (for example `quinn-default-keepalive`). Each holds exactly one commit; see "Patches". Some are stacked by design rather than based on the upstream tag, because they build on another patch: S4 on S1, S5 on S2, S7 on S5 (and S9 on the assembled 1.0.3 line, for the transport-ACK tests). Check each with `git cherry -v <branch base> <branch>` against its own base, which should list exactly one `+`.

## Current release

| Field | Value |
|---|---|
| Upstream base | `v1.0.0` (`50035977` is the merge of `v1.0.0` into the old `main`) |
| Integration branch | `upgrade/salvo-v1.0` (a historical per-minor name; the next upstream base gets `upgrade/salvo-v<upstream version>`, for example `upgrade/salvo-v1.0.1`) |
| Tags | `v1.0.0-aster.5` / `aster-salvo-2026-10-11` |
| Pinned commit | `35aa745dfca50c2e5a640c0d0c93153b033fa931` |
| Crate versions | all 22 Salvo crates 1.0.4; `h3-quinn` 0.0.12; `salvo-http3` 0.7.2; `h3` 0.0.9; `h3-datagram` 0.0.3 |
| Status | source, tags and the 1.0.4 registry wave published (the Aster registry serves all 22 Salvo crates at 1.0.4) |
| Source repository | `https://forge.emrul.dev/Aster/salvo.git` |

Until this release, salvo's `main` was itself the pinned integration line: upstream merged in, patches cherry-picked on top, never force-pushed. `upgrade/salvo-v1.0` now holds that line at the same commit, and `main` became upstream `main` plus one commit (the README preamble and this ledger).

Remotes: `origin` is `https://emrul@forge.emrul.dev/Aster/salvo.git` (Forge: `main`, integration line, patch branches, tags; push here only); `upstream` is `https://github.com/salvo-rs/salvo` (read-only). The Forge push-mirrors this repository to `github.com/getaster/salvo`; nothing is pushed to GitHub directly. (Earlier checkouts had a `github` remote for `github.com/aster-rpc/salvo`; it is retired.)

Pin block for source-tree builds (`[patch.crates-io]`). The fork patches `h3`, `h3-quinn`, `h3-datagram` and `salvo-http3` internally, and `[patch.crates-io]` does not cross workspaces, so every root re-declares the whole family at the same rev:

```toml
salvo        = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "35aa745dfca50c2e5a640c0d0c93153b033fa931" }
salvo_core   = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "35aa745dfca50c2e5a640c0d0c93153b033fa931" }
salvo_macros = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "35aa745dfca50c2e5a640c0d0c93153b033fa931" }
salvo-http3  = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "35aa745dfca50c2e5a640c0d0c93153b033fa931" }
h3           = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "35aa745dfca50c2e5a640c0d0c93153b033fa931" }
h3-quinn     = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "35aa745dfca50c2e5a640c0d0c93153b033fa931" }
h3-datagram  = { git = "https://forge.emrul.dev/Aster/salvo.git", rev = "35aa745dfca50c2e5a640c0d0c93153b033fa931" }
```

Workspace version spec: `salvo = "=1.0.4"`. Crates published to the Aster registry: `h3`, `h3-datagram`, `h3-quinn`, `salvo`, `salvo-acme`, `salvo-cache`, `salvo-compression`, `salvo-cors`, `salvo-craft`, `salvo-craft-macros`, `salvo-csrf`, `salvo-flash`, `salvo-http3`, `salvo-jwt-auth`, `salvo-oapi`, `salvo-oapi-macros`, `salvo-otel`, `salvo-proxy`, `salvo-rate-limiter`, `salvo-serde-util`, `salvo-serve-static`, `salvo-session`, `salvo-tus`, `salvo_core`, `salvo_extra`, `salvo_macros`.

### Consumers

Everything in the Aster product family must build one identical Salvo. Mismatched consumer revs mean two different Salvos in the family and subtle WebTransport/H3 breakage.

| Consumer | Pin location |
|---|---|
| The Aster repository | `Cargo.toml` `[patch.crates-io]` (salvo, salvo_core, salvo_macros, salvo-http3, h3, h3-quinn, h3-datagram) |
| portal_desktop | Aster registry, exact `salvo = "=<version>"` (no patch block) |
| portal-agent | Aster registry, exact `salvo = "=<version>"` (moving off its git patch block with the 2026-10-03 update) |
| Fork manifest (a record, not a build) | `[forks.salvo]` and the `salvo_patch_crates_io` snippet in `iroh-fork-manifest.toml` in the Aster repository (the only copy). |

Since Aster 0.3.355 the whole Salvo family (all 22 crates plus the h3 stack) is published to the Aster registry, so registry consumers can enable any Salvo feature. Keep it that way. The published `salvo` names every sub-crate as an optional dependency, and one left out of the published set keeps its crates.io source: at an Aster-local version that crates.io lacks, publishing fails (v0.3.353 stopped at `salvo-cache ^1.0.1`); at one it has, consumers get a second `salvo_core`. The Aster publisher's `publish-native-stack.py check` rejects a published crate with a path dependency on an unpublished sibling.

## Patches

Each topic branch carries exactly one commit. Keep it that way (one patch, one branch, one commit); if a patch grows a fixup, squash it before re-cherry-picking. Topic branches are not deleted after assembly: they are the canonical source of each patch and are re-cherry-picked after every upstream merge. All except the last two were re-cut on `v1.0.0` on 2026-10-03; check a branch with `git cherry -v <base> <branch>`, which should list exactly one `+`.

| Id | Topic branch (base) | What | Why / what in Aster depends on it | Commits (topic / integration) | Status | Conflict or porting notes |
|---|---|---|---|---|---|---|
| S1 | `quinn-default-keepalive` (`v1.0.0`) | Enables Quinn keep-alive for the HTTP/3 listener. | Keeps idle H3 links alive. | `6eef79d8` / older pre-1.0 copy on the integration line | carried | Apply first. Lives in `listener.rs`, preserved through the 2026-06-27 raw-quinn conflict resolution. Provides `build_transport_config`, which S4 needs. |
| S2 | `wt-webtransport-stream-control` (`v1.0.0`) | Exposes WebTransport stream control. Adds the vendored h3 stack under `vendor/`. | Per-stream control for WebTransport. | `8d263509` / older pre-1.0 copy | carried | Apply second. S5 and S8 edit the `vendor/` tree this patch adds. |
| S3 | `wt-web-transport-owned` (`v1.0.0`) | `Request::web_transport_owned()` returns an owned WebTransport session. Since 2026-10-03 it also carries the hand-off fix for upstream #1671/#1672: `web_transport_owned()` records a private `WebTransportOwnedHandOff` marker (forwarded to the response by `service.rs`). If the spawned task still holds the session when the handler returns, `process_web_transport` returns `Ok(None)` (clean hand-off, the accept loop ends, no "WebTransport session is still shared" error). If the Arc is unique again, upstream's split-and-reclaim path runs and the H3 connection keeps serving requests. A session shared without `web_transport_owned()` still gets upstream's error. | A handler can spawn the WebTransport session into its own task; portal_desktop uses only `web_transport_owned()`. | `7dc523f3` (includes the hand-off fix and tests) / split across two commits on the integration line, including `9ed1e06e` | carried | Interacts with upstream #1671/#1672 `take_unique_arc_extension`. Applies and compiles on bare `v1.0.0`, but its `handoff_tests` (`crates/core/src/conn/quinn/handoff_tests.rs`) pass only with S5 applied, because the vendored h3 test client sends no WebTransport SETTINGS and bare upstream rejects that with `H3_SETTINGS_ERROR`. They pass in the assembled line. |
| S4 | `quinn-transport-config-tuner` (`quinn-default-keepalive`) | `QuinnListener::transport_config_tuner()` callback, `#[must_use]` since 2026-10-03. | The consumer tunes pacing, windows and congestion control on top of the keep-alive/idle defaults (real-time media). | `f0a7a7ae` / split across two commits on the integration line, including `c86d797c` (`#[must_use]`) | carried | Stacked on S1 because it needs `build_transport_config`. |
| S5 | `wt-ratified-webkit-interop` (`wt-webtransport-stream-control`) | WebTransport interop with ratified-era clients (Safari/WebKit Network.framework): advertise `0xc671706a`/`0x14e9cd29`, omit the draft-13 flow-control ids, accept clients that send no WebTransport settings. | Safari 26.4+ (Network.framework) cancels draft-02-only servers. | `690fc04d` / older pre-1.0 copy | carried | Stacked on S2 (edits `vendor/`). `max_webtransport_sessions` must stay 1. |
| S6 | `http-alt-svc-toggle` (`v1.0.0`) | Exposes the `HttpBuilder::auto_alt_svc_header` toggle. | Lets consumers keep page/API traffic on TCP (Safari migrates fetches onto H3 and wedges them); WebTransport dials QUIC directly. | `8b195ee6` / older pre-1.0 copy | carried | |
| S7 | `wt-transport-acked` (`wt-ratified-webkit-interop`) | `SendStream::transport_acked()` resolves true only when QUIC acked the stream's FIN and all bytes (quinn `stopped() == Ok(None)`); a peer STOP or error is false. The future owns quinn's notifier, so the caller can still `reset_with_error_code(0)` a superseded stream. Edits `vendor/` (h3 `BufRecvStream::stream_ref()`, h3-quinn, salvo-http3). | Upstreamed into this fork from portal_desktop's `third_party/aster-wt-ack` (2026-10-03). portal_desktop can drop that directory and its registry `[patch]` once it consumes Aster with Salvo 1.0.1 or later. | `be8b54f4` / `c2c53442` | carried | Stacked on S5 (edits `vendor/`; its tests need ratified SETTINGS). Conflicts trivially with S3 in `crates/core/src/conn/quinn.rs`'s test-module list: keep both `mod` lines. Tests: `crates/core/src/conn/quinn/transport_acked_tests.rs`. |
| S8 | `quinn-accept-cancellation-safe` (`v1.0.0`) | Retains an admitted QUIC handshake across cancelled accepts. `JoinedAcceptor` can select TCP after QUIC completes; the patch retains the `IncomingFuture`, the admitted fuse configuration, addresses and the original deadline until the next accept. | A cancelled accept no longer drops a completed QUIC handshake (the live Desktop cancellation trigger remains inferred from the evidence). | `097f8ac6f88b15a012e39ae184a42e311b4243fe` / `9eeed585` | carried | Apply after S7. Tests cover real QUIC cancellation/resume, admission/rejection, timeout, drop and the joined TCP winner. Tokio's original ready-first timeout semantics are preserved; a pending handshake still expires at its original deadline. Adopted and source-published 2026-10-07. |
| S9 | `wt-recv-control-cancellation-safe` (`0cf945a9`, the assembled 1.0.3 predecessor; needs the transport-ACK tests) | Keeps Quinn receive ownership across pending reads: native `poll_read` with a lazy 64 KiB buffer keeps `STOP_SENDING` and stream identity usable after a cancelled or pending read. | Fixes the pending-read STOP/receive-ID panic. | `9a38a8a9` / `9a38a8a9` | carried | Apply last, after S8. Regression checks: a pending read, a stable receive ID, and STOP code 37 observed by the peer. |
| S10 | none (committed on the integration line) | Builds the `salvo-acme` dependency certon without its `rsa-keys` feature. | ACME feature correction, 2026-10-05. | — / `b636f542` | carried | Not on a topic branch. |

Version-bump commits are not patches; redo them on each absorption so registry versions stay immutable: `d4cdbbd3` (1.0.1 family, h3 0.0.9, h3-quinn 0.0.11, salvo-http3 0.7.1, vendored h3-datagram 0.0.3), `e9b48e52` (1.0.2), `0cf945a9` (1.0.3), `35aa745d` (1.0.4, h3-quinn 0.0.12, salvo-http3 0.7.2).

`vendor/h3-datagram` is crates.io 0.0.2 (sha256 `9d2c9f77…fdf0f`) with only its version (0.0.3) and its h3 requirement changed. It used to be a registry bridge, and its `^0.0.8` h3 requirement would have pulled in a second h3; the root `[patch.crates-io]` now maps `h3-datagram` too.

### Upstreamed or dropped

| Date | Patch (branch) | Landed upstream as | Status | Consumer impact |
|---|---|---|---|---|
| 2026-06-27 | `quinn-webtransport-raw-connection` (`3c3298b8`, "Expose raw Quinn connection to WebTransport") | [PR #1528](https://github.com/salvo-rs/salvo/pull/1528) "Expose Quinn connection for WebTransport requests" (commit `037a1577`, merged 2026-06-18) | upstreamed | Upstream named the accessor `Request::quinn_connection() -> Option<crate::proto::quinn::Connection>`; Aster had exposed it via the `QuinnConnection::quinn()` field plus a `::quinn::Connection` request extension. The Aster transport (`aster-transport-salvo`) uses only `web_transport_mut()` and portal_desktop only `web_transport_owned()`. Correction (2026-10-03): portal-agent did use the raw handle, reading `req.extensions().get::<quinn::Connection>()` (`transport.rs:6382`); that still works on upstream 1.0, which inserts the same extension, but portal-agent was pinned to `cdfdc90f` (pre-absorption) at the time; migrate it to `req.quinn_connection()` when it is bumped. Branch deleted; the original commit is not an ancestor of the integration line (which carries a cherry-picked copy plus upstream's version), so it is archived as tag `archive/quinn-webtransport-raw-connection`. Do not re-cherry-pick. |

On 2026-10-03 (v1.0.0) nothing graduated: all six patches then existing were still `+` against `v1.0.0`.

## Porting to a new upstream release

1. Fetch upstream and tags (`git fetch upstream --tags`) and mirror the release tag to the Forge (`git push origin <upstream-tag>`). Prefer a release tag over a moving upstream `main`.
2. **Triage upstream's new commits against every patch: did upstream land an equivalent?** Signals: `git cherry -v <upstream-tag> <branch>` shows `-` (upstream supplied an equivalent; for stacked branches read the result for the branch's own tip commit), or an upstream commit touches the same API surface (for example adds a `web_transport_*` accessor or exposes the raw quinn connection) and covers the use case. Compare against the upstream tag, never against the integration line, which already holds copies of the patches. `git cherry` compares by patch-id, not commit hash, so it detects an equivalent change under a different SHA; the upstream tag answers "did upstream absorb this?", the integration line answers "is this patch in our assembly?".

   ```bash
   for b in quinn-default-keepalive wt-webtransport-stream-control \
            wt-web-transport-owned quinn-transport-config-tuner \
            wt-ratified-webkit-interop http-alt-svc-toggle \
            wt-transport-acked quinn-accept-cancellation-safe \
            wt-recv-control-cancellation-safe; do
     echo "=== $b ==="
     git cherry -v <upstream-tag> "$b"   # '-' = upstream has an equivalent: graduate it
   done
   ```

   For a stacked branch (S4, S5, S7, S9) this lists the commits it sits on as well; read the line for the branch's own tip commit. To confirm a branch still holds exactly one patch, run `git cherry -v <branch base> <branch>` against its own base.

3. Build the new integration branch in a separate worktree cut from the upstream tag, and cherry-pick the patches in stack order S1 → S10: keepalive (S1) → stream-control (S2) → owned (S3) → tuner (S4) → ratified (S5) → alt-svc (S6) → transport-acked (S7) → cancellation-safe accept (S8) → cancellation-safe receive control (S9) → certon features (S10); then a version-bump commit.

   ```bash
   git worktree add -b upgrade/salvo-v<upstream version> ../salvo-v<upstream version> <upstream-tag>
   cd ../salvo-v<upstream version>
   # S1 6eef79d8, S2 8d263509, S3 7dc523f3, S4 f0a7a7ae, S5 690fc04d,
   # S6 8b195ee6, S7 be8b54f4 (keep both `mod` lines against S3), S8 097f8ac6,
   # S9 9a38a8a9, S10 b636f542 (integration-line commit; no topic branch)
   git cherry-pick -x 6eef79d8 8d263509 7dc523f3 f0a7a7ae 690fc04d \
                      8b195ee6 be8b54f4 097f8ac6 9a38a8a9 b636f542
   # version-bump commit, then
   git push origin upgrade/salvo-v<upstream version>
   ```

   Version: publish the next patch above the highest Aster-published Salvo version (1.0.4 → 1.0.5 on base v1.0.1). Bump h3/h3-quinn/salvo-http3 only if their src changed. If upstream itself ships a version Aster has already published (for example a Salvo 1.0.x or h3 0.0.9), take the next patch above both.
4. Completeness check: the assembled stack on the upstream tag must equal the previous integration line's tree apart from upstream's changes. `git cherry -v <integration-line> <branch>` no longer reliably prints `-` for every patch, because the old line carried pre-1.0 copies and split S3/S4 across two commits each, so their patch-ids differ; use tree equality instead.
5. Build and smoke-test the consumers (below), then tag (below).

Retiring a patch that upstream accepted:

1. Accept the upstream version; do not re-cherry-pick ours. Delete the redundant branch only after confirming the upstream API is a true superset. Before deleting, if the branch's commit is not an ancestor of the integration line (it usually is not), archive the original SHA as a tag so the graduated-patches table stays resolvable: `git tag archive/<branch> <sha> && git push origin archive/<branch>`.
2. Check whether consumers must change (upstream's API may differ in name, signature or feature gating). If the Aster transport (`aster-transport-salvo`), `aster-expose` or portal_desktop must change, write a migration note (`salvo_migrations/<date>-<patch>.md`, kept by Aster maintainers) with: which patch graduated and the upstream commit/PR that replaced it; our API and upstream's side by side; the exact edits each consumer needs; and the rev to bump to.
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
cargo check -p aster-transport-salvo -p aster-expose --features aster-expose/edge
# portal_desktop builds its own WebTransport path against web_transport_owned()
```

Validation used for releases: `cargo fmt` check; clippy (workspace, all features, all targets), with the existing `collapsible_if` (cookie extraction) and `redundant_clone` (TCP test) diagnostics allowed, since strict clippy still reports those unrelated warnings; `salvo_core` library tests with `quinn,rustls` (or `quinn ring server-handle`); the vendored crates tested in a temporary workspace; then Aster's transport suite, including the raw WebTransport real-network lifecycle/control tests.

## History

### 2026-10-11: receive-control release (current)

- Forgejo `main` and immutable tags `v1.0.0-aster.5` / `aster-salvo-2026-10-11` point at `35aa745dfca50c2e5a640c0d0c93153b033fa931`. Patch commit `9a38a8a9` (S9) fixes the pending-read STOP/receive-ID panic; the following version commit keeps registry versions immutable: all 22 Salvo crates 1.0.4, `h3-quinn` 0.0.12, `salvo-http3` 0.7.2; `h3` stays 0.0.9 and `h3-datagram` 0.0.3. The release was assembled in an isolated worktree, leaving the original developer checkout unchanged.
- Validation at this exact release: 362 `salvo_core` library tests passed with `quinn,rustls`, including all 18 QUIC tests. All-target clippy passed with only the two existing baseline diagnostics allowed; strict clippy still reports them. Aster's full transport suite passed against this pin, including all five raw WebTransport real-network lifecycle/control tests. Registry publication and downstream consumer pins remain the consuming workspace's release gate.
- This commit is now the tip of `upgrade/salvo-v1.0`.

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
