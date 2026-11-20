//! Diagnostics export (ops.md §5, contracts.md §7, A-33). Pure and fail closed: the text is
//! rendered from an allow-list of typed fields, then scanned for private data; any hit refuses the
//! whole export. No source string (version label, `.toc` text, record title, OS or provider message,
//! display name) is ever copied into the output.

use std::fmt::Write as _;
use unicode_normalization::UnicodeNormalization;

/// Technical caps (contracts.md §8).
pub const MAX_EXPORT_BYTES: usize = 256 * 1024;
pub const MAX_ACTIVITY: usize = 200;
pub const MAX_ADDONS: usize = 500;
pub const MAX_COMPUTERS: usize = 200;
pub const MAX_VARIANTS: usize = 64;
pub const MAX_HOLDS: usize = 200;
pub const MAX_ERRORS: usize = 200;
pub const MAX_LINE_BYTES: usize = 300;
/// Room kept free for the "more omitted" markers.
const TAIL_RESERVE: usize = 2048;
/// Redaction strings shorter than this are not matched: a one or two character name would
/// refuse every export.
pub const MIN_CONTEXT_CHARS: usize = 3;

pub const FORMAT_LINE: &str = "wowsync-diagnostics/1";

/// The "never included" list shown on the export sheet (§5.4).
pub const NEVER_INCLUDED: &[&str] = &[
    "Saved settings (WTF) contents, add-on files or Lua",
    ".toc text, titles, notes and version labels",
    "Account, realm or character names",
    "Your user name, SIDs or profile paths",
    "Full or partial file paths",
    "Display names and computer, enrollment and group ids",
    "Credentials (none exist)",
    "Memory dumps and arbitrary error text",
    "File fingerprints, unless you turn them on",
];

/// Shown on the preview sheet and when a save path is refused.
pub const SHARED_FOLDER_WARNING: &str = "Anyone you send this to can read it. Do not save it in your shared Dropbox folder: everyone in the group can read that folder.";

/// GUI text for a refused export.
pub const REFUSED_TEXT: &str = "WoW Sync could not make a safe export. Nothing was saved.";

/// `^[A-Za-z0-9_.\- ]{1,64}$`: the only folder names that are ever exported (§5.1).
pub fn folder_name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b' '))
}

/// Version information leaves the machine only as a class (contracts.md §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionClass {
    Numeric(u8),
    Date,
    Text,
    None,
}

impl VersionClass {
    pub fn classify(label: &str) -> VersionClass {
        let s = label.trim();
        if s.is_empty() {
            return VersionClass::None;
        }
        if is_date(s) {
            return VersionClass::Date;
        }
        let digits = s.strip_prefix(['v', 'V']).unwrap_or(s);
        let parts: Vec<&str> = digits.split('.').collect();
        if parts.len() <= 8
            && parts
                .iter()
                .all(|p| !p.is_empty() && p.len() <= 9 && p.bytes().all(|b| b.is_ascii_digit()))
        {
            return VersionClass::Numeric(parts.len() as u8);
        }
        VersionClass::Text
    }
}

impl std::fmt::Display for VersionClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionClass::Numeric(n) => write!(f, "numeric({n})"),
            VersionClass::Date => f.write_str("date"),
            VersionClass::Text => f.write_str("text"),
            VersionClass::None => f.write_str("none"),
        }
    }
}

fn is_date(s: &str) -> bool {
    let b = s.as_bytes();
    let (y, m, d) = match b.len() {
        8 if b.iter().all(u8::is_ascii_digit) => (&s[0..4], &s[4..6], &s[6..8]),
        10 if matches!(b[4], b'-' | b'.' | b'/') && b[7] == b[4] => (&s[0..4], &s[5..7], &s[8..10]),
        _ => return false,
    };
    let num = |t: &str| {
        t.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| t.parse::<u32>().ok())
            .flatten()
    };
    matches!((num(y), num(m), num(d)), (Some(y), Some(m), Some(d))
        if (1900..=2199).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d))
}

macro_rules! label_enum {
    ($(#[$m:meta])* $name:ident { $($v:ident => $l:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name { $($v),+ }
        impl $name {
            pub fn label(self) -> &'static str {
                match self { $($name::$v => $l),+ }
            }
        }
    };
}

