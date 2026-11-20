# R4-notify-diag: phase D (notifications, update notice, diagnostics export)

- **Role / model:** implementation, claude-sonnet-5.5.
- **Worktree:** `D:\GitHub\wow-sync.worktrees\R4-notify-diag`, on branch
  `run004/R4-notify-diag`, created from `main` after the R4-integrate
  command-surface merge (`93724ed`).
- **Rules:** [COMMON.md](COMMON.md) applies in full.
- **Parallel work:** R4-integrate continues in its own worktree (service,
  control, settings apply, rollback). The file sets are disjoint. You don't
  own `service.rs` or `control.rs`; R4-integrate does that wiring from your
  handover (below).

## Authority

- [ops.md](../../../design/ops.md):
  - §3 (notifications, `notify-rust =4.19.0`, D-24 P4-2);
  - §4 (update notice, `ureq =2.12.1` + `native-tls =0.2.18`, no env proxy,
    redirect policy, D-24 P4-1);
  - §5 (diagnostics export);
  - the §6 rows for `notice`, `update`, `diagnostics`, `notify`,
    `update_http`, `notices`, `updates`, `diag`;
  - the §7 rows T-27-*, T-28-*, T-33-*.
- [contracts.md](../../../design/contracts.md) §7 (the diagnostics boundary)
  and the §9.3 R4-notify-diag row (your file set).
- [ui/interaction.md](../../../design/ui/interaction.md) "Notifications",
  [ui/screens.md](../../../design/ui/screens.md) "About and updates", for
  wording only. You build no UI.
- D-24 P4-1, P4-2; D-25: Q-63 (native macOS notifications are required; an
  in-app-only fallback is not a pass), Q-85 (recurrence quiet period), Q-86
  (`needs_me`), and the other notification defaults.

## Files you own (exactly contracts.md §9.3)

- Core: `src\notice.rs`, `src\update.rs`, `src\diagnostics.rs`.
- Adapters: `src\notify.rs`, `src\update_http.rs`, `src\winproc.rs`,
  `src\localstate.rs`, `wow-sync-adapters\Cargo.toml`.
- Workspace `Cargo.lock`.
- Runtime: `src\notices.rs`, `src\updates.rs`, `src\diag.rs`.
- Your section of `tests\required-cases.toml`, between
  `# >>> section: R4-notify-diag` and `# <<< section: R4-notify-diag`.
- Evidence: `docs\runs\run-004\evidence\R4-notify-diag.md`.

Don't touch anything else. That includes any `lib.rs`, `service.rs`,
`control.rs`, `deny.toml`, the root `Cargo.toml` and the app crate. If
`cargo deny` rejects a license or advisory in the new crates' tree, stop
and report the exact finding. Don't edit `deny.toml`.

## Work

1. **Core planners**, pure and fully tested: the notification planner
   (batching, gaming deferral, recurrence quiet period, `needs_me`, bounded
   persisted ledger), `parse_latest` / `due`, and the diagnostics builder
   with redaction, the 256 KiB bound, `VersionClass`, and the folder-name
   and fingerprint options.
2. **Adapters:**
   - `Notifier` / `OsNotifier` (notify-rust), with `RecordingNotifier` in
     the testkit.
   - `ReleaseSource` / `GithubReleases` (ureq + native-tls, the exact
     headers, no env proxy, the redirect policy, the size cap), with
     `FakeReleases`.
   - The workspace arch guard (T-28-7).
   - Any `localstate.rs` / `winproc.rs` additions §6 requires.
3. **Runtime** `notices.rs`, `updates.rs`, `diag.rs`:
   - the scheduling, single flight and persist-before-request flows;
   - functions the service and control layer can call.
4. **Tests:** every T-27, T-28 and T-33 row that is not manual, with the
   A-38 controls the rows name (T-28-7 planted use, T-33-2 injection).
   Manual rows:
   - **T-27-11, a native notification:** run it on this Windows machine and
     record the result. macOS is NOT RUN here; it goes to R4-mac-kit.
   - **T-28-13, one live GET against GitHub Releases** (allowed by D-24
     P4-1, anonymous): run it once on Windows and record the actual result.

## Handover (required in your evidence)

The exact runtime functions R4-integrate must call, and where:

- the cycle-end notification step;
- the update check schedule and the `CheckUpdates` handler;
- the `ExportDiagnostics` handler.

Give each one's signature and the state it needs. R4-integrate (or the
orchestrator's next packet) wires them in. Also list the strings and states
R4-app shows: notification toggles, the About "Could not check" text, and
the export dialog.

## Gate and evidence

- Offline and native verify pass on the final tree.
- `git diff --name-only main..HEAD` lists only your files and evidence.
- The section check passes.
- No Windows-only test is listed in an offline case.
- **Evidence:** each T-row with its result (passed, failed, blocked or not
  run), the A-38 controls, the dependency additions with their E-06
  justification and `cargo deny` result, the manual rows with the actual
  environment, and the handover.
- **Commits:** prefix `run004(R4-notify-diag):`, with the Copilot trailer.
  Commit per work item. Don't push. Never use
  `git checkout --`/`restore`/`reset --hard`/`stash` on uncommitted work.
