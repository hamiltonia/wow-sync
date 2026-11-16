//! Version-label classification and strict same-scheme ordering (domain.md §4).
//! No input here is a clock or a file time: ordering uses labels only.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Version {
    DottedNumeric {
        parts: Vec<u64>,
        pre: Option<(String, u64)>,
        describe: Option<(u64, String)>,
    },
    Integer(u64),
    Revision(u64),
    Date(u32, u32, u32),
    Tagged {
        tag: String,
        v: Box<Version>,
    },
    Missing,
    Token(String),
    GitHash(String),
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionOrder {
    Less,
    Equal,
    Greater,
    Ambiguous(String),
}

fn parse_u64(s: &str, max_digits: usize) -> Option<u64> {
    if s.is_empty() || s.len() > max_digits || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

fn valid_date(y: u32, m: u32, d: u32) -> bool {
    if !(1990..=2100).contains(&y) || !(1..=12).contains(&m) || d == 0 {
        return false;
    }
    let leap = (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400);
    let dim = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if leap {
                29
            } else {
                28
            }
        }
    };
    d <= dim
}

fn classify_date(s: &str) -> Option<Version> {
    let (y, m, d) = if s.len() == 10
        && (s.as_bytes()[4] == b'-' || s.as_bytes()[4] == b'.')
        && s.as_bytes()[7] == s.as_bytes()[4]
    {
        (&s[0..4], &s[5..7], &s[8..10])
    } else if s.len() == 8 {
        (&s[0..4], &s[4..6], &s[6..8])
    } else {
        return None;
    };
    let (y, m, d) = (
        parse_u64(y, 4)? as u32,
        parse_u64(m, 2)? as u32,
        parse_u64(d, 2)? as u32,
    );
    valid_date(y, m, d).then_some(Version::Date(y, m, d))
}

fn classify_dotted(s: &str) -> Option<Version> {
    let s = s.strip_prefix(['v', 'V']).unwrap_or(s);
    let mut parts = Vec::new();
    let mut rest = s;
    loop {
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        if end == 0 {
            break;
        }
        parts.push(parse_u64(&rest[..end], 18)?);
        rest = &rest[end..];
        match rest.strip_prefix('.') {
            Some(after) if after.starts_with(|c: char| c.is_ascii_digit()) => rest = after,
            _ => break,
        }
    }
    if parts.len() < 2 {
        return None;
    }
    if rest.is_empty() {
        return Some(Version::DottedNumeric {
            parts,
            pre: None,
            describe: None,
        });
    }
    let tail = rest.strip_prefix(['-', '.']).unwrap_or(rest);
    if let Some((n, hash)) = tail.split_once("-g") {
        if let Some(n) = parse_u64(n, 9) {
            if (4..=40).contains(&hash.len()) && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Some(Version::DottedNumeric {
                    parts,
                    pre: None,
                    describe: Some((n, hash.to_ascii_lowercase())),
                });
            }
        }
    }
    let word_end = tail
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(tail.len());
    let (word, digits) = tail.split_at(word_end);
    let channel = match word.to_ascii_lowercase().as_str() {
        "dev" => "dev",
        "alpha" | "a" => "alpha",
        "beta" | "b" => "beta",
        "rc" => "rc",
        _ => return None,
    };
    let number = if digits.is_empty() {
        0
    } else {
        parse_u64(digits, 9)?
    };
    Some(Version::DottedNumeric {
        parts,
        pre: Some((channel.to_string(), number)),
        describe: None,
    })
}

pub fn classify(raw: Option<&str>) -> Version {
    let Some(raw) = raw else {
        return Version::Missing;
    };
    let s = raw.trim();
    if s.is_empty() {
        return Version::Missing;
    }
    if s.contains('@') || (s.contains('<') && s.contains('>')) {
        return Version::Token(s.to_string());
    }
    if let Some(d) = classify_date(s) {
        return d;
    }
    if let Some(n) = parse_u64(s, 18) {
        return Version::Integer(n);
    }
    if let Some(rest) = s.strip_prefix(['r', 'R']) {
        if let Some(n) = parse_u64(rest, 18) {
            return Version::Revision(n);
        }
    }
    if let Some(v) = classify_dotted(s) {
        return v;
    }
    if let Some((tag, rest)) = s.split_once(char::is_whitespace) {
        if !tag.is_empty() && tag.bytes().all(|b| b.is_ascii_alphabetic()) {
            if let Some(v @ Version::DottedNumeric { .. }) = classify_dotted(rest.trim()) {
                return Version::Tagged {
                    tag: tag.to_ascii_lowercase(),
                    v: Box::new(v),
                };
            }
        }
    }
    if (7..=40).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Version::GitHash(s.to_string());
    }
    Version::Other(s.to_string())
}

