# S-addon evidence: addon/package identity and version ordering

Run 001, Phase 2. Worktree `D:\GitHub\wow-sync.worktrees\S-addon`, branch `run001/S-addon`.
Intent: SPEC §7 "Reliable addon/package identity and version-ordering coverage with
real metadata"; R-05. Dataset: dev PC `_forever_` AddOns (D-14), read-only,
`.toc` text parsed as data only (no Lua or saved variables read or executed).

## Question and answer

**Can addon identity and version ordering be derived reliably from real metadata?**

- Identity: **feasible with caveats** (confidence medium-high). Folder name is the only
  universal key; external ids exist on just 28.4% of folders. Multi-folder packages
  are groupable from hard dependencies and shared ids (75.5% of folders in 8 groups),
  but 24.5% of folders have a name-related sibling that no id/dependency evidence links.
- Ordering: **feasible for most, not universal** (confidence medium). 93.1% of folders
  carry a version string whose scheme can be ordered against the same scheme (PASS,
  one snapshot, dev PC). 6.9% (7 of 102: 5 missing, 1 hash, 1 template token) never
  can. Cross-scheme changes (e.g. `r272` to `12.1.23`) are ambiguous by design.
  Real two-release pairs were **not available** (one installed version per addon), so
  ordering is validated on synthetic/unit cases only (not real update histories).
- What would change it: a corpus of real consecutive releases from the provider
  (CurseForge/Wago/WoWInterface) per addon; confirmation of how the Forever client
  picks `_Camelot`/`_Forever` suffixed tocs.

## Commands and environment (Windows 11 x64 dev PC, Rust 1.100.0 pinned)

```
cd spikes\addon
cargo test                      # 12 tests: PASS (Windows dev PC)
cargo clippy --all-targets      # warnings only (dead code in bin, items after test mod)
cargo run -q --release -- "D:\Games\World of Warcraft\_forever_\Interface\AddOns" `
    --csv ..\..\docs\runs\run-001\evidence\S-addon\folders.csv `
    --report ..\..\docs\runs\run-001\evidence\S-addon\coverage.md
```

macOS runtime: BLOCKED (no Mac); the tool is std-only and path-agnostic (not run on macOS).

## Results

Full output: [coverage.md](S-addon/coverage.md) (per-scheme version lists, group
membership, interface table) and [folders.csv](S-addon/folders.csv) (one row per folder:
folder name, toc suffixes, interface numbers, version, scheme, X-ids, hard dependencies,
group). Only D-14 derived fields are committed.

Headline (102 folders, all with a `.toc`):

| measure | result |
|---|---|
| semver-like (3 numeric parts, optional `v`/pre-release) | 66 |
| tagged-numeric (`Forever 1.60.1`, `BCC 2.5.4`) | 7 |
| revision `rNNN` (all DBM `r272`) | 14 |
| integer build (ArkInventory 31217, Auctionator 346) | 5 |
| numeric-dotted other (`1.121126`, `3.220`) | 3 |
| missing `## Version` | 5 |
| bare git hash | 1 |
| unsubstituted/template token | 1 |
| date, git-describe, alpha/beta | 0 on real data (synthetic fixtures only) |
| folders with orderable scheme | 95 (93.1%) |
| folders with external id (Curse 50 tocs, Wago 40, WoWI 13) | 29 (28.4%) |
| folders identified only by folder name | 73 (71.6%) |
| folders in a confident multi-folder group | 77 (75.5%), 8 groups |
| folders with unresolved name-related sibling | 25 (24.5%) |

### Ordering rules (tested in `spikes/addon/src/version.rs`)

| scheme | orderable vs same addon? | rule / ambiguity |
|---|---|---|
| semver-like / numeric-dotted | yes | numeric components, zero-padded; `dev<alpha<beta<rc<release`; unknown suffix (`-foo`) ambiguous |
| git-describe `v1.2.3-4-gabc` | yes | by tag, then commits-ahead; ahead of tag is newer than the tag |
| date (`YYYY-MM-DD`, `YYYYMMDD`, `YYYY.MM.DD`) | yes within date | date vs semver/integer ambiguous |
| integer build / `rNNN` | yes within scheme | `r10` vs `10` ambiguous |
| tagged (`Forever 1.60.1`) | yes only for the same tag word | `BCC 2.5.4` vs `Forever 1.60.1` ambiguous |
| missing / token / bare hash / other | **no** | identical strings do not prove identical code |

### Ambiguity catalog (real data)

1. Missing `## Version`: DBM-Brawlers, DBM-Party-BfA, DBM-Party-Shadowlands,
   DBM-WorldEvents, Questie (version-less sub-modules and Questie itself).
