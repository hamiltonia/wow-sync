# Domain rules

Part of the [slice design](README.md). All rules here are implemented once, in
`wow-sync-core` ([crates.md](crates.md)). Inputs are bytes and observations
supplied by adapters; nothing is executed (R-11).

## 1. Computer, group and enrollment identity (R-03)

| Identity | Form | Created by | Notes |
|---|---|---|---|
| `ComputerId` | 32 lowercase hex chars (128 random bits) | shell, once per local app-state directory, via `adapters::sysinfo::random_id_hex` | Stored in local state `computer.json`. Not derived from host name, user name, SID or hardware (privacy, R-15) |
| display name | free text ≤ 64 chars | user | Label only; never identity or authorization |
| `GroupId` | 32 hex | shell, when the user creates a group | One group = one variant product code (§2). A shared location may hold several groups |
| `EnrollmentId` | 32 hex | shell, each time this computer joins a group | Member records and state files carry it. Retirement/re-enrollment (later run) keys on `(ComputerId, EnrollmentId)` |
| `OpId` | 32 hex | shell, per apply operation | Names staging, journal and recovery directories |

Enrollment steps (GUI in R2-slice; rules in core):

1. Shared location chosen by the user; must be an existing directory. The app
   uses `<shared>\WoWSync\` beneath it ([publication.md](publication.md) §1).
   For tests it is a temp dir; for provider tests exactly
   `D:\Dropbox\Gaming Sync\WoW Sync Test\<run-subdir>` (D-08).
2. Install root chosen by the user; candidates = immediate subdirectories whose
   names match `^_[a-z0-9_]+_$` (case-insensitive) and contain `.flavor.info`.
3. Variant validated (§2). `Refused` → no enrollment, shown with the reason.
   `Unfamiliar` → GUI shows product code, folder, build version and requires
   an explicit confirmation checkbox (A-46). The display name cannot change
   any of these values.
4. Create a new group (records the variant product code) or join an existing
   group in the shared location. Join is refused unless the group's product
   code equals the validated local product code byte for byte.
5. Consent text shown before writing anything (R-03), minimum content:
   "This computer will share its installed addons for <label> (<product>) with
   other computers in this group, and will install newer versions found on
   them. Folders it replaces are kept on this computer for recovery under
   `<variant>\.wowsync\recovery`. Addons missing here are installed, never
   treated as removed. Settings (WTF) are not synchronized in this version."
6. Member record written, then the first state is published (§ apply.md).

## 2. Variant identity and validation (R-02)

Observations (adapters read bytes; core parses):

| Input | Path | Parsed as |
|---|---|---|
| folder name | `<root>\<folder>` | string, e.g. `_forever_` |
| `.flavor.info` | `<root>\<folder>\.flavor.info` | BPSV (below); exactly one column, first data row = product code |
| `.build.info` | `<root>\.build.info` | BPSV; columns `Product` and `Active` required, `Version` optional |
| AddOns directory | `<root>\<folder>\Interface\AddOns` | must exist as a plain directory (no reparse point) |

`.product.db` (binary) is **not parsed** in the slice; the S-forever finding
(REPORT §3: `wowf` consistent across `.build.info`, `.product.db`,
`.flavor.info`) is matched by the first two (Q-11).

**BPSV format** (`core::bpsv`): UTF-8 (BOM allowed), lines split on `\n`
(strip trailing `\r`); lines starting `##` are comments; empty lines ignored.
First non-comment line is the header: fields separated by `|`, each
`Name!TYPE:size`; the name is the part before `!`. Every following line is a
row with exactly as many `|`-separated fields as the header, else
`Unparseable`. Max 1 MiB input. The real header names of `.flavor.info` are
not assumed: only "exactly one column" is required.

**Product code** syntax: `^[a-z0-9_]{1,32}$` after trimming; otherwise
`InvalidProductCode`.

**Validation** `validate_variant(obs) -> VariantVerdict`, first match wins:

| # | Condition | Verdict |
|---|---|---|
| 1 | `.flavor.info` missing / unreadable | `Refused(FlavorMissing / FlavorUnreadable)` |
| 2 | not one column or no data row | `Refused(FlavorUnparseable)` |
| 3 | product code syntax invalid | `Refused(InvalidProductCode)` |
| 4 | `.build.info` missing / unreadable / unparseable / lacks `Product` or `Active` | `Refused(BuildInfo…)` |
| 5 | no row with `Product == code` | `Refused(ProductNotInBuildInfo)` (disagreement) |
| 6 | more than one such row | `Refused(ProductAmbiguousInBuildInfo)` |
| 7 | that row's `Active` is not `1` | `Refused(ProductInactive)` |
| 8 | code is a known profile and folder name (case-insensitive) differs from the profile's folder | `Refused(FolderMismatch)` (disagreement) |
| 9 | AddOns directory absent or not a plain directory | `Refused(AddOnsMissing)` |
| 10 | code is a known profile | `Recognized(identity)` |
| 11 | otherwise | `Unfamiliar(identity)`: blocked until `confirm_variant(.., user_confirmed = true)` |

