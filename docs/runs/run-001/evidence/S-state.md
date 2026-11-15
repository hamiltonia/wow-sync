# S-state evidence: addon-owned saved state identification

Run 001, Phase 2, packet [S-state](../packets/S-state.md) (common rules:
[S-common](../packets/S-common.md)). Role: design. Requested model
claude-opus-5.5; actual runtime model is recorded by the orchestrator.

Checkout `D:\GitHub\wow-sync.worktrees\S-state`, branch `run001/S-state`,
based on `f2a1323`. No sub-agents. No Lua executed. WoW, Battle.net and
launchers were not started, stopped or touched. WoW data was only **read**
(D-14 paths); nothing was written under `D:\Games`. No Dropbox access.

## Question and answer

SPEC §7: "Safe identification of addon-only account/character saved state"
(R-07, R-04 uninstall, R-10 recovery). Can saved-variable files owned by an
addon be identified safely without executing Lua, and which cases are
unknown/shared?

**Answer: feasible**, confidence medium-high for the file-level rule on the
`_forever_` client. A file `SavedVariables\<Folder>.lua` is attributed to the
installed addon folder of the same name, and a non-executing scanner confirms
that every top-level name in it is declared by that folder's `.toc` for that
scope (account vs character) and by no other installed folder. On the dev PC
all 47 third-party `.lua` files met that rule (0 unknown, 0 shared, 0 orphan);
negative cases were produced on synthetic fixtures and on a perturbed copy
of the `.toc` inventory.

What would change it: evidence that the client writes a file under a name
other than the loading folder; that it writes non-literal data; client
`.toc` selection rules for Forever that differ from the union used here; or
real installs with global-name collisions (expected; they become **shared**).
Client write timing and `.bak` rotation are **not** answered here (needs
S-proc-style evidence).

## Environment

- Windows 11 x64 dev PC (Insider build 26300; not official support evidence).
- Rust 1.100.0 / cargo 1.100.0 (root `rust-toolchain.toml`; spike has no
  toolchain override and **no dependencies**).
- Data: `D:\Games\World of Warcraft\_forever_\WTF` and `...\Interface\AddOns`
  (D-14), read-only. 102 addon folders with `.toc`, 52 declaring saved
  variables; 1 account, 1 realm folder, 8 character folders (redacted).

## WTF layout (file names only; dev PC, redacted)

| Path (relative to `_forever_\WTF`) | Seen on dev PC | Owner / sync scope |
|---|---|---|
| `Config.wtf`, `gamecontrollerdb.txt`, `GamePadConfig_Default.json` | yes | client (machine) — excluded |
| `SavedVariables\Blizzard_*.lua` (+ `.lua.bak`) | 3 + 2 bak | client, machine-wide — excluded |
| `Account\<ACCOUNT>\` `bindings-cache.wtf`, `config-cache.wtf`/`.old`, `macros-cache.txt`/`.old`, `edit-mode-cache-account.*`, `flagged-cache-account.*`, `tts-cache-account.txt`, `chat-frontend-cache.txt`, `character-list-order.txt`, `cache.md5` | 13 files | client UI / server-sync caches — excluded |
| `Account\<ACCOUNT>\SavedVariables\<Addon>.lua` | 31 addon + 5 Blizzard (+3 Blizzard bak) | **addon account-wide state** (candidate) / Blizzard excluded |
| `Account\<ACCOUNT>\<REALM>\<CHARACTER>\AddOns.txt` | 8 | client per-character addon enable list — excluded (see decisions) |
| `...\<CHARACTER>\` `config-cache.*`, `layout-local.txt`, `chat-cache.txt`, `macros-cache.txt`, `click-bindings-cache.txt`, `edit-mode-cache-character.*`, `flagged-cache-character.*`, `tts-cache-character.txt`, `cache.md5` | 12 (1 character) | client — excluded |
| `...\<CHARACTER>\SavedVariables\<Addon>.lua` | 16 addon + 6 Blizzard | **addon per-character state** (candidate) / Blizzard excluded |

Observations: the realm directory level exists even though the WTF tree has a
single realm; 7 of 8 character folders contain only `AddOns.txt` (no saved
variables yet); no third-party `.lua.bak` was present (only some `Blizzard_*`
files had `.bak`); the files with `.old` siblings are client caches, not saved
variables. Public knowledge (not verified here): the client writes saved
variables at logout, `/reload` and normal exit, not continuously, and rotates
the previous copy to `.bak` for some files. **NOT RUN**: any timing/rotation
observation (requires running the client; out of packet scope).

## Classification rules (implemented in `spikes/state/src/classify.rs`)

Inputs: scope (Machine = `WTF\SavedVariables`, Account, Character), file stem
(without `.lua`/`.lua.bak`), scanner result, and the installed folder
inventory (union and intersection of `## SavedVariables` /
`## SavedVariablesPerCharacter` over every `.toc` in the folder). Folder
matching is case-insensitive (Windows/client behavior); a case difference is
flagged. Evaluated in order:

