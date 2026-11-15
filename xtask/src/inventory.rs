use serde::Deserialize;

pub const TIERS: [&str; 3] = ["offline", "native", "provider"];

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub id: String,
    pub tier: String,
    pub tests: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    #[serde(default)]
    pub case: Vec<Case>,
}

pub fn parse(text: &str) -> Result<Inventory, String> {
    let inv: Inventory = toml::from_str(text).map_err(|e| format!("invalid inventory: {e}"))?;
    let mut seen = std::collections::HashSet::new();
    for c in &inv.case {
        if c.id.trim().is_empty() {
            return Err("case with empty id".into());
        }
        if !seen.insert((c.id.as_str(), c.tier.as_str())) {
            return Err(format!("duplicate case {} for tier {}", c.id, c.tier));
        }
        if !TIERS.contains(&c.tier.as_str()) {
            return Err(format!("case {} has unknown tier '{}'", c.id, c.tier));
        }
        if c.tests.is_empty() || c.tests.iter().any(|t| t.trim().is_empty()) {
            return Err(format!("case {} lists no usable test names", c.id));
        }
    }
    Ok(inv)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    Failed,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestResult {
    pub name: String,
    pub outcome: Outcome,
}

/// Parses libtest pretty output lines of the form `test <name> ... <outcome>`.
pub fn parse_test_output(output: &str) -> Vec<TestResult> {
    output
        .lines()
        .filter_map(|line| {
            let rest = line.strip_prefix("test ")?;
            let (name, outcome) = rest.rsplit_once(" ... ")?;
            let outcome = match outcome.split([',', ' ']).next()? {
                "ok" => Outcome::Ok,
                "FAILED" => Outcome::Failed,
                "ignored" => Outcome::Ignored,
                _ => return None,
            };
            Some(TestResult {
                name: name.to_string(),
                outcome,
            })
        })
        .collect()
}

/// Returns one problem per violated obligation for the cases of `tier`.
/// An empty result means every listed test ran, was not ignored, and passed.
pub fn check_required(inv: &Inventory, tier: &str, results: &[TestResult]) -> Vec<String> {
    let cases: Vec<&Case> = inv.case.iter().filter(|c| c.tier == tier).collect();
    let mut problems = Vec::new();
    let executed = results
        .iter()
        .filter(|r| r.outcome != Outcome::Ignored)
        .count();
    if executed == 0 {
        problems.push(format!(
            "tier '{tier}' executed zero tests ({} listed case(s))",
            cases.len()
        ));
    }
    if cases.is_empty() {
        problems.push(format!(
            "inventory lists no cases for executable tier '{tier}'; an empty inventory is a missing obligation"
        ));
    }
    if executed == 0 {
        return problems;
    }
    for case in cases {
        for test in &case.tests {
            let matches: Vec<&TestResult> = results.iter().filter(|r| &r.name == test).collect();
            if matches.is_empty() {
                problems.push(format!("{}: listed test '{test}' did not run", case.id));
            } else if matches.iter().any(|r| r.outcome == Outcome::Ignored) {
                problems.push(format!("{}: listed test '{test}' was ignored", case.id));
            } else if matches.iter().any(|r| r.outcome == Outcome::Failed) {
                problems.push(format!("{}: listed test '{test}' failed", case.id));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    const INV: &str = r#"
[[case]]
id = "A-1"
tier = "offline"
tests = ["a::one", "a::two"]
"#;

    fn r(name: &str, outcome: Outcome) -> TestResult {
        TestResult {
            name: name.into(),
            outcome,
        }
    }

    #[test]
    fn empty_inventory_parses() {
        assert!(parse("").unwrap().case.is_empty());
    }

    #[test]
    fn rejects_unknown_tier_empty_tests_and_duplicates() {
        assert!(parse("[[case]]\nid=\"X\"\ntier=\"nope\"\ntests=[\"t\"]").is_err());
        assert!(parse("[[case]]\nid=\"X\"\ntier=\"offline\"\ntests=[]").is_err());
        let dup = format!("{INV}{INV}");
        assert!(parse(&dup).unwrap_err().contains("duplicate"));
        assert!(parse("[[case]]\nid=\"X\"\ntier=\"offline\"\ntests=[\"t\"]\nextra=1").is_err());
    }

    #[test]
    fn parses_libtest_output_outcomes() {
        let out = "running 3 tests\ntest a::one ... ok\ntest a::two ... ignored, slow\ntest b::x ... FAILED\ntest result: FAILED. 1 passed\n";
        let res = parse_test_output(out);
        assert_eq!(
            res,
            vec![
                r("a::one", Outcome::Ok),
                r("a::two", Outcome::Ignored),
                r("b::x", Outcome::Failed)
            ]
        );
    }

    #[test]
    fn all_listed_tests_passing_has_no_problems() {
        let inv = parse(INV).unwrap();
        let res = [r("a::one", Outcome::Ok), r("a::two", Outcome::Ok)];
        assert!(check_required(&inv, "offline", &res).is_empty());
    }

    #[test]
    fn zero_tests_with_listed_cases_is_a_problem() {
        let inv = parse(INV).unwrap();
        let p = check_required(&inv, "offline", &[]);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("zero tests"));
    }

    #[test]
    fn zero_executed_tests_fail_even_with_empty_inventory() {
        let inv = parse("").unwrap();
        let p = check_required(&inv, "offline", &[]);
        assert_eq!(p.len(), 2);
        assert!(p[0].contains("zero tests"));
        // Ignored tests are not executed tests.
        let p = check_required(&inv, "offline", &[r("x", Outcome::Ignored)]);
        assert!(p[0].contains("zero tests"));
    }

    #[test]
    fn empty_inventory_for_executable_tier_fails_even_when_tests_pass() {
        let inv = parse("").unwrap();
        let p = check_required(&inv, "offline", &[r("x", Outcome::Ok)]);
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("no cases"));
        // Cases listed only for another tier do not satisfy this tier.
        let inv = parse(INV).unwrap();
        assert!(!check_required(&inv, "native", &[r("x", Outcome::Ok)]).is_empty());
    }

    #[test]
    fn missing_ignored_and_failed_listed_tests_are_reported() {
        let inv = parse(INV).unwrap();
        let res = [r("a::one", Outcome::Ignored), r("other", Outcome::Ok)];
        let p = check_required(&inv, "offline", &res);
        assert_eq!(p.len(), 2);
        assert!(p[0].contains("ignored"));
        assert!(p[1].contains("did not run"));
        let res = [r("a::one", Outcome::Failed), r("a::two", Outcome::Ok)];
        assert!(check_required(&inv, "offline", &res)[0].contains("failed"));
    }
}
