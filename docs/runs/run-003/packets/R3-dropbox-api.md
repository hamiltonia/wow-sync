# R3-dropbox-api (Design spike, opus-5.5)

Read [COMMON.md](COMMON.md) first. Authority also includes **D-21**, which
asks for this spike, and **D-08**: no Dropbox API credentials and no API
calls. This is research only.

**Inputs:**
- SPEC, especially:
  - the provider requirements;
  - R-14 (memory);
  - R-15 (privacy);
  - R-16 (distribution).
- `docs/design/` (the provider abstraction and how the Dropbox folder is
  used today).
- Run-001 evidence for S-dropbox (folder behaviour, online-only files, Q-23).

**Owned paths:**
- `docs/design/dropbox-api.md` (new).
- `docs/runs/run-003/evidence/R3-dropbox-api.md`.

Use only public Dropbox documentation and well-known crates' docs. Cite URLs.

## Questions to answer

Compare **A**, the desktop-client folder used today, with **B**, the Dropbox
HTTP API (v2), and **C**, a hybrid of the two.

1. **Auth:**
   - app registration and app key distribution for an open-source desktop
     app;
   - OAuth 2 PKCE without a secret;
   - refresh tokens;
   - scoped "App folder" versus full Dropbox access;
   - where the token is stored (Windows Credential Manager, macOS
     Keychain);
   - the first-run UX.
2. **Change detection:**
   - `list_folder/longpoll` and cursors versus filesystem watching;
   - latency, and battery or idle cost.
3. **Consistency:**
   - `rev` and content hash, and conditional upload (`update` mode with a
     rev) as compare-and-swap;
   - conflict behaviour;
   - whether the API gives stronger publish/ack semantics than the client
     folder. SPEC says publication is not proof of application.
4. **Problems the API removes:**
   - online-only and placeholder files (Q-23);
   - selective sync;
   - client-not-running;
   - locked files;
   - path relocation.
5. **Problems the API adds:**
   - rate limits and quotas;
   - large or chunked uploads (addon packages are up to about 3.3 GB per
     install);
   - offline operation;
   - network failure and retry;
   - Dropbox app review and production-status limits (user caps for
     development apps);
   - ToS for open-source clients;
   - credential handling under R-15;
   - the security review surface.
6. **Costs:**
   - memory and binary impact against A-29 (a TLS/HTTP stack in Rust);
   - effect on A-34 multi-computer tests;
   - the effect on the existing provider abstraction, and the migration
     path for an existing group.
7. **Recommendation** (keep A, switch to B, or hybrid C), including:
   - what owner approvals a live trial would need (credentials, app
     registration);
   - a small, safe trial plan for a later run.

## Done

- `dropbox-api.md` has a comparison table, risks and the recommendation.
- Add a row for it to `docs/design/README.md`. That file is owned for this
  one row only.
- The evidence lists the sources consulted.
- `cargo xtask verify --tier offline` passes. Commit.