Known profiles (the only one in the slice; everything else is unfamiliar,
Q-12):

| product | folder | label | `.toc` suffixes, in order | evidence |
|---|---|---|---|---|
| `wowf` | `_forever_` | Forever | `Forever`, `Camelot` | S-forever (dev PC, owner review pending); toc order **unverified** |

Never label `wowf` as Classic Era (AGENTS.md). Interface numbers (e.g. 16001)
are **not** used for identity (S-addon: plain tocs carry unions).

The group record stores the product code; every record read from the provider
folder whose variant product differs from the group's is rejected as invalid
(never merged). The variant key for all data is `(GroupId, product)`.

## 3. `.toc` parsing and the single selection rule

One parser and one selection function for the product (P3-review M4, L5).

**Candidates** for folder `F`: files directly in `F` whose name, compared
ASCII-case-insensitively, is `F.toc`, `F_<S>.toc` or `F-<S>.toc` where `<S>`
is `[A-Za-z0-9]+`. Other `.toc` names are ignored for selection and listed in
diagnostics.

**Parse** (`parse_toc`): ≤ 256 KiB else `TooLarge`; strip UTF-8 BOM; decode
UTF-8, invalid bytes → `NotUtf8` error (fail closed, never "empty"). Only lines
starting `##` are read: `## Key: Value` (key and value trimmed; key compared
case-insensitively). Recorded keys:

| Key | Field |
|---|---|
| `Interface` | `interface: Vec<u32>` (comma-separated; non-numeric entries ignored) |
| `Version` | `version: Option<String>` |
| `Title` | `title: Option<String>` (display only; color codes kept as text) |
| `Dependencies`, `RequiredDeps`, any key starting `Dep` | `hard_deps: Vec<String>` (comma-separated, trimmed, non-empty) |
| `X-Curse-Project-ID`, `X-Wago-ID`, `X-WoWI-ID` | `ids` |

Everything else (including `SavedVariables`, file lists, Lua/XML) is ignored
in the slice. A candidate that cannot be read or parsed makes the selection an
`Error` (not empty).

**Select** `select_toc(folder, candidates, profile) -> TocSelection`:

1. For each suffix in `profile.toc_suffixes` order, collect candidates with
   that suffix (either separator, case-insensitive).
2. If candidates exist for one or more listed suffixes: parse them all. If
   every parsed one has identical `version`, `hard_deps` (as sets) and `ids`,
   select the one with the first listed suffix. If they differ →
   `Ambiguous { files }`.
3. Else, if `F.toc` exists → select it.
4. Else (only other suffixes, e.g. only `_Vanilla`) → `None { reason:
   "no toc for this variant" }`.

For an unfamiliar variant the suffix list is empty, so only the plain toc is
used. The `wowf` order and the treatment of `_Camelot` are the stated
uncertainty (Q-3): owner-run client check needed; the rule fails closed on
disagreement instead of guessing.

## 4. Version ordering (R-05)

`version::classify(raw: Option<&str>) -> Version` and
`version::compare(a, b) -> VersionOrder` with `VersionOrder =
Less | Equal | Greater | Ambiguous(AmbiguityReason)`. Ported from
`spikes/addon/src/version.rs` (behavior, not code).

| Scheme | Recognized as | Orders against |
|---|---|---|
| `DottedNumeric` | optional `v`/`V`, ≥ 2 numeric parts separated by `.`, optional pre-release `-`/`.`/none + `dev`/`alpha`/`a`/`beta`/`b`/`rc` + optional number; or git-describe `<dotted>-<n>-g<hex>` | same scheme only; numeric parts zero-padded (`1.2` = `1.2.0`); `dev < alpha < beta < rc < release`; describe `-n-g…` is newer than its tag by `n`. Covers the spike's semver-like, numeric-dotted and git-describe as **one** scheme (Q-10) |
| `Integer` | all digits, ≤ 18 digits | `Integer` only |
| `Revision` | `r`/`R` + digits | `Revision` only |
| `Date` | `YYYY-MM-DD`, `YYYY.MM.DD`, `YYYYMMDD` with a valid date | `Date` only (checked before `Integer`) |
| `Tagged` | `<letters> <DottedNumeric>` e.g. `Forever 1.60.1` | `Tagged` with the same tag (case-insensitive); pre-release on either side → ambiguous |
| `Missing` | absent or blank | nothing |
| `Token` | contains `@` or `<…>` (unsubstituted template) | nothing |
| `GitHash` | 7–40 hex chars only | nothing (equal strings are still not proof of equal code) |
| `Other` | anything else, incl. unknown pre-release words | nothing |

Rules: any `Missing`/`Token`/`GitHash`/`Other` → `Ambiguous`; different
schemes → `Ambiguous("different version schemes")`; same scheme compares
strictly. `Equal` means equal **labels**, never equal content. There is no
input for file times, clocks or computer preference (R-05).