1. Stem starts `Blizzard_` → **blizzard** (excluded). If an installed folder
   has that name → unknown (`blizzard-name-installed`).
2. Machine scope and not `Blizzard_` → unknown (`third-party-machine-scope`).
3. Scanner error → unknown (`parse-error`); non-literal data inside a table
   (function/call/bare identifier value) → unknown (`non-literal`); a name
   assigned twice → unknown (`duplicate-names`).
4. No folder of that name:
   - names declared by some installed folder → unknown
     (`declared-by-other-folder`, e.g. renamed/split package);
   - otherwise → **orphan** (addon removed or never installed here).
5. Folder exists but declares nothing → unknown (`no-declaration`).
6. Any name not declared for this scope → unknown (`wrong-scope` if declared
   for the other scope, else `undeclared-names`).
7. Any held name also declared by another installed folder → **shared**.
8. Otherwise → **owned** (or **owned-empty** if no assignments).
   Flag `flavor-dependent` when a held name is declared by only some of the
   folder's `.toc` files.
9. `.lua.bak` is classified exactly like its base file (same owner).

Safely addon-owned = `owned`/`owned-empty`. Excluded = `blizzard` and all
non-`SavedVariables` WTF files. Needs diagnosis = `shared`, `orphan`, every
`unknown:*`.

## Scanner (`spikes/state/src/scanner.rs`)

A Lua 5.1 lexer (long brackets of any level, line/long comments, `"`/`'`
strings with escapes and `\`-newline, numbers incl. hex/exponent, operators,
UTF-8 BOM, CRLF) feeding a strict acceptor: top level must be
`Name = <nil|true|false|number|-number|string|{table}>` separated by optional
`;`. Tables are consumed with an explicit bracket stack (no recursion), and
anything that is not data inside a table is reported. Keywords, `local`,
calls, dotted names and non-ASCII bytes outside strings fail closed. It never
evaluates anything.

Is top-level-name scanning enough? **Yes for confirming which declared
globals a file holds**, because the client serializes only declared globals as
literal top-level assignments. It is **not** sufficient for anything below the
top level (e.g. per-profile or per-character keys inside an account-wide
table); splitting inside a table is out of scope and must not be attempted
(R-07: do not split a shared file inconsistently).

## Commands and results

From `D:\GitHub\wow-sync.worktrees\S-state\spikes\state` after
`$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"`:

| Command | Result |
|---|---|
| `cargo test --release` | **PASS** (Windows dev PC): 14 tests (fixtures: simple, long strings, comments, escaped quotes, nested/numeric, code in table, code/`local`/name at top level, unterminated string/long bracket, unbalanced/mismatched brackets, invalid byte, missing value, duplicates, comment-only/empty, BOM+CRLF, UTF-8; classification of every rule above; `.toc` parsing; flavor and case flags) |
| same, huge/deep test | **PASS**: generated 30.5 MB file scanned in ~150–330 ms (release); 200,000-level nested table scanned without stack overflow |
| `cargo clippy --release --all-targets -- -D warnings`; `cargo fmt --check` | **PASS** |
| `cargo run --release -q -- survey "<_forever_>\Interface\AddOns" "<_forever_>\WTF"` | **PASS** (dev PC, read-only) — [survey-devpc.tsv](S-state/survey-devpc.tsv) |
| same with a scratch `.toc`-only copy of the inventory (Prat-3.0 removed, VuhDoOptions renamed VuhDoOpts, synthetic folder declaring `XPerlConfig_Global` added) | **PASS** — [survey-perturbed.tsv](S-state/survey-perturbed.tsv) |
| same with an empty AddOns dir | **PASS** — [survey-empty-addons.tsv](S-state/survey-empty-addons.tsv) |

The scratch inventory lived in `spikes/state/target/scratch` (git-ignored,
`.toc` copies only, not committed).

### Aggregate dev-PC outcomes (real inventory)

| Scope | owned | owned-empty | shared | orphan | unknown | blizzard | client files |
|---|---|---|---|---|---|---|---|
| Machine | 0 | 0 | 0 | 0 | 0 | 3 (+2 bak) | 3 (root) |
| Account | 31 | 0 | 0 | 0 | 0 | 5 (+3 bak) | 13 |
| Character | 16 | 0 | 0 | 0 | 0 | 6 | 20 |

66 saved-variable files (1,030,022 bytes) scanned with zero scanner errors
and zero non-literal findings. 2 files flagged `flavor-dependent`
(Questie account/character): its base `Questie.toc` is an `## Interface: 00000`
stub that declares nothing, while all flavor `.toc` files (including
`Questie_Forever.toc`) declare the variables.

### Negative cases on real-shaped data

- Perturbed inventory: Prat-3.0 files (account + character) → `orphan` (2);
  VuhDoOptions files → `unknown:declared-by-other-folder` naming `VuhDoOpts`
  (2); XPerl account file → `shared` with the synthetic folder (1). All other
  files unchanged.
- Empty inventory: all 47 third-party files → `orphan`; Blizzard unchanged.

## Consequences for sync scope, uninstall and recovery

- **Sync scope (R-07):** only `owned`/`owned-empty` files at Account and
  Character scope, keyed by variant + account + (realm, character) + addon
  folder. Never Machine scope, `Blizzard_*`, `AddOns.txt`, caches or
  `Config.wtf`. A logical addon (SPEC §2 vocabulary "Addon") maps to one or more folders;
  its settings are the union of its folders' owned files (e.g. VuhDo +
  VuhDoOptions, DBM-Core + DBM-StatusBarTimers). Package grouping is the
  S-addon/design question, not decided here.