label_enum!(OsFamily { Windows => "windows", MacOs => "macos", Other => "other" });
label_enum!(CpuArch { X64 => "x64", Arm64 => "arm64", Other => "other" });
label_enum!(Tri { Yes => "yes", No => "no", Unknown => "unknown" });
label_enum!(VariantKind {
    Retail => "retail", ClassicEra => "classic-era", Forever => "forever", Other => "other"
});
label_enum!(IdentityStatus {
    Verified => "verified", Unverified => "unverified", Blocked => "blocked"
});
label_enum!(EnrollmentState {
    Active => "active", Joining => "joining", Held => "held", Retired => "retired"
});
label_enum!(ReportState { Current => "current", Stale => "stale", NeverReported => "never" });
label_enum!(AddonStatus {
    Ok => "ok", Waiting => "waiting", NeedsDecision => "needs-decision",
    NeedsHelp => "needs-help", Removed => "removed", Other => "other"
});
label_enum!(ReasonKind {
    None => "none", Ambiguous => "ambiguous", PartialCandidate => "partial-candidate",
    ChoicesDiffer => "choices-differ", RollbackTargetsDiffer => "rollback-targets-differ",
    RecoveredUnavailable => "recovered-unavailable", LookalikeOfRejected => "lookalike-of-rejected",
    NewerAfterRollback => "newer-after-rollback",
    NewVersionAfterDecision => "new-version-after-decision",
    UnusableMetadata => "unusable-metadata", DifferentTocMapping => "different-toc-mapping",
    UnvalidatedTocMapping => "unvalidated-toc-mapping", RemovalUnclear => "removal-unclear",
    ReinstallObserved => "reinstall-observed", Other => "other"
});
label_enum!(HoldKind { None => "none", Addons => "addons", AllApplies => "all-applies" });
label_enum!(PlanBlockKind {
    UnresolvedJournal => "unresolved-journal", GameRunning => "game-running",
    ActivityUnknown => "activity-unknown", Paused => "paused", Retired => "retired",
    RetirementUnresolved => "retirement-unresolved", Other => "other"
});
label_enum!(JournalState {
    Pending => "pending", Recovered => "recovered", NeedsHelp => "needs-help", Other => "other"
});
label_enum!(ActivityKind {
    Applied => "applied", Decision => "decision", Retirement => "retirement", Paused => "paused",
    Resumed => "resumed", Recovery => "recovery", UpdateCheckFailed => "update-check-failed",
    ExportRefused => "export-refused", Error => "error", Other => "other"
});
label_enum!(ErrorKind {
    Io => "Io", Provider => "Provider", Parse => "Parse", Validation => "Validation",
    Network => "Network", Other => "Other"
});
label_enum!(IoKind {
    NotFound => "NotFound", PermissionDenied => "PermissionDenied",
    AlreadyExists => "AlreadyExists", InUse => "InUse", Other => "Other"
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Toggles {
    pub update_check: bool,
    pub notify_summaries: bool,
    pub notify_needs_me: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VariantRow {
    pub kind: VariantKind,
    pub identity: IdentityStatus,
    pub addon_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerRow {
    pub is_this_computer: bool,
    pub enrollment: EnrollmentState,
    pub report: ReportState,
    pub last_report_age_hours: Option<u32>,
    pub watermark_seq: u64,
    /// Where the alias would have come from. Never read: aliases are assigned by position.
    pub display_name: String,
}

/// Raw source strings the caller may hold for an add-on. Only `version_label` is read, to
/// compute a [`VersionClass`]; the rest exist so callers hand over their data as is and so tests
/// can prove none of it reaches the export.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceText {
    pub version_label: String,
    pub toc_title: String,
    pub toc_notes: String,
    pub record_title: String,
    pub rejection_version_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonRow {
    /// Exported only when the option is on and it passes [`folder_name_ok`].
    pub folder_name: String,
    pub status: AddonStatus,
    pub candidate_count: u32,
    pub reason: ReasonKind,
    pub hold: HoldKind,
    pub source_text: SourceText,
    pub fingerprint: Option<[u8; 32]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityRow {
    pub at_unix_secs: i64,
    pub kind: ActivityKind,
    /// Index into [`DiagnosticsInput::computers`].
    pub computer: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorRow {
    pub kind: ErrorKind,
    pub io: Option<IoKind>,
    pub os_code: Option<i32>,
    /// OS or provider message text. Never read.
    pub message: String,
}

/// Everything the export is built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsInput {
    pub app_version: &'static str,
    /// At most eight hex digits, so no fingerprint-sized hex run appears.
    pub build_commit: &'static str,
    pub toggles: Toggles,
    pub schema_majors: Vec<(&'static str, u32)>,
    pub os: OsFamily,
    pub os_version: Option<(u32, u32, u32)>,
    pub arch: CpuArch,
    pub process_memory_mib: Option<u64>,
    pub wow_running: Tri,
    pub variants: Vec<VariantRow>,
    pub computers: Vec<ComputerRow>,
    pub addons: Vec<AddonRow>,
    pub hold_scope: HoldKind,
    pub plan_blocks: Vec<PlanBlockKind>,
    pub journals: Vec<JournalState>,
    pub activity: Vec<ActivityRow>,
    pub errors: Vec<ErrorRow>,
}

/// Strings that must not appear in the output (§5.3). Matching ignores case and Unicode
/// normalization form, and treats `\` and `/` alike.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RedactionContext {
    pub home_dir: String,
    pub os_user_name: String,
    pub computer_names: Vec<String>,
    pub display_names: Vec<String>,
    pub install_roots: Vec<String>,
    pub shared_root: String,
    pub dropbox_root: String,
    pub local_state_root: String,
    pub wtf_account_names: Vec<String>,
    pub wtf_realm_names: Vec<String>,
    pub wtf_character_names: Vec<String>,
}

impl RedactionContext {
    fn strings(&self) -> Vec<&str> {
        let mut v: Vec<&str> = vec![
            &self.home_dir,
            &self.os_user_name,
            &self.shared_root,
            &self.dropbox_root,
            &self.local_state_root,
        ];
        for list in [
            &self.computer_names,
            &self.display_names,
            &self.install_roots,
            &self.wtf_account_names,
            &self.wtf_realm_names,
            &self.wtf_character_names,
        ] {
            v.extend(list.iter().map(String::as_str));
        }
        v
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportOptions {
    /// Q-64 default: on.
    pub include_folder_names: bool,
    /// Default: off.
    pub include_fingerprints: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        ExportOptions {
            include_folder_names: true,
            include_fingerprints: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrivateKind {
    ContextString,
    Sid,
    WindowsProfilePath,
    MacProfilePath,
    HomePath,
    UncPath,
    Email,
    WtfAccountPath,
    AccountSuffix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportRefused {
    PrivateDataFound(PrivateKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    /// The exact bytes to preview and save.
    pub text: String,
    pub replaced: Replaced,
}

/// What was replaced, for "Replaced: 4 folder paths, 3 computer names".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Replaced {
    pub folder_paths: u32,
    pub computer_names: u32,
    pub addon_names: u32,
}

impl Replaced {
    pub fn summary(&self) -> String {
        format!(
            "Replaced: {} folder paths, {} computer names, {} add-on names",
            self.folder_paths, self.computer_names, self.addon_names
        )
    }
}

struct Writer {
    buf: String,
    limit: usize,
}

impl Writer {
    fn line(&mut self, s: &str) -> bool {
        let s = clip(s);
        if self.buf.len() + s.len() + 1 > self.limit.saturating_sub(TAIL_RESERVE) {
            return false;
        }
        self.buf.push_str(s);
        self.buf.push('\n');
        true
    }

    fn reserved(&mut self, s: &str) {
        self.buf.push_str(clip(s));
        self.buf.push('\n');
    }

    fn header(&mut self, name: &str) {
        self.reserved(&format!("[{name}]"));
    }

    /// Writes at most `cap` rows within the byte bound, then the omitted marker.
    fn rows<T>(&mut self, items: &[T], cap: usize, mut render: impl FnMut(usize, &T) -> String) {
        let mut written = 0;
        for (i, it) in items.iter().enumerate() {
            if i >= cap || !self.line(&render(i, it)) {
                break;
            }
            written += 1;
        }
        if written < items.len() {
            self.reserved(&format!("(… {} more omitted)", items.len() - written));
        }
    }
}

fn clip(s: &str) -> &str {
    if s.len() <= MAX_LINE_BYTES {
        return s;
    }
    let mut end = MAX_LINE_BYTES;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

fn on_off(b: bool) -> &'static str {
    if b {
        "on"
    } else {
        "off"
    }
}

fn computer_alias(i: usize, this: bool) -> String {
    if this {
        return "This computer".to_string();
    }
    let mut n = i;
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    format!("Computer {s}")
}

fn utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// Builds the export text, or refuses (fail closed).
pub fn build(
    input: &DiagnosticsInput,
    ctx: &RedactionContext,
    options: ExportOptions,
) -> Result<Export, ExportRefused> {
    build_inner(input, ctx, options, MAX_EXPORT_BYTES, None)
}

/// `inject` writes an extra line into the About section: tests use it to force a string into a
/// typed position and prove the output scan catches it.
fn build_inner(
    input: &DiagnosticsInput,
    ctx: &RedactionContext,
    options: ExportOptions,
    limit: usize,
    inject: Option<&str>,
) -> Result<Export, ExportRefused> {
    let mut w = Writer {
        buf: String::new(),
        limit,
    };
    w.reserved(FORMAT_LINE);

    w.header("About");
    w.reserved(&format!("app_version: {}", input.app_version));
    w.reserved(&format!("build_commit: {}", input.build_commit));
    w.reserved(&format!(
        "check_for_updates: {}",
        on_off(input.toggles.update_check)
    ));
    w.reserved(&format!(
        "notify_summaries: {}",
        on_off(input.toggles.notify_summaries)
    ));
    w.reserved(&format!(
        "notify_needs_me: {}",
        on_off(input.toggles.notify_needs_me)
    ));
    let majors: Vec<String> = input
        .schema_majors
        .iter()
        .map(|(n, v)| format!("{n}={v}"))
        .collect();
    w.reserved(&format!("schema_majors: {}", majors.join(", ")));
    if let Some(s) = inject {
        w.reserved(&format!("injected: {s}"));
    }

    w.header("Environment");
    let ver = input
        .os_version
        .map(|(a, b, c)| format!("{a}.{b}.{c}"))
        .unwrap_or_else(|| "unknown".into());
    w.reserved(&format!("os: {} {ver}", input.os.label()));
    w.reserved(&format!("arch: {}", input.arch.label()));
    let mem = input
        .process_memory_mib
        .map(|m| format!("{m} MiB"))
        .unwrap_or_else(|| "unknown".into());
    w.reserved(&format!("process_memory: {mem}"));
    w.reserved(&format!("wow_running: {}", input.wow_running.label()));

    w.header("Settings");
    w.reserved(&format!(
        "include_folder_names: {}",
        on_off(options.include_folder_names)
    ));
    w.reserved(&format!(
        "include_fingerprints: {}",
        on_off(options.include_fingerprints)
    ));

    w.header("Variants");
    w.rows(&input.variants, MAX_VARIANTS, |i, v| {
        format!(
            "<install-{}>: kind={} identity={} addons={}",
            i + 1,
            v.kind.label(),
            v.identity.label(),
            v.addon_count
        )
    });

    w.header("Computers");
    let mut others = 0usize;
    let aliases: Vec<String> = input
        .computers
        .iter()
        .map(|c| {
            let a = computer_alias(others, c.is_this_computer);
            if !c.is_this_computer {
                others += 1;
            }
            a
        })
        .collect();
    w.rows(&input.computers, MAX_COMPUTERS, |i, c| {
        let age = c
            .last_report_age_hours
            .map(|h| format!("{h}h"))
            .unwrap_or_else(|| "none".into());
        format!(
            "{}: enrollment={} retired={} report={} last_report_age={age} watermark_seq={}",
            aliases[i],
            c.enrollment.label(),
            on_off(c.enrollment == EnrollmentState::Retired),
            c.report.label(),
            c.watermark_seq
        )
    });

    w.header("Addons");
    let mut unnamed = 0u32;
    let mut rows = Vec::new();
    for (i, a) in input.addons.iter().enumerate().take(MAX_ADDONS) {
        let named = options.include_folder_names && folder_name_ok(&a.folder_name);
        if !named {
            unnamed += 1;
        }
        let label = if named {
            a.folder_name.clone()
        } else {
            format!("Add-on {}", i + 1)
        };
        let mut line = format!(
            "{label}: status={} candidates={} version={} reason={} hold={}",
            a.status.label(),
            a.candidate_count,
            VersionClass::classify(&a.source_text.version_label),
            a.reason.label(),
            a.hold.label()
        );
        if let (true, Some(fp)) = (options.include_fingerprints, a.fingerprint) {
            line.push_str(" fingerprint=");
            for b in &fp[..6] {
                let _ = write!(line, "{b:02x}");
            }
        }
        rows.push(line);
    }
    unnamed += input.addons.len().saturating_sub(MAX_ADDONS) as u32;
    w.rows(&rows, MAX_ADDONS, |_, l| l.clone());
    if rows.len() < input.addons.len() {
        w.reserved(&format!(
            "(… {} more omitted)",
            input.addons.len() - rows.len()
        ));
    }

    w.header("Holds");
    w.reserved(&format!("hold_scope: {}", input.hold_scope.label()));
    w.rows(&input.plan_blocks, MAX_HOLDS, |_, b| {
        format!("plan_block: {}", b.label())
    });
    w.rows(&input.journals, MAX_HOLDS, |_, j| {
        format!("journal: {}", j.label())
    });

    w.header("Recent activity");
    // Newest entries, as many as the cap allows.
    let skip = input.activity.len().saturating_sub(MAX_ACTIVITY);
    if skip > 0 {
        w.reserved(&format!("(… {skip} older omitted)"));
    }
    w.rows(&input.activity[skip..], MAX_ACTIVITY, |_, a| {
        let who = a
            .computer
            .and_then(|c| aliases.get(c))
            .map(|n| format!(" computer={n}"))
            .unwrap_or_default();
        format!("{} kind={}{who}", utc(a.at_unix_secs), a.kind.label())
    });

    w.header("Errors");
    w.rows(&input.errors, MAX_ERRORS, |_, e| match (e.io, e.os_code) {
        (Some(io), Some(code)) => format!("{}({}, {code})", e.kind.label(), io.label()),
        (Some(io), None) => format!("{}({})", e.kind.label(), io.label()),
        (None, Some(code)) => format!("{}({code})", e.kind.label()),
        (None, None) => e.kind.label().to_string(),
    });

    if let Some(kind) = find_private(&w.buf, ctx) {
        return Err(ExportRefused::PrivateDataFound(kind));
    }
    Ok(Export {
        text: w.buf,
        replaced: Replaced {
            folder_paths: input.variants.len() as u32,
            computer_names: input.computers.len() as u32,
            addon_names: unnamed,
        },
    })
}

fn fold(s: &str) -> String {
    s.nfc().collect::<String>().to_lowercase()
}

/// The output scan: context strings first, then the patterns, on NFC-normalized, case-folded text.
fn find_private(text: &str, ctx: &RedactionContext) -> Option<PrivateKind> {
    let folded = fold(text);
    let unix = folded.replace('\\', "/");
    for s in ctx.strings() {
        let f = fold(s);
        if f.chars().count() < MIN_CONTEXT_CHARS {
            continue;
        }
        if folded.contains(&f) || unix.contains(&f.replace('\\', "/")) {
            return Some(PrivateKind::ContextString);
        }
    }
    if unix.contains("s-1-5-21-") {
        return Some(PrivateKind::Sid);
    }
    let b = unix.as_bytes();
    for (i, _) in unix.match_indices(":/users/") {
        if i > 0 && b[i - 1].is_ascii_alphabetic() {
            return Some(PrivateKind::WindowsProfilePath);
        }
    }
    if unix.contains("/users/") {
        return Some(PrivateKind::MacProfilePath);
    }
    if unix.contains("~/") {
        return Some(PrivateKind::HomePath);
    }
    if folded.contains("\\\\") {
        return Some(PrivateKind::UncPath);
    }
    if has_email(&unix) {
        return Some(PrivateKind::Email);
    }
    if unix.contains("wtf/account/") {
        return Some(PrivateKind::WtfAccountPath);
    }
    if has_account_suffix(&unix) {
        return Some(PrivateKind::AccountSuffix);
    }
    None
}

fn has_email(s: &str) -> bool {
    let b = s.as_bytes();
    s.match_indices('@').any(|(i, _)| {
        let before = i > 0 && b[i - 1].is_ascii_alphanumeric();
        let rest = &s[i + 1..];
        let host_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
            .unwrap_or(rest.len());
        let host = &rest[..host_end];
        before && host.find('.').is_some_and(|d| d > 0 && d + 1 < host.len())
    })
}

fn has_account_suffix(s: &str) -> bool {
    let b = s.as_bytes();
    s.match_indices('#')
        .any(|(i, _)| b.get(i + 1).is_some_and(u8::is_ascii_digit))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "PLANTED_SECRET_DO_NOT_EXPORT";

    fn base() -> DiagnosticsInput {
        DiagnosticsInput {
            app_version: "0.1.0",
            build_commit: "abc12345",
            toggles: Toggles {
                update_check: true,
                notify_summaries: true,
                notify_needs_me: false,
            },
            schema_majors: vec![("authority", 1), ("journal", 2)],
            os: OsFamily::Windows,
            os_version: Some((10, 0, 26100)),
            arch: CpuArch::X64,
            process_memory_mib: Some(42),
            wow_running: Tri::No,
            variants: vec![VariantRow {
                kind: VariantKind::Forever,
                identity: IdentityStatus::Verified,
                addon_count: 2,
            }],
            computers: vec![
                computer(true, "Laptop"),
                computer(false, "Den PC"),
                computer(false, "Spare"),
            ],
            addons: vec![
                addon("WeakAuras", Some([0xab; 32])),
                addon("Deadly Boss Mods", Some([0xcd; 32])),
            ],
            hold_scope: HoldKind::Addons,
            plan_blocks: vec![PlanBlockKind::GameRunning],
            journals: vec![JournalState::Recovered],
            activity: vec![ActivityRow {
                at_unix_secs: 1_700_000_000,
                kind: ActivityKind::Applied,
                computer: Some(1),
            }],
            errors: vec![ErrorRow {
                kind: ErrorKind::Io,
                io: Some(IoKind::PermissionDenied),
                os_code: Some(5),
                message: String::new(),
            }],
        }
    }

    fn computer(this: bool, name: &str) -> ComputerRow {
        ComputerRow {
            is_this_computer: this,
            enrollment: EnrollmentState::Active,
            report: ReportState::Current,
            last_report_age_hours: Some(3),
            watermark_seq: 12,
            display_name: name.into(),
        }
    }

    fn addon(name: &str, fp: Option<[u8; 32]>) -> AddonRow {
        AddonRow {
            folder_name: name.into(),
            status: AddonStatus::Ok,
            candidate_count: 1,
            reason: ReasonKind::None,
            hold: HoldKind::None,
            source_text: SourceText {
                version_label: "1.2.3".into(),
                ..SourceText::default()
            },
            fingerprint: fp,
        }
    }

    fn off() -> ExportOptions {
        ExportOptions {
            include_folder_names: false,
            include_fingerprints: false,
        }
    }

    fn variants_of(s: &str) -> Vec<String> {
        let nfc = s.nfc().collect::<String>();
        vec![
            s.to_string(),
            s.to_lowercase(),
            s.to_uppercase(),
            nfc.to_lowercase(),
        ]
    }

    fn assert_absent(text: &str, secret: &str, why: &str) {
        let hay = fold(text);
        for v in variants_of(secret) {
            assert!(!hay.contains(&fold(&v)), "{why}: {v} leaked into\n{text}");
        }
    }

    #[test]
    fn export_has_the_documented_shape() {
        let e = build(
            &base(),
            &RedactionContext::default(),
            ExportOptions::default(),
        )
        .unwrap();
        let t = &e.text;
        assert_eq!(t.lines().next(), Some(FORMAT_LINE));
        let order = [
            "[About]",
            "[Environment]",
            "[Settings]",
            "[Variants]",
            "[Computers]",
            "[Addons]",
            "[Holds]",
            "[Recent activity]",
            "[Errors]",
        ];
        let at: Vec<usize> = order.iter().map(|h| t.find(h).expect(h)).collect();
        assert!(at.windows(2).all(|p| p[0] < p[1]), "fixed section order");
        assert!(t.contains("<install-1>: kind=forever identity=verified addons=2"));
        assert!(t.contains("This computer: enrollment=active"));
        assert!(t.contains("Computer A: ") && t.contains("Computer B: "));
        assert!(t.contains("2023-11-14T22:13Z kind=applied computer=Computer A"));
        assert!(t.contains("Io(PermissionDenied, 5)"));
        assert!(t.contains("WeakAuras: status=ok candidates=1 version=numeric(3)"));
        assert_eq!(
            e.replaced.summary(),
            "Replaced: 1 folder paths, 3 computer names, 0 add-on names"
        );
        assert!(!t.contains("Laptop") && !t.contains("Den PC"));
    }

    #[test]
    fn planted_secret_in_every_free_string_source_never_reaches_the_export() {
        for opts in [off(), ExportOptions::default()] {
            let mut i = base();
            let nfd = "PLANTED_SECRET_cafe\u{301}";
            i.addons[0].source_text = SourceText {
                version_label: format!("{SECRET} {nfd}"),
                toc_title: SECRET.into(),
                toc_notes: SECRET.to_lowercase(),
                record_title: format!("{SECRET}{nfd}"),
                rejection_version_label: SECRET.into(),
            };
            i.computers[1].display_name = SECRET.into();
            i.computers[2].display_name = nfd.into();
            i.errors.push(ErrorRow {
                kind: ErrorKind::Provider,
                io: None,
                os_code: None,
                message: format!("provider said {SECRET} at C:\\Users\\x"),
            });
            i.errors.push(ErrorRow {
                kind: ErrorKind::Io,
                io: Some(IoKind::Other),
                os_code: Some(2),
                message: format!("os said {SECRET}"),
            });
            let e = build(&i, &RedactionContext::default(), opts).unwrap();
            assert_absent(&e.text, SECRET, "free-string source");
            assert_absent(&e.text, nfd, "NFD variant");
            assert!(e.text.contains("version=text"), "label became a class only");
        }
    }

    #[test]
    fn a_planted_folder_name_appears_only_as_a_folder_name_that_passes_the_pattern() {
        let mut i = base();
        i.addons[0].folder_name = SECRET.into();
        i.addons[1].folder_name = format!("{SECRET}/x");
        let on = build(&i, &RedactionContext::default(), ExportOptions::default()).unwrap();
        assert_eq!(on.text.matches(SECRET).count(), 1, "{}", on.text);
        assert!(on.text.contains(&format!("{SECRET}: status=ok")));
        assert!(on.text.contains("Add-on 2: status=ok"));
        let off = build(&i, &RedactionContext::default(), off()).unwrap();
        assert_absent(&off.text, SECRET, "names off");
        assert!(off.text.contains("Add-on 1: ") && off.text.contains("Add-on 2: "));
        assert_eq!(off.replaced.addon_names, 2);
    }

    #[test]
    fn planted_private_strings_do_not_appear_in_a_successful_export() {
        let ctx = RedactionContext {
            home_dir: r"C:\Users\zz_user_PLANT".into(),
            os_user_name: "zz_user_PLANT".into(),
            computer_names: vec!["PLANT-PC".into()],
            display_names: vec!["Plant Display".into()],
            install_roots: vec![r"D:\Games\WoW".into()],
            shared_root: r"D:\Dropbox\Gaming Sync".into(),
            dropbox_root: r"D:\Dropbox".into(),
            local_state_root: r"C:\Users\zz_user_PLANT\AppData\Local\WoWSync".into(),
            wtf_account_names: vec!["PLANTACCT#1".into()],
            wtf_realm_names: vec!["PlantRealm".into()],
            wtf_character_names: vec!["Plantchar".into()],
        };
        let mut i = base();
        i.computers[0].display_name = "PLANT-PC".into();
        i.addons[0].source_text.toc_title = "Plantchar of PlantRealm".into();
        i.errors[0].message =
            r"Access denied (os error 5): C:\Users\zz_user_PLANT\AppData\x S-1-5-21-1-2-3-1001"
                .into();
        let e = build(&i, &ctx, ExportOptions::default()).unwrap();
        for p in [
            "zz_user_PLANT",
            r"C:\Users\zz_user_PLANT",
            "PLANT-PC",
            "PLANTACCT#1",
            "PlantRealm",
            "Plantchar",
            "S-1-5-21-1-2-3-1001",
            r"D:\Dropbox",
            "Access denied",
        ] {
            assert_absent(&e.text, p, "private string");
        }
    }

    #[test]
    fn a_string_forced_into_a_typed_field_is_refused_whole() {
        let ctx = RedactionContext {
            os_user_name: "zz_user_PLANT".into(),
            ..RedactionContext::default()
        };
        let forced = |s: &str| {
            build_inner(
                &base(),
                &ctx,
                ExportOptions::default(),
                MAX_EXPORT_BYTES,
                Some(s),
            )
        };
        assert_eq!(
            forced("ZZ_USER_plant"),
            Err(ExportRefused::PrivateDataFound(PrivateKind::ContextString)),
            "case-folded"
        );
        let nfd_ctx = RedactionContext {
            os_user_name: "caf\u{e9}_user".into(),
            ..RedactionContext::default()
        };
        assert_eq!(
            build_inner(
                &base(),
                &nfd_ctx,
                ExportOptions::default(),
                MAX_EXPORT_BYTES,
                Some("CAFE\u{301}_USER")
            ),
            Err(ExportRefused::PrivateDataFound(PrivateKind::ContextString)),
            "NFC-normalized"
        );
        let c = RedactionContext::default();
        for (s, kind) in [
            ("S-1-5-21-111-222-333-1001", PrivateKind::Sid),
            (r"c:\Users\Bob\x", PrivateKind::WindowsProfilePath),
            ("/Users/bob/Library", PrivateKind::MacProfilePath),
            ("~/Documents", PrivateKind::HomePath),
            (r"\\server\share\x", PrivateKind::UncPath),
            ("bob@example.com", PrivateKind::Email),
            (r"WTF\Account\NAME", PrivateKind::WtfAccountPath),
            ("Char#1234", PrivateKind::AccountSuffix),
        ] {
            let r = build_inner(
                &base(),
                &c,
                ExportOptions::default(),
                MAX_EXPORT_BYTES,
                Some(s),
            );
            assert_eq!(r, Err(ExportRefused::PrivateDataFound(kind)), "{s}");
        }
        // Plain text of the same shape is fine.
        for s in ["version 1.2.3 at example", "50 percent", "a@b"] {
            assert!(
                build_inner(
                    &base(),
                    &c,
                    ExportOptions::default(),
                    MAX_EXPORT_BYTES,
                    Some(s)
                )
                .is_ok(),
                "{s}"
            );
        }
        // A folder name equal to a context string is refused, not silently exported.
        let mut i = base();
        i.addons[0].folder_name = "Plantchar".into();
        let named = RedactionContext {
            wtf_character_names: vec!["Plantchar".into()],
            ..RedactionContext::default()
        };
        assert_eq!(
            build(&i, &named, ExportOptions::default()),
            Err(ExportRefused::PrivateDataFound(PrivateKind::ContextString))
        );
        assert!(
            build(&i, &named, off()).is_ok(),
            "names off: replaced by a number"
        );
    }

    #[test]
    fn the_export_is_bounded_with_omitted_markers() {
        let mut i = base();
        i.activity = (0..10_000)
            .map(|n| ActivityRow {
                at_unix_secs: n,
                kind: ActivityKind::Applied,
                computer: Some(2),
            })
            .collect();
        i.addons = (0..5_000)
            .map(|n| addon(&format!("Addon{n}"), Some([1; 32])))
            .collect();
        i.computers = (0..1_000).map(|n| computer(n == 0, "x")).collect();
        let opts = ExportOptions {
            include_folder_names: true,
            include_fingerprints: true,
        };
        let e = build(&i, &RedactionContext::default(), opts).unwrap();
        assert!(e.text.len() <= MAX_EXPORT_BYTES);
        assert!(e.text.contains("(… 4500 more omitted)"));
        assert!(e.text.contains("(… 9800 older omitted)"));
        assert!(e.text.contains("(… 800 more omitted)"));
        assert_eq!(
            e.text
                .lines()
                .filter(|l| l.contains("kind=applied"))
                .count(),
            200
        );
        assert!(e.text.lines().all(|l| l.len() <= MAX_LINE_BYTES));

        // The byte bound itself: a tiny limit stops the rows and never overruns.
        let small = build_inner(&i, &RedactionContext::default(), opts, 6 * 1024, None).unwrap();
        assert!(small.text.len() <= 6 * 1024, "{}", small.text.len());
        assert!(small.text.contains("more omitted"));
        assert!(
            small.text.contains("[Errors]"),
            "later sections still written"
        );
    }

    #[test]
    fn source_file_contents_never_appear() {
        let mut i = base();
        for (n, marker) in ["WTF_MARKER_Q1", "LUA_MARKER_Q2", "TOC_MARKER_Q3"]
            .iter()
            .enumerate()
        {
            i.addons[0].source_text.toc_title = marker.to_string();
            i.addons[1].source_text.toc_notes =
                format!("## Notes: {marker}\nlocal x = \"{marker}\"");
            i.errors[0].message = format!("SavedVariables {marker} {n}");
            let e = build(&i, &RedactionContext::default(), ExportOptions::default()).unwrap();
            assert_absent(&e.text, marker, "file content");
        }
    }

    #[test]
    fn fingerprints_appear_only_when_asked_and_only_as_twelve_hex_digits() {
        let hex_runs = |t: &str| -> Vec<String> {
            let mut runs = Vec::new();
            let mut cur = String::new();
            for c in t.chars().chain(std::iter::once(' ')) {
                if c.is_ascii_hexdigit() && !c.is_ascii_uppercase() {
                    cur.push(c);
                } else {
                    if cur.len() >= 12 {
                        runs.push(cur.clone());
                    }
                    cur.clear();
                }
            }
            runs
        };
        let off = build(
            &base(),
            &RedactionContext::default(),
            ExportOptions::default(),
        )
        .unwrap();
        assert!(hex_runs(&off.text).is_empty(), "{:?}", hex_runs(&off.text));
        assert!(!off.text.contains("fingerprint="));
        let on = build(
            &base(),
            &RedactionContext::default(),
            ExportOptions {
                include_folder_names: true,
                include_fingerprints: true,
            },
        )
        .unwrap();
        assert_eq!(hex_runs(&on.text), vec!["abababababab", "cdcdcdcdcdcd"]);
        let mut none = base();
        none.addons[0].fingerprint = None;
        let on_none = build(
            &none,
            &RedactionContext::default(),
            ExportOptions {
                include_folder_names: true,
                include_fingerprints: true,
            },
        )
        .unwrap();
        assert_eq!(hex_runs(&on_none.text).len(), 1);
    }

    #[test]
    fn version_information_is_only_a_class() {
        for (label, class) in [
            ("1.2.3", "numeric(3)"),
            ("v10.0", "numeric(2)"),
            ("7", "numeric(1)"),
            ("2024-05-01", "date"),
            ("20240501", "date"),
            ("2024.05.01", "date"),
            ("r123-beta", "text"),
            ("1.2.3-alpha", "text"),
            ("2024-13-01", "text"),
            ("1.2.3.4.5.6.7.8.9", "text"),
            ("", "none"),
            ("   ", "none"),
        ] {
            assert_eq!(
                VersionClass::classify(label).to_string(),
                class,
                "{label:?}"
            );
        }
        let mut i = base();
        i.addons[0].source_text.version_label = "9.9.9-PRIVATE_LABEL".into();
        let e = build(&i, &RedactionContext::default(), ExportOptions::default()).unwrap();
        assert!(e.text.contains("version=text"));
        assert_absent(&e.text, "9.9.9-PRIVATE_LABEL", "version label");
        assert_absent(&e.text, "PRIVATE_LABEL", "version label part");
    }

    #[test]
    fn folder_name_pattern_is_exact() {
        for ok in ["WeakAuras", "Deadly Boss Mods", "A_b.c-d", &"x".repeat(64)] {
            assert!(folder_name_ok(ok), "{ok}");
        }
        for bad in [
            "",
            &"x".repeat(65),
            "a/b",
            "a\\b",
            "é",
            "a\nb",
            "a@b",
            "a#1",
            "~x",
        ] {
            assert!(!folder_name_ok(bad), "{bad:?}");
        }
    }

    #[test]
    fn aliases_are_positional_and_never_names() {
        assert_eq!(computer_alias(0, false), "Computer A");
        assert_eq!(computer_alias(25, false), "Computer Z");
        assert_eq!(computer_alias(26, false), "Computer AA");
        assert_eq!(computer_alias(27, false), "Computer AB");
        assert_eq!(computer_alias(5, true), "This computer");
    }
}