Tests use real labels from run-001 S-addon (`12.1.23`, `v1.3.9`, `3.220`,
`r272`, `31217`, `Forever 1.60.1`, `BCC 2.5.4`, `a8e9f0e`,
`<project-version-goes-here>`), committed as D-14 derived metadata
([tests.md](tests.md)).

## 5. Addon identity and packages (R-04, R-05, A-09)

**Folder key**: `FolderKey` = folder name lowercased (`str::to_lowercase`).
Folder names and file paths must satisfy `paths` rules: ASCII printable only
(0x20–0x7E) minus `<>:"/\|?*`, no leading/trailing space, no trailing dot, not
`.`/`..`, not a Windows reserved device name (`CON`, `PRN`, `AUX`, `NUL`,
`COM1`–`COM9`, `LPT1`–`LPT9`, with or without extension), component ≤ 255
bytes, relative path ≤ 400 bytes, no tab/newline. Non-conforming folders are
`Unsupported` and never transferred (limitation, Q-14). Two names in one
inventory with the same key → both `Unsupported` (case collision, A-23).

**Grouping** `group_packages(metas) -> Vec<Package>`: union-find over folder
keys using every observation available (local inventory plus all valid
member states), so all computers derive the same packages from the same
inputs. Edges (each recorded with its kind for explanations):

| Edge | Condition | Example (S-addon data) |
|---|---|---|
| `SharedId` | two folders' selected tocs have the same value for the same id kind | `DBM-Core`/`DBM-Raids-Midnight` (curse 3358); `VuhDo`/`VuhDoOptions` |
| `Dependency` | `F` hard-depends on installed `G` **and** `stem(G)` is a prefix of `key(F)`, where `stem(G)` = `key(G)` up to its first `-` or `_` | `DBM-Party-BC → DBM-Core` (stem `dbm`); `AtlasLootClassic_Data → AtlasLootClassic`; **not** `AtlasBIStooltips → AtlasLootClassic` |
| `NamePrefix` | `key(F)` = `key(G)` + (`-` or `_`) + more | `GearQuestForever_Languages → GearQuestForever` |

Dependencies on folders not present in any observation (e.g. `Blizzard_*`)
create no edge. Over-grouping only enlarges the unit that is replaced together
and can only add ambiguity; under-grouping can mix package parts, so the rule
errs toward grouping (Q-1). Known residual under-grouping: name-only
relatives without a separator (`QuestieDB`/`Questie`) stay separate.

**Package key** = smallest member `FolderKey` (byte order). **Display folder**
= member with an external id and fewest in-package dependencies, ties by key.

**Package version** (one per member computer) = that computer's folders of
the package, each with its selected toc version and folder tree hash, plus a
package hash over the folder hashes ([publication.md](publication.md) §3).

**Comparison** `compare_package(a, b) -> PackageOrder`:

1. Equal package hashes → `Same` (identical content converges without
   replacement, R-05).
2. Any member folder of either side with status other than `Ok`
   → `Ambiguous(FolderNotUsable)`.
3. For each folder key present on **both** sides whose tocs are `Selected` on
   both and whose tree hashes differ: `version::compare`.
   Folders with identical trees are skipped. Folders whose toc is `None`,
   `Ambiguous` or `Error` on either side and whose trees differ contribute
   `Unversioned`.
4. Any `Ambiguous` result → `Ambiguous(reason, folder)`.
5. Both `Less` and `Greater` present → `Ambiguous(MembersDisagree)`.
6. Some `Greater` (and no `Less`) → `Greater`; some `Less` → `Less`.
   `Unversioned` members follow this result (they ship with their package,
   e.g. version-less `DBM-Brawlers`).
7. No ordered results (all `Equal` labels, only `Unversioned`, or no common
   folder) → `Ambiguous(SameLabelsDifferentContent | NoComparableVersion)`
   (A-08 duplicate label, different content).

Evidence for the explanation lists every compared folder with both labels.
This compares **all** versioned members rather than only a primary folder
(S-addon suggested primary-only), because primary-only would silently
downgrade a sub-folder updated on another computer.

## 6. Group target (R-05, R-06)

For each package key, candidates = distinct package hashes among: the local
inventory and every **valid** member state of the group (excluding records
rejected for group/variant/schema/hash reasons). Member states that are
unavailable (online-only, unreadable, missing trees) contribute nothing and
are shown as unavailable; they are not absence (R-04).

- Exactly one candidate → `Chosen`.
- A candidate `c` with `compare_package(c, x)` ∈ {`Same`, `Greater`} for every
  other candidate `x` → `Chosen(c)` (it dominates even if other candidates are
  mutually ambiguous).
- Otherwise → `NeedsDecision { candidates }` for that package only; other
  packages proceed (R-06).

`sources` = all members holding the chosen hash, sorted by `ComputerId`.
Input order and duplicates never change the result: inputs are sorted by
`(ComputerId, seq)` and deduplicated by package hash before evaluation.
A package that a member does not have is simply not a candidate from that
member; absence never removes (R-04). There is no removal record in the slice.
