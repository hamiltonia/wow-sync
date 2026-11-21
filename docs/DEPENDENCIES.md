# Direct third-party dependencies (E-06, RV-10)

Every direct third-party dependency of the workspace, with licence, maintenance
and need. Licence and repository come from each crate's own `Cargo.toml` as
recorded in the local registry (`cargo metadata --offline --locked`); no
network was used. Versions are the exact pins in `Cargo.toml`/`Cargo.lock`.

**Maintenance column.** The local registry has no activity data. "Established"
means a widely used crate with a public repository that we have no reason to
doubt; it was **not** checked online for this document. `cargo deny check`
(advisories, licences, bans, sources) is the enforced gate, see `deny.toml`
and [VERIFY.md](VERIFY.md). Transitive dependencies are covered by `cargo deny`
and are not listed.

| Crate | Pin | Licence | Repository | Used by | Need | Maintenance |
|---|---|---|---|---|---|---|
| serde | =1.0.229 | MIT OR Apache-2.0 | serde-rs/serde | core, runtime, xtask | Typed (de)serialization of records, state and reports | Established |
| serde_json | =1.0.154 | MIT OR Apache-2.0 | serde-rs/json | core, runtime, xtask | JSON records, update-check parsing, verify report | Established |
| sha2 | =0.10.9 | MIT OR Apache-2.0 | RustCrypto/hashes | core, adapters | SHA-256 content fingerprints (sync identity, integrity) | Established (RustCrypto) |
| getrandom | =0.3.4 | MIT OR Apache-2.0 | rust-random/getrandom | adapters | OS random bytes for unique temp and recovery names | Established |
| unicode-normalization | =0.1.24 | MIT/Apache-2.0 | unicode-rs/unicode-normalization | core | Normalizing names so case/Unicode variants cannot alias | Established (unicode-rs) |
| windows-sys | =0.59.0 | MIT OR Apache-2.0 | microsoft/windows-rs | adapters (Windows only) | Process observation, file and volume APIs | Microsoft-maintained |
| ureq | =2.12.1 | MIT OR Apache-2.0 | algesten/ureq | adapters | The single D-24 GitHub Releases request (only `update_http.rs` names it) | Established; 2.x line |
| native-tls | =0.2.18 | MIT OR Apache-2.0 | rust-native-tls/rust-native-tls | adapters | OS TLS stack (Schannel/Security.framework) for that request, no bundled roots | Established |
| notify-rust | =4.19.0 | MIT OR Apache-2.0 | hoodie/notify-rust | adapters | OS notifications (ops.md notices) | Established, small maintainer base |
| gpui | 0.2.2 (git, rev `e8bc16ac`) | Apache-2.0 | zed-industries/zed | app, proto | GUI framework (D-18); pinned to a Zed release commit | Actively developed; git pin, not on crates.io release cadence |
| gpui_platform | 0.1.0 (git, same rev) | Apache-2.0 | zed-industries/zed (no `repository` field in crate metadata) | app, proto | GPUI platform backends | Same as gpui |
| tray-icon | =0.26.1 | MIT OR Apache-2.0 | tauri-apps/tray-icon | app, proto | System tray icon and menu | Established (Tauri) |
| raw-window-handle | 0.6 | MIT OR Apache-2.0 OR Zlib | rust-windowing/raw-window-handle | app, proto | Window handle type needed to attach the tray to the GPUI window | Established |
| png | =0.18.1 | MIT OR Apache-2.0 | image-rs/image-png | app | Decoding the embedded tray icon (already in the lock via tray-icon) | Established (image-rs) |
| embed-resource | =3.0.12 | MIT | nabijaczleweli/rust-embed-resource | app (Windows build) | Embedding the Windows icon/manifest resource (already in the lock via GPUI) | Established, single maintainer |
| toml | 0.8 | MIT OR Apache-2.0 | toml-rs/toml | xtask | Reading `tests/required-cases.toml` | Established |

Notes:

- All licences are in the `deny.toml` allow list and compatible with the MIT
  project licence. `gpui`/`gpui_platform` are Apache-2.0 only.
- `windows-sys` is pinned to 0.59 for the adapters; other versions in the lock
  are transitive.
- `serde`/`serde_json` in `xtask` are declared as `"1"` and `toml` as `"0.8"`
  in `xtask/Cargo.toml`; `Cargo.lock` (and `--locked` in xtask and the release
  build) fixes the resolved versions.
- Adding a dependency needs an entry here (licence, maintenance, need) and a
  passing `cargo deny check` (E-06).
