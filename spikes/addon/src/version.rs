//! Version-string classification and the ordering policy under test.
//! Policy (R-05): order only when the scheme proves it; otherwise `Ambiguous`.

use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scheme {
    Missing,
    /// Unsubstituted packager token such as `@project-version@`.
    Token,
    /// Exactly three numeric components, optional pre-release.
    Semver,
    /// 1, 2 or 4+ numeric components, optional pre-release.
    NumericDotted,
    Date,
    /// `v1.2.3-4-gabcdef0`
    GitDescribe,
    /// Plain integer build number.
    Integer,
    /// `r123` revision.
    Revision,
    /// Bare commit hash.
    GitHash,
    /// Word plus numeric version (e.g. Forever 1.60.1); ordered only for the same word.
    Tagged,
    Other,
}

impl Scheme {
    pub fn name(&self) -> &'static str {
        match self {
            Scheme::Missing => "missing",
            Scheme::Token => "unsubstituted-token",
            Scheme::Semver => "semver-like",
            Scheme::NumericDotted => "numeric-dotted",
            Scheme::Date => "date",
            Scheme::GitDescribe => "git-describe",
            Scheme::Integer => "integer-build",
            Scheme::Revision => "revision-rN",
            Scheme::GitHash => "git-hash",
            Scheme::Tagged => "tagged-numeric",
            Scheme::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub scheme: Scheme,
    nums: Vec<u64>,
    /// Pre-release rank: None = final release; lower sorts earlier.
    pre: Option<(u8, u64)>,
    /// git-describe commits since tag.
    ahead: u64,
    tag: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Less,
    Equal,
    Greater,
    Ambiguous(&'static str),
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn parse_nums(core: &str) -> Option<Vec<u64>> {
    core.split('.')
        .map(|p| if all_digits(p) { p.parse().ok() } else { None })
        .collect()
}

fn parse_pre(s: &str) -> Option<(u8, u64)> {
    let l = s.to_ascii_lowercase();
    let l = l.trim_start_matches(['-', '.']);
    let split = l.find(|c: char| c.is_ascii_digit()).unwrap_or(l.len());
    let (word, num) = l.split_at(split);
    let rank = match word.trim_end_matches(['.', '-']) {
        "dev" => 0,
        "alpha" | "a" => 1,
        "beta" | "b" => 2,
        "rc" => 3,
        _ => return None,
    };
    let n = if num.is_empty() {
        0
    } else {
        num.trim_matches('.').parse().ok()?
    };
    Some((rank, n))
}

fn valid_date(y: u64, m: u64, d: u64) -> bool {
    (1990..=2100).contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
}

fn parse_date(s: &str) -> Option<Vec<u64>> {
    if all_digits(s) && s.len() == 8 {
        let (y, m, d) = (
            s[..4].parse().ok()?,
            s[4..6].parse().ok()?,
            s[6..].parse().ok()?,
        );
        return valid_date(y, m, d).then(|| vec![y, m, d]);
    }
    for sep in ['-', '.'] {
        let parts: Vec<&str> = s.split(sep).collect();
        if parts.len() == 3 && parts[0].len() == 4 && parts.iter().all(|p| all_digits(p)) {
            let n: Vec<u64> = parts.iter().map(|p| p.parse().unwrap()).collect();
            if valid_date(n[0], n[1], n[2]) {
                return Some(n);
            }
        }
    }
    None
}

pub fn parse_version(raw: Option<&str>) -> Parsed {
    let mk = |scheme, nums, pre, ahead| Parsed {
        scheme,
        nums,
        pre,
        ahead,
        tag: String::new(),
    };
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return mk(Scheme::Missing, vec![], None, 0);
    };
    if raw.contains('@')
        || raw.contains("${")
        || raw.contains("{{")
        || raw.to_ascii_lowercase().contains("project-version")
    {
        return mk(Scheme::Token, vec![], None, 0);
    }
    if let Some(n) = parse_date(raw) {
        return mk(Scheme::Date, n, None, 0);
    }
    if all_digits(raw) {
        if let Ok(n) = raw.parse() {
            return mk(Scheme::Integer, vec![n], None, 0);
        }
    }
    if let Some(r) = raw.strip_prefix(['r', 'R']) {
        if all_digits(r) {
            if let Ok(n) = r.parse() {
                return mk(Scheme::Revision, vec![n], None, 0);
            }
        }
    }
    if raw.len() >= 7 && raw.len() <= 40 && raw.bytes().all(|b| b.is_ascii_hexdigit()) {
        if raw.bytes().any(|b| b.is_ascii_alphabetic()) && raw.bytes().any(|b| b.is_ascii_digit()) {
            return mk(Scheme::GitHash, vec![], None, 0);
        }
    }
    let s = raw.strip_prefix(['v', 'V']).unwrap_or(raw);
    // git describe: <tag>-<n>-g<hash>
    if let Some(gi) = s.rfind("-g") {
        let (head, hash) = (&s[..gi], &s[gi + 2..]);
        if hash.len() >= 7 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
            if let Some((tag, n)) = head.rsplit_once('-') {
                if all_digits(n) {
                    let inner = parse_version(Some(tag));
                    if matches!(inner.scheme, Scheme::Semver | Scheme::NumericDotted) {
                        return mk(
                            Scheme::GitDescribe,
                            inner.nums,
                            inner.pre,
                            n.parse().unwrap_or(0),
                        );
                    }
                }
            }
        }
    }
    // numeric core with optional -pre / +build / bare pre-release suffix
    let core_end = s
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(s.len());
    let (core, rest) = s.split_at(core_end);
    let core = core.trim_end_matches('.');
    if let Some(nums) = parse_nums(core) {
        let rest = rest.split('+').next().unwrap_or("");
        let pre = if rest.is_empty() {
            Some(None)
        } else {
            parse_pre(rest).map(Some)
        };
        if let Some(pre) = pre {
            let scheme = if nums.len() == 3 {
                Scheme::Semver
            } else {
                Scheme::NumericDotted
            };
            return mk(scheme, nums, pre, 0);
        }
    }
    if let Some((tag, rest)) = raw.split_once(' ') {
        if tag.bytes().all(|b| b.is_ascii_alphabetic()) {
            let inner = parse_version(Some(rest));
            if matches!(inner.scheme, Scheme::Semver | Scheme::NumericDotted) {
                let mut p = inner;
                p.scheme = Scheme::Tagged;
                p.tag = tag.to_ascii_lowercase();
                return p;
            }
        }
    }
    mk(Scheme::Other, vec![], None, 0)
}

fn numeric_like(s: &Scheme) -> bool {
    matches!(
        s,
        Scheme::Semver | Scheme::NumericDotted | Scheme::GitDescribe
    )
}

fn cmp_nums(a: &[u64], b: &[u64]) -> Ordering {
    let n = a.len().max(b.len());
    for i in 0..n {
        let c = a.get(i).unwrap_or(&0).cmp(b.get(i).unwrap_or(&0));
        if c != Ordering::Equal {
            return c;
        }
    }
    Ordering::Equal
}

fn from_ord(o: Ordering) -> Order {
    match o {
        Ordering::Less => Order::Less,
        Ordering::Equal => Order::Equal,
        Ordering::Greater => Order::Greater,
    }
}

/// Orders two version strings of the same addon. `Equal` means the strings
/// denote the same version, not that the code is byte-identical.
pub fn compare(a: &Parsed, b: &Parsed, a_raw: &str, b_raw: &str) -> Order {
    use Scheme::*;
    if matches!(a.scheme, Missing | Token) || matches!(b.scheme, Missing | Token) {
        return Order::Ambiguous("missing or unsubstituted version carries no ordering");
    }
    if matches!(a.scheme, Other | GitHash) || matches!(b.scheme, Other | GitHash) {
        return if a_raw.trim() == b_raw.trim() && a.scheme == GitHash {
            Order::Equal
        } else {
            Order::Ambiguous("unordered scheme (hash/other)")
        };
    }
    if numeric_like(&a.scheme) && numeric_like(&b.scheme) {
        let c = cmp_nums(&a.nums, &b.nums);
        if c != Ordering::Equal {
            return from_ord(c);
        }
        // Same core: a describe-after-tag is newer than the tag; pre-release is older.
        let rank = |p: &Parsed| p.pre.map(|(r, n)| (r, n)).unwrap_or((u8::MAX, 0));
        let c = rank(a).cmp(&rank(b));
        if c != Ordering::Equal {
            return from_ord(c);
        }
        return from_ord(a.ahead.cmp(&b.ahead));
    }
    if a.scheme == Tagged && b.scheme == Tagged {
        if a.tag != b.tag {
            return Order::Ambiguous("different version tags");
        }
        let c = cmp_nums(&a.nums, &b.nums);
        if c != Ordering::Equal || (a.pre.is_none() && b.pre.is_none()) {
            return from_ord(c);
        }
        return Order::Ambiguous("tagged pre-release");
    }
    if a.scheme != b.scheme {
        return Order::Ambiguous("different version schemes");
    }
    match a.scheme {
        Date | Integer | Revision => from_ord(cmp_nums(&a.nums, &b.nums)),
        _ => Order::Ambiguous("unreachable"),
    }
}

pub fn cmp_str(a: &str, b: &str) -> Order {
    compare(&parse_version(Some(a)), &parse_version(Some(b)), a, b)
}

/// Can this scheme be ordered against another version of the same scheme?
pub fn orderable(s: &Scheme) -> bool {
    matches!(
        s,
        Scheme::Semver
            | Scheme::NumericDotted
            | Scheme::Date
            | Scheme::GitDescribe
            | Scheme::Integer
            | Scheme::Revision
            | Scheme::Tagged
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use Order::*;

    fn amb(a: &str, b: &str) -> bool {
        matches!(cmp_str(a, b), Ambiguous(_))
    }

    #[test]
    fn classifies() {
        let s = |v| parse_version(Some(v)).scheme;
        assert_eq!(parse_version(None).scheme, Scheme::Missing);
        assert_eq!(parse_version(Some("  ")).scheme, Scheme::Missing);
        assert_eq!(s("@project-version@"), Scheme::Token);
        assert_eq!(s("v1.2.3"), Scheme::Semver);
        assert_eq!(s("1.2.3-beta2"), Scheme::Semver);
        assert_eq!(s("1.2"), Scheme::NumericDotted);
        assert_eq!(s("1.2.3.4"), Scheme::NumericDotted);
        assert_eq!(s("2024-05-01"), Scheme::Date);
        assert_eq!(s("20240501"), Scheme::Date);
        assert_eq!(s("2024.05.01"), Scheme::Date);
        assert_eq!(s("v1.2.3-4-gabc1234"), Scheme::GitDescribe);
        assert_eq!(s("12345"), Scheme::Integer);
        assert_eq!(s("r123"), Scheme::Revision);
        assert_eq!(s("abc1234def"), Scheme::GitHash);
        assert_eq!(s("Release Candidate"), Scheme::Other);
        assert_eq!(s("Forever 1.60.1"), Scheme::Tagged);
        assert_eq!(s("<project-version-goes-here>"), Scheme::Token);
    }

    #[test]
    fn numeric_ordering() {
        assert_eq!(cmp_str("1.2.3", "1.10.0"), Less);
        assert_eq!(cmp_str("v2.0.0", "1.9.9"), Greater);
        assert_eq!(cmp_str("1.2", "1.2.0"), Equal);
        assert_eq!(cmp_str("v1.2.3", "1.2.3"), Equal);
    }

    #[test]
    fn prerelease_ordering() {
        assert_eq!(cmp_str("1.2.3-alpha", "1.2.3-beta"), Less);
        assert_eq!(cmp_str("1.2.3-beta2", "1.2.3-beta10"), Less);
        assert_eq!(cmp_str("1.2.3-rc1", "1.2.3"), Less);
        assert_eq!(cmp_str("1.2.4-alpha", "1.2.3"), Greater);
        assert!(!amb("1.2.3-beta", "1.2.3"));
    }

    #[test]
    fn unknown_suffix_is_ambiguous() {
        assert!(amb("1.2.3-foo", "1.2.3"));
        assert!(amb("1.2.3-foo", "1.2.4"));
    }

    #[test]
    fn git_describe_ordering() {
        assert_eq!(cmp_str("v1.2.3-4-gabc1234", "v1.2.3-9-gdef5678"), Less);
        assert_eq!(cmp_str("v1.2.3-4-gabc1234", "v1.2.3"), Greater);
        assert_eq!(cmp_str("v1.2.3-4-gabc1234", "v1.2.4"), Less);
        assert_eq!(cmp_str("v1.2.3-4-gabc1234", "v1.2.4-1-gabc1234"), Less);
    }

    #[test]
    fn dates_integers_revisions() {
        assert_eq!(cmp_str("2024-05-01", "2024-12-01"), Less);
        assert_eq!(cmp_str("20241201", "20240501"), Greater);
        assert_eq!(cmp_str("2024-05-01", "2024.05.01"), Equal);
        assert_eq!(cmp_str("r99", "r100"), Less);
        assert_eq!(cmp_str("99", "100"), Less);
    }

    #[test]
    fn ambiguous_cases() {
        assert!(amb("@project-version@", "@project-version@"));
        assert!(amb("1.2.3", "2024-05-01"));
        assert!(amb("1.2.3", "12345"));
        assert!(amb("abc1234def", "def5678abc"));
        assert!(amb("r10", "10"));
        assert!(amb("Alpha Build", "1.0.0"));
    }

    #[test]
    fn tagged_versions() {
        assert_eq!(cmp_str("Forever 1.60.1", "Forever 1.61.0"), Less);
        assert!(amb("Forever 1.60.1", "BCC 2.5.4"));
        assert!(amb("Forever 1.60.1", "1.60.1"));
    }

    #[test]
    fn missing_is_ambiguous() {
        let m = parse_version(None);
        let v = parse_version(Some("1.0.0"));
        assert!(matches!(compare(&m, &v, "", "1.0.0"), Ambiguous(_)));
    }
}