2. Template placeholder shipped as version: ArkInventoryRules_Example.
3. Bare hash: DBM-VPVEM (`a8e9f0e`) inside a package whose peers are `12.1.23`/`r272`.
4. Same package, different schemes in different folders: ArkInventory
   (`31217` vs `1.0.0` vs placeholder), DBM (`12.1.23`, `r272`, hash, none).
   Package-level ordering must use a designated primary folder (DBM-Core,
   ArkInventory), not all folders.
5. Per-folder version disagrees across tocs: AtlasLootClassic_DungeonsAndRaids
   (`BCC 2.5.4` in one flavor toc vs `Forever 1.60.1` elsewhere).
6. Over-grouping by dependency: AtlasBIStooltips (plugin) merges into the AtlasLootClassic
   group via a hard dependency, with versions `1.0.1`/`v1.3.9`/`Forever 1.60.1`;
   a dependency is not proof of one package.
7. Under-grouping: GearQuestForever_Languages, QuestieDB/Questie/QuestieTrace and the
   Spoken* family have no id/hard-dependency link; only naming relates them.
8. Version strings are not strictly comparable across schema changes by one author
   (e.g. DBM `rNNN` to semver-like).

## Interface values and client flavor (for S-forever)

Interface numbers seen (folder counts; a folder may list several): 11508/11509 (Vanilla era, 34), 20506 (TBC/BCC, 32),
30403/30405/38000/38002 (Wrath, 25), 40402 (Cata, 11), 50503/50504 (Mists, 23),
120100/120105/120107/120007 (retail-like 12.x, 67), **16001 (82 folders)**, 0 (1 toc).

- `16001` appears in all `_Camelot` (29 toc files) and `_Forever` (10 toc files)
  suffixed tocs and in 82 folders overall; it is not a documented retail/classic
  Interface number and plausibly denotes the Forever client. This is a hint, not proof;
  S-forever must verify. `Camelot` and `Forever` are not documented toc suffixes.
- Plain `Name.toc` files carry many different numbers (the union of all flavors), so
  a plain toc's Interface does **not** identify the client; suffix plus number does.
- Multi-flavor addons ship `_Vanilla`, `_TBC`, `_BCC`, `_Wrath`, `_Cata`, `_Mists`
  tocs; one used `-Classic` (hyphen). Hyphen and underscore forms both occur, so
  parsers must accept both. The client's selection rules are documented upstream
  (suffix tocs preferred over plain); this was not verified against the live client.

## Recommendation

- **Identity key**: `(folder name case-folded)` as the canonical key, with
  `X-Curse-Project-ID`/`X-WoWI-ID`/`X-Wago-ID` as optional corroborating attributes
  (never sole key; 71.6% lack them). Group multi-folder packages by shared ids plus hard
  `Dependencies`/`RequiredDeps`, store a designated primary folder, and surface
  name-only relatives (24.5%) as "ungrouped, needs addon-level decision" rather than
  guessing. Treat plugin dependency groups as installable units only after owner/design
  review, because dependency over-groups plugins (ambiguity 6).
- **Ordering policy**: auto-select a newer release only when both versions share an
  orderable scheme (table above) and the compare is strictly Less/Greater. Anything
  else (missing, token, hash, other, cross-scheme, unknown pre-release, tag mismatch)
  is an addon-level decision; never fall back to file timestamps (R-05).
  Compare per primary folder, not per sub-folder.
- Ask the owner/design: whether tagged `Forever x.y.z` strings may be ordered only
  within their tag; how to treat packages with a missing/hash primary version.

## Method limits and risks

- One snapshot (102 folders): no real consecutive-release pairs, so "demonstrably
  newer" correctness against real updates is untested; date, git-describe and
  alpha/beta schemes exist only as synthetic fixtures under `spikes/addon/fixtures/`.
- Group detection is heuristic (name relation is a flag, not a decision); the
  folder-count percentages depend on this dataset (many suites by the same authors).
- Tocs are read as bytes (lossy UTF-8); BOM tolerated. Not verified: `#` comment
  lines, `## Dependencies` case variants beyond the keys handled, localized
  `Title-xxXX` fields (ignored).
- `Order::Equal` means equal version strings, not byte-identical code.

## Spike code

`spikes/addon/` (own Cargo workspace, std-only): `src/toc.rs` (parsing, suffix split),
`src/version.rs` (classification, ordering, unit tests), `src/main.rs` (scan, grouping,
report; fixture test), `fixtures/` (synthetic tocs for every scheme). Rerun with the
commands above.