fn ord<T: Ord>(a: T, b: T) -> VersionOrder {
    match a.cmp(&b) {
        std::cmp::Ordering::Less => VersionOrder::Less,
        std::cmp::Ordering::Equal => VersionOrder::Equal,
        std::cmp::Ordering::Greater => VersionOrder::Greater,
    }
}

fn padded(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let n = a.len().max(b.len());
    let pad = |p: &[u64]| {
        let mut v = p.to_vec();
        v.resize(n, 0);
        v
    };
    (pad(a), pad(b))
}

fn amb(why: &str) -> VersionOrder {
    VersionOrder::Ambiguous(why.to_string())
}

fn compare_dotted(a: &Version, b: &Version) -> VersionOrder {
    let (
        Version::DottedNumeric {
            parts: pa,
            pre: prea,
            describe: da,
        },
        Version::DottedNumeric {
            parts: pb,
            pre: preb,
            describe: db,
        },
    ) = (a, b)
    else {
        return amb("different version schemes");
    };
    let (xa, xb) = padded(pa, pb);
    match (da, db) {
        (Some(_), Some(_)) => amb("two git-describe labels"),
        (Some(_), None) | (None, Some(_)) => {
            let (desc_is_a, other_pre) = if da.is_some() {
                (true, preb)
            } else {
                (false, prea)
            };
            if other_pre.is_some() || xa != xb {
                return amb("git-describe against a different label");
            }
            if desc_is_a {
                VersionOrder::Greater
            } else {
                VersionOrder::Less
            }
        }
        (None, None) => {
            let same_channel = match (prea, preb) {
                (None, None) => true,
                (Some((ca, _)), Some((cb, _))) => ca == cb,
                _ => false,
            };
            if !same_channel {
                return amb("different channels");
            }
            match ord(&xa, &xb) {
                VersionOrder::Equal => match (prea, preb) {
                    (Some((_, na)), Some((_, nb))) => ord(na, nb),
                    _ => VersionOrder::Equal,
                },
                other => other,
            }
        }
    }
}

