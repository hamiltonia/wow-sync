# R3-dropbox-api evidence: Dropbox HTTP API research spike

Run 003, worker R3-dropbox-api (Design role, research only). Worktree
`D:\GitHub\wow-sync.worktrees\R3-dropbox-api`, branch `run003/R3-dropbox-api`.
Authority: packet [R3-dropbox-api](../packets/R3-dropbox-api.md),
[COMMON](../packets/COMMON.md), OWNER-DECISIONS D-08 and D-21. Links: SPEC
R-03, R-12, R-14, R-15, R-16, §7; ACCEPTANCE A-05, A-20, A-29, A-34; design
Q-7 and Q-23.

## What was produced

- `docs/design/dropbox-api.md`: compares A (the desktop-client folder), B
  (HTTP API v2) and C (hybrid). It contains a comparison table, findings for
  each packet question, a risk table (R1 to R10), the recommendation, owner
  questions DA-1 to DA-4 and a trial plan (T0 to T7) that needs approval
  first.
- One row in `docs/design/README.md`.

Recommendation: keep A for the first release, do not switch to B, and
consider an optional read-only hybrid C after v1 once the owner approves.

## Method and safety

- **PASS (scope kept), Windows 11 x64 dev PC.** Public web documentation
  only. The fetch tool and `Invoke-WebRequest` were used on public
  docs.dropboxapi.com pages, and `Invoke-RestMethod` on the public crates.io
  crate-metadata API.
- No Dropbox credentials, app registration or Dropbox API calls (D-08).
- Nothing under `D:\Dropbox` was read or written.
- No WoW access. No sub-agents.
- Repository inputs read: AGENTS.md, SPEC.md, ACCEPTANCE.md (A-05, A-20,
  A-29, A-34, real-provider layer), OWNER-DECISIONS (D-07, D-08, D-19,
  D-21), `docs/design/{README,publication,adapters-api,questions}.md`,
  run-001 S-dropbox evidence, run-003 R3-mem evidence, `deny.toml` and
  `Cargo.lock` (to check whether a TLS stack is already a dependency).

Commands run (besides reading files):

```text
Invoke-WebRequest https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/delete-v-2.md  (grep parent_rev)
Invoke-WebRequest https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/list-folder.md (grep args)
Invoke-RestMethod https://crates.io/api/v1/crates/<webpki-roots|ring|aws-lc-rs|rustls|ureq|dropbox-sdk|keyring|rustls-platform-verifier>
Select-String Cargo.lock for reqwest/rustls/hyper/ureq/native-tls/ring/... -> only `http` 1.5.0 and `tokio` 1.54.1 present
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"; cargo xtask verify --tier offline
```

## Results

| Item | Result | Note |
|---|---|---|
| Questions 1 to 7 of the packet answered with citations | PASS | dropbox-api.md §3 and §5 |
| Live behaviour (latency, idle cost, memory, upload proof, hydration) | NOT RUN | Needs credentials; D-08 forbids them. Listed as trial T2 to T6 |
| `content_hash` check against Dropbox's example | NOT RUN | Possible offline (T0); not product work in this packet |
| Licence check of candidate TLS crates against `deny.toml` | PASS (paper) | `webpki-roots` is CDLA-Permissive-2.0, which is not allowed. Others fit |
| `cargo xtask verify --tier offline` | PASS | Windows 11 x64 dev PC: fmt, clippy, test, required-cases, deny, markdown-links all PASS; overall exit 0 |

The change is documentation only, so no product test changes and no A-38
mutation control.

## Sources consulted (all fetched 2026-11-17)

1. https://developers.dropbox.com/oauth-guide (redirects to https://docs.dropboxapi.com/dropbox-api/docs/oauth)
2. https://developers.dropbox.com/detecting-changes-guide (redirects to https://docs.dropboxapi.com/dropbox-api/docs/detecting-changes)
3. https://www.dropbox.com/developers/reference/developer-guide (redirects to https://docs.dropboxapi.com/dropbox-api/docs/developer-resources/developer-guide)
4. https://www.dropbox.com/developers/reference/content-hash (redirects to https://docs.dropboxapi.com/dropbox-api/docs/technical-reference/content-hash)
5. https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/upload.md
6. https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/list-folder-longpoll.md
7. https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/list-folder.md
8. https://www.dropbox.com/developers/reference/data-ingress-guide (redirects to https://docs.dropboxapi.com/dropbox-api/docs/performance)
9. https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/upload-session-start.md
10. https://www.dropbox.com/developers/reference/tos
11. https://www.dropbox.com/developers/reference/branding-guide (redirects to https://docs.dropboxapi.com/dropbox-api/docs/developer-resources/branding-guide)
12. https://docs.dropboxapi.com/llms.txt
13. https://help.dropbox.com/organize/conflicted-copy (cited from run-001 S-dropbox, not re-fetched)
14. https://docs.dropboxapi.com/dropbox-api/api-reference/user-endpoints/files/delete-v-2.md
15. https://help.dropbox.com/installs/locate-dropbox-folder
16. https://community.dropbox.com/en/discussion/714008/dropbox-scoped-app-folder-visibility-of-uploaded-files-to-other-users and https://community.dropbox.com/en/discussion/679737/sharing-dropbox-application-folders (forum answers from Dropbox staff; secondary, found by web search)
17. https://help.dropbox.com/installs/dropbox-for-macos-support (found by web search)
18. https://learn.microsoft.com/en-us/windows/win32/api/wincred/ns-wincred-credentialw
19. https://github.com/dropbox/dropbox-sdk-dotnet/blob/main/dropbox-sdk-dotnet/Examples/OAuthPKCE/Program.cs (found by web search)
20. https://docs.rs/keyring/latest/keyring/
21. https://docs.rs/ureq/latest/ureq/
22. https://docs.rs/dropbox-sdk/latest/dropbox_sdk/
23. https://aws.github.io/aws-lc-rs/requirements/windows.html
24. https://crates.io/api/v1/crates/<name> for the eight crates listed above

## Limits and open issues

- Every performance, latency and idle-cost statement is **unmeasured**.
- The rule that app folders cannot be shared across accounts rests on forum
  answers (S16), not on the reference docs.
- Whether Credential Manager and Keychain isolate per application is
  unverified.
- Rate limits are unpublished by Dropbox.
- The production-review outcome cannot be known in advance.
- Owner questions DA-1 to DA-4 are open (dropbox-api.md §6). Any live trial
  needs a new owner decision extending D-08.