- **Shared:** do not sync either file automatically and do not split it;
  surface a diagnosis naming both folders. Removal of one addon must not
  delete the file of the other.
- **Orphan:** cannot be attributed to an installed addon. Never delete or
  publish automatically; it is the expected residue when an addon is removed
  outside the app, and the app's own uninstall recovery point may already hold
  it. Retaining it locally is what lets a reinstall find its settings.
- **Unknown:** pause settings for the affected addon scope only (R-07
  diagnosis "for the affected scope"), keep other addons working.
- **Uninstall (R-04):** removes the addon's own `owned` files at every
  account/character scope on that computer (plus their `.bak`), after a
  recovery point captures code + those files together; never `shared`,
  `orphan` or unknown files. Recovery restores both.
- **`.bak`:** treat as client-managed sidecar of its base file: capture with
  the base for recovery, delete with the base on uninstall, but do not publish
  as separate settings. Needs client-timing confirmation (below).

## Unknown/shared cases needing owner or design decisions

1. **Orphan handling** (owner/design): keep orphans indefinitely, offer GUI
   cleanup, or adopt them when a matching addon is (re)installed via sync?
   Recommendation: keep locally, never publish, show in diagnostics, adopt on
   reinstall.
2. **Global-name collisions → shared** (design): exclude both addons'
   colliding files from automatic sync with a visible diagnosis, or allow a
   per-addon owner choice? Not observed on the dev PC.
3. **Flavor `.toc` selection** (design, depends on S-forever): rules here use
   the union of all `.toc` files. The product should evaluate the `.toc` the
   Forever client actually loads (10 folders here ship a `_Forever.toc`;
   fallback order unverified). Using the union can only over-accept names
   declared for another flavor.
4. **`AddOns.txt` (per-character enable state)** (owner): R-07 excludes WoW's
   own files, but this file decides whether a synced addon is enabled on each
   character. Confirm it stays excluded (recommendation) or becomes a
   separately decided feature.
5. **Character identity** (design): character folders are keyed by
   account/realm/name; renames, transfers and different account folder names
   on another computer break the key. R-03/R-07 boundary mapping needs a
   design decision; never map by guess.
6. **Characters without saved variables yet** (design): 7 of 8 character
   folders hold only `AddOns.txt`. Whether the master's per-character files
   are created on a peer that has the folder but no file (or no folder) needs
   a rule.
7. **Client write timing / `.bak` rotation** (needs S-proc-style evidence):
   when files are written (logout, `/reload`, exit, crash), whether partial
   writes are visible, and when `.bak` rotates. Required for R-07
   play-session capture and "interrupted write" detection. **NOT RUN.**
8. **Inner-table scope** (design): addons that keep per-character data inside
   an account-wide table cannot be split by character without parsing values;
   R-07 says do not split — the whole file follows the account scope.

## Method limits and risks

- Only one dev-PC install with a clean, small WTF; real installs will have
  orphans, collisions and stale files. The negative cases are synthetic or
  perturbed-inventory, not observed in the wild.
- Ownership by file name relies on the documented client convention (file
  named after the folder that declared the variables); not independently
  verified by running the client (out of scope; would require writing data
  through the game).
- The scanner checks names only; it does not validate numeric/string syntax
  in depth (e.g. it accepts any alphanumeric run starting with a digit as a
  number). This affects only acceptance of malformed data, not attribution.
- Unknown client serialization of non-finite numbers (`inf`/`nan`) is untested;
  such values would currently fail closed as non-literal/unexpected.
- macOS: **BLOCKED** (no Mac). Path layout is expected to be the same under
  the `_forever_` folder, but case-sensitive filesystems may change the
  case-insensitive matching rule.

## Spike code and rerun

- `spikes/state/src/scanner.rs` — non-executing scanner.
- `spikes/state/src/toc.rs` — `.toc` declaration reader.
- `spikes/state/src/classify.rs` — pure classification rules.
- `spikes/state/src/main.rs` — `scan <file>...` and redacted
  `survey <AddOns> <WTF>` CLI.
- `spikes/state/tests/fixtures.rs`, `spikes/state/fixtures/*.lua` — synthetic
  fixtures only.

Rerun: `cd spikes\state; cargo test --release` and
`cargo run --release -q -- survey "<AddOns dir>" "<WTF dir>"`.
