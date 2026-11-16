//! Variant identity validation, layout assessment and guided confirmation (domain.md §2).

use crate::bpsv::{parse_bpsv, Bpsv};
use crate::hash::Hasher;
use crate::ids::Sha256;
use crate::inventory::{FolderScan, ScanStatus};
use crate::paths::{validate_folder_name, FolderKey};
use crate::toc::{select_toc, TocSelection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SYNTHETIC_MARKER_LINE: &str = "wowsync-synthetic/1";
pub const PLAIN_MAPPING_ID: &str = "validated-plain/1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileObs {
    Missing,
    Unreadable(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirObs {
    Missing,
    PlainDir,
    NotPlain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantObservation {
    pub folder_name: String,
    pub flavor_info: FileObs,
    pub build_info: FileObs,
    pub addons_dir: DirObs,
    /// `<install root>/.wowsync-synthetic` (slice gate, F-01).
    pub synthetic_marker: FileObs,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProductCode(String);

impl ProductCode {
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let ok = !s.is_empty()
            && s.len() <= 32
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        ok.then(|| Self(s.to_string()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub product: &'static str,
    pub folder: &'static str,
    pub label: &'static str,
    pub toc_suffixes: &'static [&'static str],
    pub mapping_id: &'static str,
}

/// Only `wowf` (Forever) is known in the slice. It is not Classic Era.
pub const KNOWN_PROFILES: &[Profile] = &[Profile {
    product: "wowf",
    folder: "_forever_",
    label: "Forever",
    toc_suffixes: &["Forever", "Camelot"],
    mapping_id: "profile:wowf/1",
}];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantIdentity {
    pub product: ProductCode,
    pub folder: String,
    pub build_version: Option<String>,
    pub profile: Option<&'static Profile>,
}

impl VariantIdentity {
    /// Canonical product label, e.g. "World of Warcraft: Forever (wowf)".
    pub fn display_label(&self) -> String {
        match self.profile {
            Some(p) => format!("World of Warcraft: {} ({})", p.label, self.product.as_str()),
            None => format!("Unfamiliar game version ({})", self.product.as_str()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseReason {
    FlavorMissing,
    FlavorUnreadable(String),
    FlavorUnparseable,
    InvalidProductCode,
    BuildInfo(String),
    ProductNotInBuildInfo,
    ProductAmbiguousInBuildInfo,
    ProductInactive,
    FolderMismatch,
    AddOnsMissing,
    /// F-01: the install root has no valid synthetic marker (Q-24).
    NotSyntheticInstall,
    Layout(LayoutReason),
    ConfirmationRequired,
    StaleConfirmation,
    /// Reconfirmation offered for a different product or folder than the enrolled one.
    VariantChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariantVerdict {
    Recognized(VariantIdentity),
    Unfamiliar(VariantIdentity),
    Refused(RefuseReason),
}

fn column(b: &Bpsv, name: &str) -> Option<usize> {
    b.columns.iter().position(|c| c.eq_ignore_ascii_case(name))
}

fn marker_ok(obs: &FileObs) -> bool {
    match obs {
        FileObs::Bytes(b) => {
            let b = b.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(b);
            let first = b.split(|&c| c == b'\n').next().unwrap_or(&[]);
            let first = first.strip_suffix(b"\r").unwrap_or(first);
            first == SYNTHETIC_MARKER_LINE.as_bytes()
        }
        _ => false,
    }
}

/// Validates metadata (domain.md §2 table) and then the synthetic marker. The marker is
/// consumed here, after metadata errors, so no usable verdict exists without it (F-01).
pub fn validate_variant(obs: &VariantObservation) -> VariantVerdict {
    use RefuseReason as R;
    use VariantVerdict::Refused;
    let flavor = match &obs.flavor_info {
        FileObs::Missing => return Refused(R::FlavorMissing),
        FileObs::Unreadable(e) => return Refused(R::FlavorUnreadable(e.clone())),
        FileObs::Bytes(b) => b,
    };
    let flavor = match parse_bpsv(flavor) {
        Ok(f) if f.columns.len() == 1 && !f.rows.is_empty() => f,
        _ => return Refused(R::FlavorUnparseable),
    };
    let Some(product) = ProductCode::parse(&flavor.rows[0][0]) else {
        return Refused(R::InvalidProductCode);
    };
    let build = match &obs.build_info {
        FileObs::Missing => return Refused(R::BuildInfo("missing".into())),
        FileObs::Unreadable(e) => return Refused(R::BuildInfo(format!("unreadable: {e}"))),
        FileObs::Bytes(b) => match parse_bpsv(b) {
            Ok(b) => b,
            Err(e) => return Refused(R::BuildInfo(format!("{e:?}"))),
        },
    };
    let (Some(pc), Some(ac)) = (column(&build, "Product"), column(&build, "Active")) else {
        return Refused(R::BuildInfo("lacks Product or Active column".into()));
    };
    let vc = column(&build, "Version");
    let matching: Vec<&Vec<String>> = build
        .rows
        .iter()
        .filter(|r| r[pc] == product.as_str())
        .collect();
    let row = match matching.as_slice() {
        [] => return Refused(R::ProductNotInBuildInfo),
        [r] => *r,
        _ => return Refused(R::ProductAmbiguousInBuildInfo),
    };
    if row[ac] != "1" {
        return Refused(R::ProductInactive);
    }
    let profile = KNOWN_PROFILES
        .iter()
        .find(|p| p.product == product.as_str());
    if let Some(p) = profile {
        if !obs.folder_name.eq_ignore_ascii_case(p.folder) {
            return Refused(R::FolderMismatch);
        }
    }
    if obs.addons_dir != DirObs::PlainDir {
        return Refused(R::AddOnsMissing);
    }
    if !marker_ok(&obs.synthetic_marker) {
        return Refused(R::NotSyntheticInstall);
    }
    let build_version = vc
        .map(|i| row[i].trim().to_string())
        .filter(|v| !v.is_empty());
    let identity = VariantIdentity {
        product,
        folder: obs.folder_name.clone(),
        build_version,
        profile,
    };
    if profile.is_some() {
        VariantVerdict::Recognized(identity)
    } else {
        VariantVerdict::Unfamiliar(identity)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TocMapping {
    Profile(&'static Profile),
    ValidatedPlain,
}

impl TocMapping {
    pub fn id(&self) -> &'static str {
        match self {
            TocMapping::Profile(p) => p.mapping_id,
            TocMapping::ValidatedPlain => PLAIN_MAPPING_ID,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OtherVariants {
    pub same_product_folders: Vec<String>,
    pub enrolled_elsewhere: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutReason {
    /// Not reachable from `assess_layout` (no AddOns observation); `validate_variant` returns `AddOnsMissing`.
    AddOnsLayout,
    DuplicateProductFolder,
    AlreadyEnrolled,
    /// Not reachable from `assess_layout`; the marker is consumed by `validate_variant` (F-01).
    NotSyntheticInstall,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BlockReason {
    UnsupportedName(String),
    CaseCollision,
    NotReadable(String),
    UnvalidatedTocMapping,
    NoToc,
    TocError(String),
    TocAmbiguous,
}

impl BlockReason {
    fn code(&self) -> &'static str {
        match self {
            BlockReason::UnsupportedName(_) => "name",
            BlockReason::CaseCollision => "case",
            BlockReason::NotReadable(_) => "unreadable",
            BlockReason::UnvalidatedTocMapping => "unvalidated",
            BlockReason::NoToc => "notoc",
            BlockReason::TocError(_) => "tocerror",
            BlockReason::TocAmbiguous => "ambiguous",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutAssessment {
    Supported {
        mapping: TocMapping,
        eligible: Vec<FolderKey>,
        blocked: Vec<(FolderKey, BlockReason)>,
        digest: Sha256,
    },
    Unsupported(LayoutReason),
}

pub fn assess_layout(
    id: &VariantIdentity,
    _unfamiliar: bool,
    addons: &[FolderScan],
    others: &OtherVariants,
) -> LayoutAssessment {
    if !others.same_product_folders.is_empty() {
        return LayoutAssessment::Unsupported(LayoutReason::DuplicateProductFolder);
    }
    if others.enrolled_elsewhere {
        return LayoutAssessment::Unsupported(LayoutReason::AlreadyEnrolled);
    }
    let mapping = TocMapping::ValidatedPlain;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for f in addons {
        *counts.entry(f.name.to_lowercase()).or_default() += 1;
    }
    let mut eligible = Vec::new();
    let mut blocked = Vec::new();
    for f in addons {
        if matches!(f.status, ScanStatus::Unstable(_)) {
            continue;
        }
        let verdict = classify_folder(
            f,
            counts.get(&f.name.to_lowercase()).copied().unwrap_or(0),
            &mapping,
        );
        let key = validate_folder_name(&f.name).unwrap_or_else(|_| FolderKey::lossy(&f.name));
        match verdict {
            Ok(()) => eligible.push(key),
            Err(r) => blocked.push((key, r)),
        }
    }
    eligible.sort();
    blocked.sort();
    let mut h = Hasher::new();
    h.update(
        format!(
            "wowsync-layout-v1\n{}\t{}\t{}\t{}\n",
            id.product.as_str(),
            id.folder,
            id.build_version.as_deref().unwrap_or("-"),
            mapping.id()
        )
        .as_bytes(),
    );
    for k in &eligible {
        h.update(format!("E\t{k}\n").as_bytes());
    }
    for (k, r) in &blocked {
        h.update(format!("B\t{k}\t{}\n", r.code()).as_bytes());
    }
    LayoutAssessment::Supported {
        mapping,
        eligible,
        blocked,
        digest: h.finish(),
    }
}

fn classify_folder(
    f: &FolderScan,
    name_count: usize,
    mapping: &TocMapping,
) -> Result<(), BlockReason> {
    if let Err(e) = validate_folder_name(&f.name) {
        return Err(BlockReason::UnsupportedName(e.to_string()));
    }
    if name_count > 1 {
        return Err(BlockReason::CaseCollision);
    }
    match &f.status {
        ScanStatus::Ok => {}
        ScanStatus::Unreadable(r) | ScanStatus::Unsafe(r) | ScanStatus::Unstable(r) => {
            return Err(BlockReason::NotReadable(r.clone()))
        }
    }
    match select_toc(&f.name, &f.tocs, mapping) {
        TocSelection::Selected { .. } => Ok(()),
        TocSelection::Unvalidated { .. } => Err(BlockReason::UnvalidatedTocMapping),
        TocSelection::None { .. } => Err(BlockReason::NoToc),
        TocSelection::Ambiguous { .. } => Err(BlockReason::TocAmbiguous),
        TocSelection::Error { reason, .. } => Err(BlockReason::TocError(reason)),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuidedConfirmation {
    pub identity: bool,
    pub scope: bool,
    pub digest: Sha256,
}

/// Evidence stored with the local enrollment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VariantValidation {
    /// `known` or `guided`.
    pub kind: String,
    pub product: String,
    pub folder: String,
    pub build_version: Option<String>,
    pub mapping_id: String,
    pub digest: Sha256,
    pub eligible: u32,
    pub blocked: u32,
    pub identity_confirmed: bool,
    pub scope_confirmed: bool,
}

/// The only value accepted by enrollment and planning; constructible only by `confirm_variant`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmedVariant {
    identity: VariantIdentity,
    mapping: TocMapping,
    validation: VariantValidation,
}

impl ConfirmedVariant {
    pub fn identity(&self) -> &VariantIdentity {
        &self.identity
    }
    pub fn mapping(&self) -> &TocMapping {
        &self.mapping
    }
    pub fn validation(&self) -> &VariantValidation {
        &self.validation
    }
    pub fn product(&self) -> &str {
        self.identity.product.as_str()
    }
    pub fn label(&self) -> &str {
        self.identity
            .profile
            .map_or(self.identity.product.as_str(), |p| p.label)
    }
}

/// Recognized + Supported: Ok. Unfamiliar + Supported: Ok only with both confirmations for the
/// current digest. Anything else, including consent without a Supported assessment: Err.
pub fn confirm_variant(
    v: &VariantVerdict,
    a: &LayoutAssessment,
    c: Option<&GuidedConfirmation>,
) -> Result<ConfirmedVariant, RefuseReason> {
    let (identity, unfamiliar) = match v {
        VariantVerdict::Refused(r) => return Err(r.clone()),
        VariantVerdict::Recognized(i) => (i, false),
        VariantVerdict::Unfamiliar(i) => (i, true),
    };
    let LayoutAssessment::Supported {
        mapping,
        eligible,
        blocked,
        digest,
    } = a
    else {
        let LayoutAssessment::Unsupported(r) = a else {
            unreachable!()
        };
        return Err(RefuseReason::Layout(r.clone()));
    };
    let (identity_confirmed, scope_confirmed) = match c {
        Some(c) => {
            if unfamiliar {
                if !c.identity || !c.scope {
                    return Err(RefuseReason::ConfirmationRequired);
                }
                if c.digest != *digest {
                    return Err(RefuseReason::StaleConfirmation);
                }
            }
            (c.identity, c.scope)
        }
        None if unfamiliar => return Err(RefuseReason::ConfirmationRequired),
        None => (false, false),
    };
    Ok(ConfirmedVariant {
        identity: identity.clone(),
        mapping: mapping.clone(),
        validation: VariantValidation {
            kind: if unfamiliar { "guided" } else { "known" }.to_string(),
            product: identity.product.as_str().to_string(),
            folder: identity.folder.clone(),
            build_version: identity.build_version.clone(),
            mapping_id: mapping.id().to_string(),
            digest: *digest,
            eligible: eligible.len() as u32,
            blocked: blocked.len() as u32,
            identity_confirmed,
            scope_confirmed,
        },
    })
}

/// Per-cycle re-derivation for an already enrolled variant (M1). The scope confirmation covers
/// the mapping rule, not a frozen folder list (domain.md §2.1), so folders added later never
/// invalidate it. Only a different product, folder, mapping or recognition class does; the
/// caller then offers guided reconfirmation.
pub fn confirm_enrolled(
    v: &VariantVerdict,
    a: &LayoutAssessment,
    stored: &VariantValidation,
) -> Result<ConfirmedVariant, RefuseReason> {
    let (identity, unfamiliar) = match v {
        VariantVerdict::Refused(r) => return Err(r.clone()),
        VariantVerdict::Recognized(i) => (i, false),
        VariantVerdict::Unfamiliar(i) => (i, true),
    };
    let LayoutAssessment::Supported { mapping, .. } = a else {
        let LayoutAssessment::Unsupported(r) = a else {
            unreachable!()
        };
        return Err(RefuseReason::Layout(r.clone()));
    };
    if stored.product != identity.product.as_str() || stored.folder != identity.folder {
        return Err(RefuseReason::VariantChanged);
    }
    if unfamiliar != (stored.kind == "guided") || stored.mapping_id != mapping.id() {
        return Err(RefuseReason::StaleConfirmation);
    }
    if unfamiliar && !(stored.identity_confirmed && stored.scope_confirmed) {
        return Err(RefuseReason::ConfirmationRequired);
    }
    Ok(ConfirmedVariant {
        identity: identity.clone(),
        mapping: mapping.clone(),
        validation: stored.clone(),
    })
}

/// Guided reconfirmation of an enrolled variant whose stored confirmation went stale. The
/// product and folder must still be the enrolled ones; consent never changes which client this is.
pub fn reconfirm_variant(
    v: &VariantVerdict,
    a: &LayoutAssessment,
    c: &GuidedConfirmation,
    stored: &VariantValidation,
) -> Result<ConfirmedVariant, RefuseReason> {
    if let VariantVerdict::Recognized(i) | VariantVerdict::Unfamiliar(i) = v {
        if stored.product != i.product.as_str() || stored.folder != i.folder {
            return Err(RefuseReason::VariantChanged);
        }
    }
    confirm_variant(v, a, Some(c))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentAction {
    Enroll,
    Reconfirm,
}

/// Why the consent given does not let an enroll or reconfirm proceed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsentGap {
    /// The add-on scope was not agreed to.
    Scope,
    /// The game version was not confirmed where that is required.
    Identity,
    /// The consent was given for a different preview than the current assessment.
    Stale,
}

impl From<ConsentGap> for RefuseReason {
    fn from(g: ConsentGap) -> Self {
        match g {
            ConsentGap::Scope | ConsentGap::Identity => RefuseReason::ConfirmationRequired,
            ConsentGap::Stale => RefuseReason::StaleConfirmation,
        }
    }
}

/// Whether an enroll or reconfirm may proceed on this consent (R-03, A-46; mi-1). Scope
/// consent is always required. Identity confirmation is required for an unfamiliar (guided)
/// client and for every reconfirmation. For a guided client the consent must be for the
/// current assessment digest (`None` = no supported assessment; `confirm_variant` refuses).
/// This is checked in addition to `confirm_variant`, which decides the variant itself.
pub fn consent_gap(
    action: ConsentAction,
    guided: bool,
    c: &GuidedConfirmation,
    current_digest: Option<&Sha256>,
) -> Option<ConsentGap> {
    if !c.scope {
        return Some(ConsentGap::Scope);
    }
    if (guided || action == ConsentAction::Reconfirm) && !c.identity {
        return Some(ConsentGap::Identity);
    }
    match current_digest {
        Some(d) if guided && *d != c.digest => Some(ConsentGap::Stale),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinRefusal {
    pub group_product: String,
    pub local_product: String,
}

impl std::fmt::Display for JoinRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "that group syncs {}, this installation is {}",
            self.group_product, self.local_product
        )
    }
}

/// Join is allowed only when the group's product code equals the validated local one (domain.md §1 step 4).
pub fn check_join(v: &ConfirmedVariant, group_product: &str) -> Result<(), JoinRefusal> {
    if group_product == v.product() {
        Ok(())
    } else {
        Err(JoinRefusal {
            group_product: group_product.to_string(),
            local_product: v.product().to_string(),
        })
    }
}

pub struct ConsentContext<'a> {
    pub identity: &'a VariantIdentity,
    pub mapping_id: &'a str,
    pub install_root: &'a str,
    pub addons_path: &'a str,
}

/// Disclosures shown before any enrollment write (domain.md §1 step 5 and §2.1).
pub fn consent_disclosures(c: &ConsentContext) -> Vec<String> {
    let label = c.identity.display_label();
    vec![
        format!(
            "This computer will share its installed add-ons for {label} with other computers in this group, and will install newer versions found on them."
        ),
        format!(
            "Chosen install location: {}. Add-ons are read from and changed only in {}.",
            c.install_root, c.addons_path
        ),
        format!(
            "Folder rule {}: an add-on folder is synced only if it has a plain <Name>.toc and no client-specific variants of it. Other folders are listed as blocked and are never changed.",
            c.mapping_id
        ),
        "Newer add-on versions found on other computers are installed, replacing the folder here, automatically.".to_string(),
        "Add-ons missing on this computer are installed, never treated as removed; an add-on missing elsewhere is never deleted here.".to_string(),
        format!(
            "Folders it replaces are kept on this computer for recovery under {}\\{}\\.wowsync\\recovery.",
            c.install_root, c.identity.folder
        ),
        "Settings (WTF) are not synchronized in this version; their layout for this client has not been validated.".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit;

    fn obs(product: &str, folder: &str) -> VariantObservation {
        testkit::variant_obs(product, folder)
    }

    fn with_flavor(mut o: VariantObservation, bytes: &[u8]) -> VariantObservation {
        o.flavor_info = FileObs::Bytes(bytes.to_vec());
        o
    }

    fn refused(o: &VariantObservation) -> RefuseReason {
        match validate_variant(o) {
            VariantVerdict::Refused(r) => r,
            other => panic!("expected refusal, got {other:?}"),
        }
    }

    fn scan(name: &str, tocs: &[&str]) -> FolderScan {
        testkit::folder_scan(name, &[("a.lua", b"x")], tocs)
    }

    #[test]
    fn forever_metadata_is_recognized() {
        match validate_variant(&testkit::variant_obs_forever()) {
            VariantVerdict::Recognized(id) => {
                assert_eq!(id.product.as_str(), "wowf");
                assert_eq!(id.folder, "_forever_");
                assert_eq!(id.build_version.as_deref(), Some("1.15.8.12345"));
                assert_eq!(id.profile.map(|p| p.label), Some("Forever"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn flavor_build_info_disagreement_refused() {
        let mut o = testkit::variant_obs_forever();
        o.build_info =
            FileObs::Bytes(b"Product!STRING:0|Active!DEC:1\nwow_classic_era|1\n".to_vec());
        assert_eq!(refused(&o), RefuseReason::ProductNotInBuildInfo);
    }

    #[test]
    fn missing_flavor_info_refused() {
        let mut o = testkit::variant_obs_forever();
        o.flavor_info = FileObs::Missing;
        assert_eq!(refused(&o), RefuseReason::FlavorMissing);
        o.flavor_info = FileObs::Unreadable("denied".into());
        assert!(matches!(refused(&o), RefuseReason::FlavorUnreadable(_)));
        assert_eq!(
            refused(&with_flavor(o.clone(), b"A!S:0|B!S:0\nx|y\n")),
            RefuseReason::FlavorUnparseable
        );
        assert_eq!(
            refused(&with_flavor(o, b"A!S:0\n")),
            RefuseReason::FlavorUnparseable
        );
    }

    #[test]
    fn inactive_product_refused() {
        let mut o = testkit::variant_obs_forever();
        o.build_info = FileObs::Bytes(b"Product!STRING:0|Active!DEC:1\nwowf|0\n".to_vec());
        assert_eq!(refused(&o), RefuseReason::ProductInactive);
    }

    #[test]
    fn ambiguous_build_rows_refused() {
        let mut o = testkit::variant_obs_forever();
        o.build_info = FileObs::Bytes(b"Product!STRING:0|Active!DEC:1\nwowf|1\nwowf|1\n".to_vec());
        assert_eq!(refused(&o), RefuseReason::ProductAmbiguousInBuildInfo);
        o.build_info = FileObs::Bytes(b"Product!STRING:0|Other!DEC:1\nwowf|1\n".to_vec());
        assert!(matches!(refused(&o), RefuseReason::BuildInfo(_)));
        o.build_info = FileObs::Missing;
        assert!(matches!(refused(&o), RefuseReason::BuildInfo(_)));
    }

    #[test]
    fn known_product_in_wrong_folder_refused() {
        assert_eq!(
            refused(&obs("wowf", "_classic_era_")),
            RefuseReason::FolderMismatch
        );
        let mut o = obs("wowf", "_forever_");
        o.addons_dir = DirObs::NotPlain;
        assert_eq!(refused(&o), RefuseReason::AddOnsMissing);
        o.addons_dir = DirObs::Missing;
        assert_eq!(refused(&o), RefuseReason::AddOnsMissing);
    }

    #[test]
    fn invalid_product_code_refused() {
        for bad in ["WoWF", "wow-f", "", &"a".repeat(33)] {
            let o = with_flavor(
                testkit::variant_obs_forever(),
                format!("Product!STRING:0\n{bad}\n").as_bytes(),
            );
            assert!(
                matches!(
                    refused(&o),
                    RefuseReason::InvalidProductCode | RefuseReason::FlavorUnparseable
                ),
                "{bad:?}"
            );
        }
        let o = with_flavor(testkit::variant_obs_forever(), b"Product!STRING:0\nWoWF\n");
        assert_eq!(refused(&o), RefuseReason::InvalidProductCode);
    }

    #[test]
    fn unfamiliar_product_requires_confirmation() {
        let v = validate_variant(&obs("wow_test", "_test_"));
        let VariantVerdict::Unfamiliar(id) = &v else {
            panic!("{v:?}")
        };
        let a = assess_layout(
            id,
            true,
            &[scan("Good", &["Good.toc"])],
            &OtherVariants::default(),
        );
        assert!(matches!(
            confirm_variant(&v, &a, None),
            Err(RefuseReason::ConfirmationRequired)
        ));
        let LayoutAssessment::Supported { digest, .. } = &a else {
            panic!()
        };
        let half = GuidedConfirmation {
            identity: true,
            scope: false,
            digest: *digest,
        };
        assert!(confirm_variant(&v, &a, Some(&half)).is_err());
        let half = GuidedConfirmation {
            identity: false,
            scope: true,
            digest: *digest,
        };
        assert!(confirm_variant(&v, &a, Some(&half)).is_err());
        let full = GuidedConfirmation {
            identity: true,
            scope: true,
            digest: *digest,
        };
        let ok = confirm_variant(&v, &a, Some(&full)).unwrap();
        assert_eq!(ok.validation().kind, "guided");
        assert_eq!(ok.mapping().id(), PLAIN_MAPPING_ID);
        let known = validate_variant(&testkit::variant_obs_forever());
        let VariantVerdict::Recognized(kid) = &known else {
            panic!()
        };
        let ka = assess_layout(kid, false, &[], &OtherVariants::default());
        assert_eq!(
            confirm_variant(&known, &ka, None)
                .unwrap()
                .validation()
                .kind,
            "known"
        );
    }

    #[test]
    fn wowf_label_is_forever_not_classic_era() {
        let p = &KNOWN_PROFILES[0];
        assert_eq!(p.label, "Forever");
        assert_eq!(p.product, "wowf");
        assert!(!p.label.to_lowercase().contains("classic"));
        assert_eq!(KNOWN_PROFILES.len(), 1);
    }

    #[test]
    fn unknown_product_with_agreeing_metadata_blocked_without_layout_validation() {
        let v = validate_variant(&obs("wow_test", "_test_"));
        assert!(matches!(v, VariantVerdict::Unfamiliar(_)));
        let bad = LayoutAssessment::Unsupported(LayoutReason::DuplicateProductFolder);
        let full = GuidedConfirmation {
            identity: true,
            scope: true,
            digest: Sha256([0; 32]),
        };
        assert!(matches!(
            confirm_variant(&v, &bad, Some(&full)),
            Err(RefuseReason::Layout(LayoutReason::DuplicateProductFolder))
        ));
    }

    #[test]
    fn consent_alone_does_not_confirm() {
        let v = validate_variant(&obs("wow_test", "_test_"));
        let VariantVerdict::Unfamiliar(id) = &v else {
            panic!()
        };
        let a = assess_layout(
            id,
            true,
            &[scan("Good", &["Good.toc"])],
            &OtherVariants::default(),
        );
        // Consent with no confirmations at all (a GUI "OK" press) never validates.
        let consent = GuidedConfirmation {
            identity: false,
            scope: false,
            digest: Sha256([0; 32]),
        };
        assert!(confirm_variant(&v, &a, Some(&consent)).is_err());
        let un = LayoutAssessment::Unsupported(LayoutReason::AddOnsLayout);
        let full = GuidedConfirmation {
            identity: true,
            scope: true,
            digest: Sha256([0; 32]),
        };
        assert!(confirm_variant(&v, &un, Some(&full)).is_err());
        assert!(confirm_variant(
            &VariantVerdict::Refused(RefuseReason::ProductInactive),
            &a,
            Some(&full)
        )
        .is_err());
    }

    #[test]
    fn stale_digest_rejected() {
        let v = validate_variant(&obs("wow_test", "_test_"));
        let VariantVerdict::Unfamiliar(id) = &v else {
            panic!()
        };
        let a1 = assess_layout(
            id,
            true,
            &[scan("Good", &["Good.toc"])],
            &OtherVariants::default(),
        );
        let a2 = assess_layout(
            id,
            true,
            &[scan("Good", &["Good.toc"]), scan("Other", &["Other.toc"])],
            &OtherVariants::default(),
        );
        let LayoutAssessment::Supported { digest: d1, .. } = &a1 else {
            panic!()
        };
        let LayoutAssessment::Supported { digest: d2, .. } = &a2 else {
            panic!()
        };
        assert_ne!(d1, d2);
        let c = GuidedConfirmation {
            identity: true,
            scope: true,
            digest: *d1,
        };
        assert!(confirm_variant(&v, &a1, Some(&c)).is_ok());
        assert_eq!(
            confirm_variant(&v, &a2, Some(&c)),
            Err(RefuseReason::StaleConfirmation)
        );
    }

    #[test]
    fn suffixed_toc_folders_blocked_for_unfamiliar() {
        let v = validate_variant(&obs("wow_test", "_test_"));
        let VariantVerdict::Unfamiliar(id) = &v else {
            panic!()
        };
        let addons = [
            scan("Plain", &["Plain.toc"]),
            scan("Mixed", &["Mixed.toc", "Mixed_Mists.toc"]),
            scan("OnlySuffix", &["OnlySuffix-Classic.toc"]),
            scan("NoToc", &[]),
            scan("Bad Name.", &["Bad Name..toc"]),
        ];
        let LayoutAssessment::Supported {
            eligible, blocked, ..
        } = assess_layout(id, true, &addons, &OtherVariants::default())
        else {
            panic!()
        };
        assert_eq!(
            eligible.iter().map(|k| k.as_str()).collect::<Vec<_>>(),
            vec!["plain"]
        );
        let reason = |n: &str| {
            blocked
                .iter()
                .find(|(k, _)| k.as_str() == n)
                .map(|(_, r)| r.clone())
        };
        assert_eq!(reason("mixed"), Some(BlockReason::UnvalidatedTocMapping));
        assert_eq!(
            reason("onlysuffix"),
            Some(BlockReason::UnvalidatedTocMapping)
        );
        assert_eq!(reason("notoc"), Some(BlockReason::NoToc));
        assert!(matches!(
            reason("bad name."),
            Some(BlockReason::UnsupportedName(_))
        ));
        // The recognized client uses the same plain mapping: its suffixed folders are blocked too.
        let known = validate_variant(&testkit::variant_obs_forever());
        let VariantVerdict::Recognized(kid) = &known else {
            panic!()
        };
        let LayoutAssessment::Supported {
            mapping, eligible, ..
        } = assess_layout(kid, false, &addons[..2], &OtherVariants::default())
        else {
            panic!()
        };
        assert_eq!(mapping, TocMapping::ValidatedPlain);
        assert_eq!(eligible.len(), 1);
    }

    #[test]
    fn duplicate_product_folder_unsupported() {
        let v = validate_variant(&obs("wow_test", "_test_"));
        let VariantVerdict::Unfamiliar(id) = &v else {
            panic!()
        };
        let others = OtherVariants {
            same_product_folders: vec!["_test_ptr_".into()],
            enrolled_elsewhere: false,
        };
        assert_eq!(
            assess_layout(id, true, &[], &others),
            LayoutAssessment::Unsupported(LayoutReason::DuplicateProductFolder)
        );
    }

    #[test]
    fn already_enrolled_variant_unsupported() {
        let v = validate_variant(&testkit::variant_obs_forever());
        let VariantVerdict::Recognized(id) = &v else {
            panic!()
        };
        let others = OtherVariants {
            same_product_folders: vec![],
            enrolled_elsewhere: true,
        };
        assert_eq!(
            assess_layout(id, false, &[], &others),
            LayoutAssessment::Unsupported(LayoutReason::AlreadyEnrolled)
        );
    }

    #[test]
    fn install_without_synthetic_marker_refused() {
        // F-01: both recognized and unfamiliar clients, every bad marker shape, and removal after enrollment.
        let bad_markers = [
            FileObs::Missing,
            FileObs::Unreadable("denied".into()),
            FileObs::Bytes(b"wowsync-synthetic/2\n".to_vec()),
            FileObs::Bytes(b"\nwowsync-synthetic/1\n".to_vec()),
            FileObs::Bytes(b"real install".to_vec()),
            FileObs::Bytes(Vec::new()),
        ];
        for base in [testkit::variant_obs_forever(), obs("wow_test", "_test_")] {
            assert!(!matches!(
                validate_variant(&base),
                VariantVerdict::Refused(_)
            ));
            for m in &bad_markers {
                let mut o = base.clone();
                o.synthetic_marker = m.clone();
                assert_eq!(refused(&o), RefuseReason::NotSyntheticInstall, "{m:?}");
            }
            for ok in [
                b"wowsync-synthetic/1".to_vec(),
                b"wowsync-synthetic/1\r\nextra\n".to_vec(),
            ] {
                let mut o = base.clone();
                o.synthetic_marker = FileObs::Bytes(ok);
                assert!(!matches!(validate_variant(&o), VariantVerdict::Refused(_)));
            }
        }
        // Metadata errors keep priority over the marker.
        let mut o = testkit::variant_obs_forever();
        o.synthetic_marker = FileObs::Missing;
        o.flavor_info = FileObs::Missing;
        assert_eq!(refused(&o), RefuseReason::FlavorMissing);
        // Enrolled, then the marker is removed: re-validation each cycle refuses it.
        let mut o = testkit::variant_obs_forever();
        let enrolled = testkit::confirmed_forever();
        assert_eq!(enrolled.product(), "wowf");
        o.synthetic_marker = FileObs::Missing;
        assert_eq!(refused(&o), RefuseReason::NotSyntheticInstall);
    }

    fn guided_unfamiliar(addons: &[FolderScan]) -> (VariantVerdict, LayoutAssessment) {
        let v = validate_variant(&obs("wow_test", "_test_"));
        let VariantVerdict::Unfamiliar(id) = &v else {
            panic!()
        };
        let a = assess_layout(id, true, addons, &OtherVariants::default());
        (v, a)
    }

    fn confirm_guided(v: &VariantVerdict, a: &LayoutAssessment) -> ConfirmedVariant {
        let LayoutAssessment::Supported { digest, .. } = a else {
            panic!()
        };
        let c = GuidedConfirmation {
            identity: true,
            scope: true,
            digest: *digest,
        };
        confirm_variant(v, a, Some(&c)).unwrap()
    }

    #[test]
    fn later_folders_do_not_invalidate_enrollment() {
        let (v, a) = guided_unfamiliar(&[scan("Good", &["Good.toc"])]);
        let stored = confirm_guided(&v, &a).validation().clone();
        // A new eligible folder and a new blocked folder appear; the digest differs.
        let (v2, a2) = guided_unfamiliar(&[
            scan("Good", &["Good.toc"]),
            scan("Extra", &["Extra.toc"]),
            scan("Mixed", &["Mixed.toc", "Mixed_Mists.toc"]),
        ]);
        let LayoutAssessment::Supported { digest, .. } = &a2 else {
            panic!()
        };
        assert_ne!(*digest, stored.digest);
        let again = confirm_enrolled(&v2, &a2, &stored).expect("additions keep enrollment valid");
        assert_eq!(again.validation(), &stored);
        assert_eq!(again.mapping().id(), PLAIN_MAPPING_ID);
    }

    #[test]
    fn enrollment_goes_stale_only_for_product_folder_or_class_change() {
        let (v, a) = guided_unfamiliar(&[scan("Good", &["Good.toc"])]);
        let stored = confirm_guided(&v, &a).validation().clone();
        let other = validate_variant(&obs("wow_other", "_test_"));
        let VariantVerdict::Unfamiliar(oid) = &other else {
            panic!()
        };
        let oa = assess_layout(oid, true, &[], &OtherVariants::default());
        assert_eq!(
            confirm_enrolled(&other, &oa, &stored),
            Err(RefuseReason::VariantChanged)
        );
        // Recognized now but enrolled as guided, and a different product and folder: needs help.
        let known = validate_variant(&testkit::variant_obs_forever());
        let VariantVerdict::Recognized(kid) = &known else {
            panic!()
        };
        let ka = assess_layout(kid, false, &[], &OtherVariants::default());
        assert_eq!(
            confirm_enrolled(&known, &ka, &stored),
            Err(RefuseReason::VariantChanged)
        );
        // Same product and folder but a different recognition class: reconfirmation can help.
        let mut as_known = stored.clone();
        as_known.kind = "known".into();
        assert_eq!(
            confirm_enrolled(&v, &a, &as_known),
            Err(RefuseReason::StaleConfirmation)
        );
        let mut unconfirmed = stored.clone();
        unconfirmed.scope_confirmed = false;
        assert_eq!(
            confirm_enrolled(&v, &a, &unconfirmed),
            Err(RefuseReason::ConfirmationRequired)
        );
        let bad = LayoutAssessment::Unsupported(LayoutReason::DuplicateProductFolder);
        assert!(matches!(
            confirm_enrolled(&v, &bad, &stored),
            Err(RefuseReason::Layout(_))
        ));
        let refused = VariantVerdict::Refused(RefuseReason::NotSyntheticInstall);
        assert_eq!(
            confirm_enrolled(&refused, &a, &stored),
            Err(RefuseReason::NotSyntheticInstall)
        );
    }

    #[test]
    fn reconfirmation_needs_same_client_and_both_confirmations() {
        let (v, a) = guided_unfamiliar(&[scan("Good", &["Good.toc"])]);
        let stored = confirm_guided(&v, &a).validation().clone();
        let LayoutAssessment::Supported { digest, .. } = &a else {
            panic!()
        };
        let full = GuidedConfirmation {
            identity: true,
            scope: true,
            digest: *digest,
        };
        assert!(reconfirm_variant(&v, &a, &full, &stored).is_ok());
        let scope_only = GuidedConfirmation {
            identity: false,
            ..full.clone()
        };
        assert_eq!(
            reconfirm_variant(&v, &a, &scope_only, &stored),
            Err(RefuseReason::ConfirmationRequired)
        );
        let (ov, oa) = {
            let v = validate_variant(&obs("wow_other", "_test_"));
            let VariantVerdict::Unfamiliar(id) = &v else {
                panic!()
            };
            let a = assess_layout(id, true, &[], &OtherVariants::default());
            (v, a)
        };
        let LayoutAssessment::Supported { digest: od, .. } = &oa else {
            panic!()
        };
        let c = GuidedConfirmation {
            identity: true,
            scope: true,
            digest: *od,
        };
        assert_eq!(
            reconfirm_variant(&ov, &oa, &c, &stored),
            Err(RefuseReason::VariantChanged)
        );
    }

    #[test]
    fn join_requires_equal_product() {
        let k = testkit::confirmed_forever();
        assert!(check_join(&k, "wowf").is_ok());
        let e = check_join(&k, "wow_classic_era").unwrap_err();
        assert_eq!(e.group_product, "wow_classic_era");
        assert_eq!(e.local_product, "wowf");
        assert!(e.to_string().contains("wow_classic_era"));
        assert!(check_join(&k, "WOWF").is_err());
    }

    #[test]
    fn identity_label_is_canonical_not_classic_era() {
        let VariantVerdict::Recognized(id) = validate_variant(&testkit::variant_obs_forever())
        else {
            panic!()
        };
        assert_eq!(id.display_label(), "World of Warcraft: Forever (wowf)");
        let VariantVerdict::Unfamiliar(u) = validate_variant(&obs("wow_test", "_test_")) else {
            panic!()
        };
        assert_eq!(u.display_label(), "Unfamiliar game version (wow_test)");
    }

    #[test]
    fn consent_disclosures_cover_scope_mapping_replacement_removal_and_recovery() {
        let VariantVerdict::Unfamiliar(id) = validate_variant(&obs("wow_test", "_test_")) else {
            panic!()
        };
        let text = consent_disclosures(&ConsentContext {
            identity: &id,
            mapping_id: PLAIN_MAPPING_ID,
            install_root: "C:\\Synthetic\\Root",
            addons_path: "C:\\Synthetic\\Root\\_test_\\Interface\\AddOns",
        })
        .join("\n");
        for needle in [
            "C:\\Synthetic\\Root\\_test_\\Interface\\AddOns",
            "validated-plain/1",
            "install newer versions",
            "replacing the folder here, automatically",
            "never treated as removed",
            "C:\\Synthetic\\Root\\_test_\\.wowsync\\recovery",
            "Settings (WTF) are not synchronized",
            "Unfamiliar game version (wow_test)",
        ] {
            assert!(text.contains(needle), "missing {needle:?} in:\n{text}");
        }
    }

    #[test]
    fn consent_predicate_requires_scope_and_guided_identity() {
        use ConsentAction::{Enroll, Reconfirm};
        let d = Sha256([1; 32]);
        let c = |identity, scope, digest| GuidedConfirmation {
            identity,
            scope,
            digest,
        };
        // Recognized client: scope consent is required even though identity is not (mi-1).
        assert_eq!(
            consent_gap(Enroll, false, &c(true, false, d), Some(&d)),
            Some(ConsentGap::Scope)
        );
        assert_eq!(
            consent_gap(Enroll, false, &c(false, true, d), Some(&d)),
            None
        );
        // Unfamiliar client: both, bound to the current preview.
        assert_eq!(
            consent_gap(Enroll, true, &c(false, true, d), Some(&d)),
            Some(ConsentGap::Identity)
        );
        assert_eq!(
            consent_gap(Enroll, true, &c(true, true, Sha256([2; 32])), Some(&d)),
            Some(ConsentGap::Stale)
        );
        assert_eq!(consent_gap(Enroll, true, &c(true, true, d), Some(&d)), None);
        // Reconfirmation always needs both boxes, also for a recognized client.
        assert_eq!(
            consent_gap(Reconfirm, false, &c(false, true, d), Some(&d)),
            Some(ConsentGap::Identity)
        );
        assert_eq!(
            consent_gap(Reconfirm, false, &c(true, false, d), Some(&d)),
            Some(ConsentGap::Scope)
        );
        assert_eq!(
            consent_gap(Reconfirm, false, &c(true, true, d), Some(&d)),
            None
        );
        assert_eq!(
            RefuseReason::from(ConsentGap::Stale),
            RefuseReason::StaleConfirmation
        );
    }
}