pub fn compare(a: &Version, b: &Version) -> VersionOrder {
    use Version::*;
    match (a, b) {
        (Missing | Token(_) | GitHash(_) | Other(_), _)
        | (_, Missing | Token(_) | GitHash(_) | Other(_)) => amb("version label cannot be ordered"),
        (Integer(x), Integer(y)) => ord(x, y),
        (Revision(x), Revision(y)) => ord(x, y),
        (Date(y1, m1, d1), Date(y2, m2, d2)) => ord((y1, m1, d1), (y2, m2, d2)),
        (Tagged { tag: t1, v: v1 }, Tagged { tag: t2, v: v2 }) => {
            if !t1.eq_ignore_ascii_case(t2) {
                return amb("different version tags");
            }
            let pre = |v: &Version| matches!(v, DottedNumeric { pre: Some(_), .. });
            if pre(v1) || pre(v2) {
                return amb("pre-release in a tagged label");
            }
            compare_dotted(v1, v2)
        }
        (DottedNumeric { .. }, DottedNumeric { .. }) => compare_dotted(a, b),
        _ => amb("different version schemes"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmp(a: &str, b: &str) -> VersionOrder {
        compare(&classify(Some(a)), &classify(Some(b)))
    }

    fn is_amb(o: VersionOrder) -> bool {
        matches!(o, VersionOrder::Ambiguous(_))
    }

    #[test]
    fn dotted_numeric_orders_numerically() {
        assert_eq!(cmp("1.10.0", "1.9.0"), VersionOrder::Greater);
        assert_eq!(cmp("v1.3.9", "v1.3.10"), VersionOrder::Less);
        assert_eq!(cmp("12.1.23", "12.1.23"), VersionOrder::Equal);
    }

    #[test]
    fn missing_parts_are_zero_padded() {
        assert_eq!(cmp("1.2", "1.2.0"), VersionOrder::Equal);
        assert_eq!(cmp("1.2", "1.2.1"), VersionOrder::Less);
    }

    #[test]
    fn prerelease_orders_below_release_same_channel_only() {
        assert_eq!(cmp("1.3.0-beta1", "1.3.0-beta2"), VersionOrder::Less);
        assert_eq!(cmp("1.3.0-rc2", "1.3.0-rc1"), VersionOrder::Greater);
        assert_eq!(cmp("1.4.0-beta1", "1.3.0-beta9"), VersionOrder::Greater);
        assert_eq!(cmp("1.3.0b2", "1.3.0-beta2"), VersionOrder::Equal);
        assert!(is_amb(cmp("1.3.0-beta1", "1.3.0-alpha2")));
    }

    #[test]
    fn prerelease_vs_release_is_ambiguous() {
        assert!(is_amb(cmp("1.3.0-beta1", "1.2.9")));
        assert!(is_amb(cmp("1.3.0-beta1", "1.3.0")));
        assert!(is_amb(cmp("2.0.0", "2.0.0-rc1")));
    }

    #[test]
    fn unknown_prerelease_word_is_ambiguous() {
        assert!(matches!(classify(Some("1.2.3-pre")), Version::Other(_)));
        assert!(is_amb(cmp("1.2.3-pre", "1.2.3-pre")));
        assert!(is_amb(cmp("1.2.3-pre", "1.2.4")));
    }

    #[test]
    fn git_describe_is_newer_than_its_exact_tag() {
        assert_eq!(cmp("1.2.3-5-g1a2b3c4", "1.2.3"), VersionOrder::Greater);
        assert_eq!(cmp("v1.2.3", "1.2.3-5-g1a2b3c4"), VersionOrder::Less);
        assert!(is_amb(cmp("1.2.4-5-g1a2b3c4", "1.2.3")));
        assert!(is_amb(cmp("1.2.3-5-g1a2b3c4", "1.2.2")));
    }

    #[test]
    fn divergent_describe_branches_ambiguous() {
        assert!(is_amb(cmp("1.2.3-5-g1a2b3c4", "1.2.3-6-gdeadbee")));
        assert!(is_amb(cmp("1.2.3-5-g1a2b3c4", "1.2.3-5-g1a2b3c4")));
        assert!(is_amb(cmp("1.2.3-5-g1a2b3c4", "1.2.3-beta1")));
    }

    #[test]
    fn integer_revision_date_order_within_scheme() {
        assert_eq!(cmp("31217", "31218"), VersionOrder::Less);
        assert_eq!(cmp("r272", "r100"), VersionOrder::Greater);
        assert_eq!(cmp("2024-01-15", "2023.12.31"), VersionOrder::Greater);
        assert_eq!(cmp("20240115", "2024-01-15"), VersionOrder::Equal);
        assert!(matches!(classify(Some("20241301")), Version::Integer(_)));
    }

    #[test]
    fn tagged_requires_same_tag() {
        assert_eq!(cmp("Forever 1.60.1", "forever 1.60.2"), VersionOrder::Less);
        assert!(is_amb(cmp("Forever 1.60.1", "BCC 1.60.2")));
        assert!(is_amb(cmp("Forever 1.60.1", "1.60.2")));
        assert!(is_amb(cmp("BCC 2.5.4-beta1", "BCC 2.5.4")));
    }

    #[test]
    fn cross_scheme_is_ambiguous() {
        assert!(is_amb(cmp("5.0.0", "31217")));
        assert!(is_amb(cmp("r272", "31217")));
        assert!(is_amb(cmp("2024-01-15", "20240115x")));
        assert!(is_amb(cmp("3.220", "r272")));
    }

    #[test]
    fn missing_token_hash_other_are_ambiguous() {
        for raw in [
            None,
            Some(" "),
            Some("<project-version-goes-here>"),
            Some("@project-version@"),
            Some("a8e9f0e"),
            Some("weird label"),
        ] {
            let v = classify(raw);
            assert!(is_amb(compare(&v, &v)), "{raw:?}");
            assert!(is_amb(compare(&v, &classify(Some("1.2.3")))), "{raw:?}");
        }
    }

    #[test]
    fn s_addon_labels_classify_as_expected() {
        use Version::*;
        assert!(matches!(classify(Some("12.1.23")), DottedNumeric { .. }));
        assert!(matches!(classify(Some("v1.3.9")), DottedNumeric { .. }));
        assert!(matches!(classify(Some("3.220")), DottedNumeric { .. }));
        assert_eq!(classify(Some("r272")), Revision(272));
        assert_eq!(classify(Some("31217")), Integer(31217));
        assert!(matches!(classify(Some("Forever 1.60.1")), Tagged { .. }));
        assert!(matches!(classify(Some("BCC 2.5.4")), Tagged { .. }));
        assert_eq!(classify(Some("a8e9f0e")), GitHash("a8e9f0e".into()));
        assert!(matches!(
            classify(Some("<project-version-goes-here>")),
            Token(_)
        ));
        assert_eq!(classify(None), Missing);
    }
}
