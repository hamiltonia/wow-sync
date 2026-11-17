# Dropbox HTTP API versus the desktop-client folder (research spike)

Status: **research only** (run-003, packet
[R3-dropbox-api](../runs/run-003/packets/R3-dropbox-api.md); asked for by
OWNER-DECISIONS D-21). This was a paper study, as D-08 requires: no Dropbox
API credentials, no app registration and no API calls. Every claim about the
API comes from the public documentation listed in [Sources](#sources), fetched
on 2026-11-17. Anything marked **unmeasured** still needs the live trial in
§8, and that trial needs a separate owner approval.

This document does not change the slice. The slice follows SPEC R-03:
"Require the Dropbox desktop client". Option B below would need a SPEC
amendment.

## 1. Options compared

- **A: desktop-client folder (current).** The app reads and writes plain
  files under `<shared>\WoWSync\`, and the Dropbox desktop client replicates
  them ([publication.md](publication.md),
  [adapters-api.md](adapters-api.md) §5).
- **B: HTTP API v2 only.** The app talks to `api.dropboxapi.com`,
  `content.dropboxapi.com` and `notify.dropboxapi.com` itself, with an OAuth
  token per computer. The desktop client is not needed.
- **C: hybrid.** The desktop client stays the data plane, as in A. The API is
  added only for narrow jobs the folder cannot do: proof that a file reached
  the server, change wake-up, and fetching a cloud-only object without
  hydrating it.

## 2. Comparison table

| Topic | A: client folder | B: HTTP API | C: hybrid (A + read-mostly API) |
|---|---|---|---|
| Credentials | None held by the app. The desktop client owns sign-in | OAuth 2 code flow with PKCE and a refresh token per computer [S1] | Same token as B, but read-only scopes |
| App registration and review | None | Needed. Only the owner's account can link until more users are enabled. 500 users at most in development status. Linking freezes 2 weeks after the 50th user unless production status is granted [S3] | Same as B |
| Least-privilege access | The user picks any folder; the app touches only `WoWSync\` | "App folder" (`/Apps/<name>`) or "Full Dropbox" [S1][S3]. App folders cannot be shared across accounts [S16] | App folder only works if the shared location is the app folder |
| Change detection | Local filesystem notification or rescan (not yet designed) after the client delivers. Client delivery latency is unspecified | `list_folder` cursor plus `list_folder/longpoll`: blocks until a change, default timeout 30 s plus up to 90 s jitter, may return `backoff` [S2][S6] | Longpoll used only as a wake-up hint; reads still come from the local folder |
| Proof of upload | **Not possible** from local data (S-dropbox 2b) | Upload returns `rev`, `content_hash`, `server_modified` [S5] | `get_metadata` / `list_folder` gives server `rev` and `content_hash` for a file the client uploaded [S5][S7] |
| Proof of application elsewhere | No; needs a peer state record (R-12) | **No.** Still needs a peer state record | No |
| Compare-and-swap | None; a concurrent edit becomes a "conflicted copy" [S13] | `WriteMode.update` with a rev; `strict_conflict`; `delete_v2` `parent_rev` [S5][S14] | Not used (writes stay in the folder) |
| Integrity check | Re-hash local bytes (Q-7 cost) | `content_hash` (SHA-256 over 4 MiB blocks) checked server-side on upload and returned in metadata [S4][S5] | Could compare server `content_hash` without reading the file |
| Online-only / placeholders (Q-23) | Real risk; Windows cfapi, and on macOS File Provider [S17]; hydration untested (S-dropbox 2a BLOCKED) | Not applicable: the app downloads exactly what it needs | Cloud-only blobs fetched by API into staging instead of hydrating |
| Selective sync, client stopped, client paused | Folder can be excluded, stale or not uploading; hard to detect | Independent of the client | Detectable: server rev differs from local or local missing |
| Locked files / transient handles | Seen (S-dropbox 2c) | None locally. Server namespace lock gives `429 too_many_write_operations` [S8] | Same as A for writes |
| Moved Dropbox root | Must re-find the root (A-05); `info.json` gives the path [S15] | Paths are server paths; local root irrelevant | Same as A |
| Offline operation | Works; client queues | Needs an app-owned outbox and retry | Works as A; API features degrade to "unknown" |
| Large files | Client handles | `/files/upload` ≤150 MiB; larger needs upload sessions (≤150 MiB per request, 4 MiB multiples for concurrent, sessions live 7 days) [S5][S9] | Same as A |
| Rate limits | None from the app's view | Per-user, unpublished; 429 with `Retry-After` [S8] | Low call volume |
| Local disk | Full copy of all group blobs on every computer | Only what each computer needs, if the client is not also syncing the folder | Same as A |
| New dependencies | None | TLS + HTTP client, JSON, OAuth, keychain | Same as B |
| Test needs (A-34) | Desktop client on both OSes | Plus a credentialed test account and a provider fake for offline tests | Plus a credentialed test account |
| SPEC fit | R-03 as written | Needs R-03 amended | Fits R-03 (client still required) |

## 3. Findings by question

### 3.1 Auth

- **Registration.** Register one app in the Developer Console. It gets an
  app key (`client_id`) and secret. Users must never be told to register
  their own app [S1]. Use one app key per distinct app [S3].
- **Open-source distribution.** Dropbox names open-source and desktop apps
  as clients that cannot keep a secret. Such clients should use PKCE (`S256`)
  instead [S1]. The app key is then public, in source and binary. The
  "Allow public clients" toggle enables PKCE [S1].
- **Forks.** The guide says developers who wrap the platform need their own
  keys [S3]. A fork shipping the owner's key is outside the owner's control,
  and abuse of that key could get the app restricted for every user.
  **Risk R6.**
- **Refresh tokens.** Access tokens are short-lived. A background app
  requests `token_access_type=offline` and stores the refresh token. It then
  gets new access tokens from `/oauth2/token` with
  `grant_type=refresh_token` [S1]. Long-lived tokens are deprecated [S1].
- **Scopes and access type.** "App folder" limits the app to `/Apps/<name>`.
  "Full Dropbox" gives the whole account. The production review checks that
  the permission is not broader than needed [S1][S3].
  - App folders cannot be shared and cannot hold shared folders. This comes
    from a Dropbox staff answer on the community forum [S16], not the
    reference docs, so confidence is medium.
  - Consequence: an App-folder group works only if every computer uses
    **the same Dropbox account**. See owner question DA-1.
  - Minimum scopes for C: `files.metadata.read` (longpoll and metadata [S6])
    and `files.content.read` for the cloud-only fallback.
  - B also needs `files.content.write` [S5].
- **Token storage.**
  - Windows: Credential Manager generic credential (`CredWriteW`,
    `CREDENTIALW` [S18]).
  - macOS: Keychain.
  - Rust: the `keyring` crate's docs now advise applications to link
    `keyring-core` plus specific stores rather than `keyring` itself [S20].
  - Both stores protect per OS user, not per application. **To verify in the
    trial.**
  - Never put the token in a file, log, diagnostics export or telemetry
    (R-15). The developer terms also require reasonable security for tokens
    and HTTPS [S10 §2].
- **First-run UX for B and C.**
  1. The user clicks **Connect Dropbox**.
  2. The system browser opens the `dropbox.com/oauth2/authorize` URL [S1].
     The user signs in only on dropbox.com; the terms forbid collecting
     credentials in the app [S10 §2.6].
  3. Dropbox redirects to a **registered, exact** loopback URI. Dropbox's own
     .NET PKCE sample uses a fixed `http://127.0.0.1:<port>/authorize` and
     tells you to register it [S19]. A fixed port can collide. The fallback
     is no `redirect_uri`: dropbox.com shows a code to paste into the app
     [S1].
  4. The app checks that the account matches the group's account (DA-1).
     The account id is a stable identifier and must not leave the computer
     (R-15).

### 3.2 Change detection

- **API.** Keep a cursor from `list_folder`/`list_folder/continue`. Block on
  `list_folder/longpoll`, then call `continue` [S2].
  - Longpoll needs no auth header, only the cursor. It returns
    `changes: true/false` and may return `backoff` seconds [S6].
  - An expired cursor gives `409 reset`, which means a full re-list [S2].
  - Webhooks need a server, so they do not fit a serverless desktop app [S2].
- **Latency.** Longpoll returns when a change is detected [S2][S6]. The real
  server-to-client latency is **unmeasured**. With the folder, latency is the
  client's delivery time plus local notification, also unmeasured. R-14
  excludes Dropbox delivery time from the 10 s reaction gate either way.
- **Idle cost.** Longpoll keeps one HTTPS request open per computer and
  reconnects at least every 30 s plus jitter [S6]. That costs periodic
  wake-ups and a TLS session.
  - Watching the local filesystem uses no network.
  - Both are **unmeasured**. Battery matters for the MacBook (D-07).

### 3.3 Consistency

- **rev** is a unique revision id. **content_hash** is SHA-256 over SHA-256
  hashes of 4 MiB blocks; it is stable and documented [S4][S5].
  - An upload may carry `content_hash`; a mismatch is rejected
    (`content_hash_mismatch`) [S5].
- **Conditional writes (compare-and-swap).**
  - `mode: update(rev)` overwrites only if the rev matches [S5]. With
    `autorename: false` a mismatch is a conflict error; with `autorename` it
    becomes a "conflicted copy" name.
  - `strict_conflict: true` also conflicts when the file was deleted, or has
    identical content [S5].
  - `delete_v2` takes `parent_rev`, for files only [S14].
  - Upload-session finish takes the same commit info, so this also works for
    large files [S9].
- **Is this stronger than the folder?** Partly.
  - **Yes:** upload is provable. A computer can show "uploaded to Dropbox"
    (server rev and hash match), which S-dropbox showed is impossible from
    local file state.
  - **Yes:** compare-and-swap makes single-winner records possible (for
    example a later settings-master reassignment or a rejection record),
    without relying on conflicted copies.
  - **No:** the current layout avoids mutable shared files on purpose (one
    writer per member directory; immutable, content-addressed blobs), so
    compare-and-swap is not needed for addon code today.
  - **No:** proof that a peer *applied* data still needs that peer's own
    state record. SPEC R-12 and A-20 are unchanged. "Uploaded" is at most a
    new sub-state between published and available.
  - SPEC §6 rules out a Dropbox lock that excludes other computers. The API
    has file locks (`file_lock_info` in metadata [S5]); this design
    recommends not using them.

### 3.4 Problems the API removes

| Problem | B | C |
|---|---|---|
| Online-only / placeholder files, hydration (Q-23) | Removed | Mitigated: fetch the object by API instead of hydrating |
| Selective sync excluding the folder | Removed | Detectable (server has files the local folder lacks) |
| Client not running, paused or signed out | Removed | Detectable (server rev not advancing for local writes) |
| Transient locks by the client (S-dropbox 2c) | Removed locally | Not removed |
| Moved Dropbox root | Removed | Not removed (`info.json` helps [S15]) |
| macOS File Provider differences [S17] | Removed | Partly (reads of cloud-only objects) |
| Full local copy of every blob on every computer | Removed, unless the user's client also syncs the app folder (it does by default) | Not removed |

### 3.5 Problems the API adds

- **Rate limits.**
  - Limits are per linked user, unpublished, with HTTP 429 and
    `Retry-After`. Retries also count against the limit [S8].
  - Parallel writes to one namespace give
    `429 too_many_write_operations`. The fix is upload sessions with
    `finish_batch` (up to 1,000 entries) [S8].
  - A first publish of a ~3.3 GB install (publication.md §5) means many
    thousands of files and therefore needs batching. The exact file count is
    **unmeasured**.
- **Data transport limit.** Some Dropbox Business plans have a monthly cap
  on upload calls [S5][S8].
- **Large and chunked uploads.**
  - `/files/upload` handles up to 150 MiB. Above that, use sessions: at most
    150 MiB per request, a 7-day session lifetime, and 4 MiB multiples for
    concurrent sessions [S5][S9].
  - Per-file content-addressed blobs are mostly small, so sessions matter
    mainly for batching.
- **Offline and network failure.** B needs an app-owned durable outbox,
  backoff, resumable sessions and "waiting for network" status. These are
  new state, tests and recovery paths. A gets this from the client.
- **App review and production status.**
  - Development status allows the owner plus enabled development users, up
    to 500.
  - The production review starts only after 50 linked users. Linking
    freezes 2 weeks after the 50th user unless production status is granted.
  - Dropbox may deny production at its sole discretion [S3][S10 §1.1].
  - The app name must not contain "Dropbox" or start with "drop" [S11].
  - Production requires a privacy policy [S3][S10 §3].
  - Public release would therefore depend on Dropbox's approval. That is
    **risk R1**.
- **Terms of service.**
  - The developer terms bind the developer who registers the app, here the
    owner. They require reasonable security for tokens and user data, HTTPS,
    breach reporting and a privacy policy, and they let Dropbox suspend
    access [S10].
  - Nothing found forbids open-source clients. PKCE is the documented path
    for them [S1].
  - Dropbox does not allow file-sharing-network apps or apps that promote IP
    infringement [S3]. Syncing a user's own installed addons between their
    own computers is neither, but a reviewer must accept that (R1).
- **Credential handling (R-15).** A token is a bearer secret for the user's
  Dropbox: the app folder with App-folder access, or everything with Full
  Dropbox. This needs:
  - storage only in the OS keychain;
  - redaction in diagnostics and support exports;
  - a revoke action on disconnect or uninstall (`/2/auth/token/revoke` [S1]);
  - disclosure in the privacy policy.
  - R-15 also says new credential access needs separate approval.
- **Security review surface.** New code to review:
  - the TLS stack and certificate roots;
  - the OAuth flow (PKCE verifier, `state`, the loopback listener bound to
    127.0.0.1 only, single use);
  - URL launching;
  - parsing server JSON as untrusted input;
  - token lifecycle and revocation;
  - dependency supply chain (§3.6).

### 3.6 Costs

- **Memory (A-29).** The current app idles at 11.6 to 16.1 MiB private
  working set ([R3-mem](../runs/run-003/evidence/R3-mem.md)), against a
  100 MiB gate. There is headroom, but the cost of a TLS/HTTP stack plus an
  idle longpoll connection is **unmeasured** and must be measured with the
  same harness (trial T4).
- **Dependencies and binary size.**
  - `Cargo.lock` has no TLS stack today.
  - Candidates: `ureq` (blocking, rustls or native-tls) [S21] with `rustls`;
    or `dropbox-sdk` 0.21 [S22]. `dropbox-sdk` is generated from the
    official spec, but its README says it is "not yet official" with no
    formal support. Its default client is `ureq` plus `rustls`, and per-
    namespace feature flags keep it small.
  - Licence check against `deny.toml` (crates.io data, 2026-11-17):
    - `ring` is Apache-2.0 AND ISC.
    - `rustls` is Apache-2.0/ISC/MIT.
    - `aws-lc-rs` is ISC/Apache-2.0, but on Windows it needs a C compiler and
      NASM, and clang-cl on ARM64 [S23]. That adds four-target build risk.
    - `webpki-roots` is **CDLA-Permissive-2.0**, which is not on the
      allowlist. Use OS roots (`rustls-platform-verifier`, MIT/Apache-2.0)
      or native-tls instead, or ask to widen the allowlist.
  - Binary size impact is **unmeasured**.
- **A-34 multi-computer tests.**
  - B and C need a real Dropbox test account token on each test computer.
    That is new credential access, and the real-provider layer says
    "owner-supplied access is a prerequisite".
  - CI would need a secret, or the API tests stay local-only.
  - B also needs a faithful server fake (cursor reset, 429, rev conflicts,
    session expiry) for offline tests.
  - A-34 still needs the desktop client under A and C.
- **Provider abstraction.** Core already talks to the provider through ports
  expressed as objects and records ([adapters-api.md](adapters-api.md) §9):
  `stat_object`/`hash_object`, `put_object`, `write_record`, `MemberObs`
  (`list` + `read`), and `stage_folder` reading blobs. A second adapter can
  implement these over the API without changing core policy (E-08, A-43).
  Differences:
  - `CloudOnly` disappears, but `NotAvailableYet` stays (404 before a peer's
    upload lands, network down).
  - `VerifiedOk` would rest on the server's `content_hash`, not on re-hashing
    local bytes. Tree records would need the Dropbox `content_hash` per file
    (computable locally [S4]), or blobs downloaded and re-hashed.
  - `write_atomic` becomes `upload` with `add` (blobs) or `update(rev)`
    (own records).
  - Listing becomes cursor-based and needs a local metadata cache.
- **Migration of an existing group.**
  - The layout can be identical on both sides. An A group inside
    `/Apps/<name>/WoWSync/` is readable by B and C unchanged.
  - A group elsewhere must be moved into the app folder (users may move
    files into it [S3]) or re-published. Content addressing makes
    re-publishing idempotent.
  - Mixed groups (some computers A, some B) work in one account, because both
    see the same files; A computers only lag by client delivery.
  - Legacy data under `Gaming Sync` (D-08) is out of reach of an App-folder
    token by construction, which is good.

## 4. Risks

| # | Risk | Applies to | Effect | Mitigation |
|---|---|---|---|---|
| R1 | Production review denied, delayed or later revoked; 50-user freeze | B, C | Public release blocked, or sync broken for all users | Keep A as the complete product; C optional and degradable to A |
| R2 | Groups that span Dropbox accounts cannot use an App folder | B, C | Would need Full Dropbox (broader permission, harder review, R-15) | Ask DA-1; limit C to single-account groups |
| R3 | Token theft gives access to the user's Dropbox data | B, C | Privacy breach (R-15) | OS keychain, App folder, read-only scopes for C, revoke on disconnect, redaction tests |
| R4 | Unpublished rate limits during first publish | B | Long or stalled first publish | Upload sessions with `finish_batch`; honour `Retry-After`; resumable |
| R5 | TLS stack licence and build issues on four targets | B, C | cargo-deny failure, ARM64 build failure | OS roots or native-tls; prefer `ring`; prove in CI before use |
| R6 | Public app key reused by forks or abusers | B, C | App restricted for everyone | Document that forks register their own key [S3]; monitor the console |
| R7 | Idle longpoll cost (wake-ups, memory) | B, C | A-29 / G-04 regressions, battery | Measure (T4); C can use longpoll only while work is pending |
| R8 | Two sources of truth (server metadata vs local folder) in C | C | Confusing status | API results are hints and proofs only; decisions still read verified bytes |
| R9 | Offline outbox complexity | B | New crash-recovery surface | Avoided by C |
| R10 | Placeholder and hydration behaviour stays unknown | A | Q-23 stays open | Owner check (D-19); C's API fetch as a fallback |

## 5. Recommendation

**Keep A as the data plane for the first release. Do not switch to B.
Consider a narrow, optional hybrid C after v1, behind owner approval.**

Why not B:

- It adds an external gate on public release (production review and the
  50-user freeze), a held credential per computer, a TLS stack, an offline
  outbox and a credentialed test layer.
- It still cannot prove application elsewhere (R-12).
- SPEC R-03 requires the desktop client.

What C would be:

- App-folder access, scopes `files.metadata.read` and `files.content.read`
  only, and no API writes.
- It would add three things the folder cannot do:
  1. an **"uploaded to Dropbox"** status from the server rev and
     `content_hash`;
  2. a **longpoll wake-up**;
  3. **fetching cloud-only blobs** into staging without hydration (Q-23).
- When the API is unavailable or not connected, everything falls back to A
  behaviour.
- It requires the shared location to be the app folder (DA-3) and
  single-account groups (DA-1).

Design implication for now (no code change): keep the provider ports
object-shaped, as they are today, so B or C is a second adapter, not a core
change.

## 6. Owner questions

| # | Question | Default if unanswered |
|---|---|---|
| DA-1 | Are all computers in one group always signed into the **same** Dropbox account? | Yes for C. Multi-account groups stay A-only |
| DA-2 | Would you register a Dropbox app under your account and accept the developer terms (you become the "Developer"; production review is needed before more than a few users) [S3][S10]? | No; C stays deferred |
| DA-3 | If C is adopted, may the shared location be fixed to the app folder (`Dropbox/Apps/<name>`)? | Not decided; C is not built |
| DA-4 | Approve a later live trial (§8) as a separate decision extending D-08? | Not approved; no API calls |

## 7. What would change this recommendation

- The owner's Q-23 check shows that online-only files make the folder
  unworkable. That makes C's API fetch more valuable, possibly needed.
- Groups that span accounts are required. Then Full Dropbox is needed,
  which weakens C.
- Trial T4 shows longpoll idle cost is negligible and T3 shows upload proof
  is reliable. That strengthens C.
- Dropbox refuses or would likely refuse production status. Then A only.

## 8. Safe trial plan for a later run (needs approval first)

Approvals needed before any step that touches the network:

- **O1:** the owner registers a development-status app in their own account
  (App folder, scopes above, "Allow public clients" on, one registered
  loopback redirect).
- **O2:** permission for agents to run the OAuth flow for the owner's account
  only. The token is stored only in the dev PC's Credential Manager; never in
  the repo, logs, evidence or CI.
- **O3:** data scope is only `/Apps/<name>/` with synthetic data (an
  extension of D-08). The local copy may be read only through that path.
- **O4:** a budget, and permission to add a TLS/HTTP dependency on a spike
  branch (spike code only, not product).

Steps, each with PASS/FAIL/BLOCKED evidence:

| Step | What | Network | Pass condition |
|---|---|---|---|
| T0 | Implement `content_hash` and check it against the published example (9,711,423 bytes → `485291fa…35e0`) [S4] | No; can run now | Exact match |
| T1 | Build a minimal client (PKCE, refresh, `get_metadata`, `list_folder`, `longpoll`, `download`) with OS roots; run `cargo deny` | No | Builds for 4 targets in CI; licences pass |
| T2 | PKCE sign-in through the loopback; store the refresh token in Credential Manager; refresh; at the end revoke the token and confirm `invalid_access_token` | Yes | Token never on disk or in logs (grep the evidence) |
| T3 | Write synthetic files through the desktop folder; poll the server metadata; record the time to a matching `content_hash`. Repeat with the client paused | Yes | Hash matches; paused client is detectable |
| T4 | Idle longpoll for 1 h with the R3-mem harness | Yes | Report the private-WS delta and wake-ups; stays within A-29 |
| T5 | The owner marks one synthetic file online-only (Q-23 steps). Fetch it by API into staging, verify the hash, confirm the local placeholder was not hydrated | Yes | Hash OK; attributes unchanged |
| T6 | Negative cases: revoked token, network off, cursor reset (re-list) | Yes | Status "unknown/not connected", never "absent"; no deletion inferred |
| T7 | Optional, only if B is still under consideration: `update(rev)` with a stale rev on a synthetic file | Yes | Conflict error, no write |

**Stop conditions:**

- any request outside the app folder;
- any 429 (stop, do not retry in a loop);
- a token seen in any output;
- any read of a path outside the approved scope.

## Sources

All fetched 2026-11-17. "Docs" means docs.dropboxapi.com, where several old
`dropbox.com/developers/...` URLs now redirect.

| # | Source |
|---|---|
| S1 | OAuth guide: https://developers.dropbox.com/oauth-guide (→ https://docs.dropboxapi.com/dropbox-api/docs/oauth) |
| S2 | Detecting changes: https://developers.dropbox.com/detecting-changes-guide (→ https://docs.dropboxapi.com/dropbox-api/docs/detecting-changes) |
| S3 | Developer guide (permissions, production approval, disallowed apps): https://www.dropbox.com/developers/reference/developer-guide (→ https://docs.dropboxapi.com/dropbox-api/docs/developer-resources/developer-guide) |
| S4 | Content hash: https://www.dropbox.com/developers/reference/content-hash (→ https://docs.dropboxapi.com/dropbox-api/docs/technical-reference/content-hash) |
| S5 | `files/upload` (WriteMode, `strict_conflict`, `content_hash`, 150 MiB, errors): https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/upload.md |
| S6 | `files/list_folder/longpoll`: https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/list-folder-longpoll.md |
| S7 | `files/list_folder`: https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/list-folder.md |
| S8 | Performance (rate limits, namespace locks, batch upload): https://www.dropbox.com/developers/reference/data-ingress-guide (→ https://docs.dropboxapi.com/dropbox-api/docs/performance) |
| S9 | `files/upload_session/start`: https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/upload-session-start.md |
| S10 | Developer terms (effective 2025-03-01): https://www.dropbox.com/developers/reference/tos |
| S11 | Developer branding guide: https://www.dropbox.com/developers/reference/branding-guide (→ https://docs.dropboxapi.com/dropbox-api/docs/developer-resources/branding-guide) |
| S12 | Docs index: https://docs.dropboxapi.com/llms.txt |
| S13 | Conflicted copies (via run-001 S-dropbox): https://help.dropbox.com/organize/conflicted-copy |
| S14 | `files/delete_v2` (`parent_rev`): https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/delete-v-2.md |
| S15 | Locating the Dropbox folder, `info.json`: https://help.dropbox.com/installs/locate-dropbox-folder |
| S16 | Community forum, Dropbox staff answer on app folders and shared folders (secondary source): https://community.dropbox.com/en/discussion/714008/dropbox-scoped-app-folder-visibility-of-uploaded-files-to-other-users and https://community.dropbox.com/en/discussion/679737/sharing-dropbox-application-folders |
| S17 | Dropbox for macOS on File Provider: https://help.dropbox.com/installs/dropbox-for-macos-support |
| S18 | Windows `CREDENTIALW`: https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw |
| S19 | Dropbox .NET SDK PKCE sample (fixed loopback redirect): https://github.com/dropbox/dropbox-sdk-dotnet/blob/main/dropbox-sdk-dotnet/Examples/OAuthPKCE/Program.cs |
| S20 | `keyring` crate: https://docs.rs/keyring/latest/keyring/ |
| S21 | `ureq` crate: https://docs.rs/ureq/latest/ureq/ |
| S22 | `dropbox-sdk` crate: https://docs.rs/dropbox-sdk/latest/dropbox_sdk/ |
| S23 | `aws-lc-rs` Windows build requirements: https://aws.github.io/aws-lc-rs/requirements/windows.html |
| S24 | crates.io API (licences and versions of `webpki-roots`, `ring`, `aws-lc-rs`, `rustls`, `ureq`, `dropbox-sdk`, `keyring`, `rustls-platform-verifier`): https://crates.io/api/v1/crates/<name> |
