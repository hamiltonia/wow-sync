mod inventory;
mod links;
mod report;

use report::{Check, Status};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const USAGE: &str =
    "usage: cargo xtask verify [--tier offline|native|provider|all] [--allow-blocked]";

struct Options {
    tiers: Vec<String>,
    allow_blocked: bool,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut it = args.iter();
    if it.next().map(String::as_str) != Some("verify") {
        return Err(USAGE.into());
    }
    let mut tier = "all".to_string();
    let mut allow_blocked = false;
    while let Some(a) = it.next() {
        match a.as_str() {
            "--allow-blocked" => allow_blocked = true,
            "--tier" => tier = it.next().ok_or("--tier needs a value")?.clone(),
            other => match other.strip_prefix("--tier=") {
                Some(v) => tier = v.to_string(),
                None => return Err(format!("unknown argument '{other}'\n{USAGE}")),
            },
        }
    }
    let tiers: Vec<String> = match tier.as_str() {
        "all" => inventory::TIERS.iter().map(|t| t.to_string()).collect(),
        t if inventory::TIERS.contains(&t) => vec![t.to_string()],
        t => return Err(format!("unknown tier '{t}'\n{USAGE}")),
    };
    Ok(Options {
        tiers,
        allow_blocked,
    })
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives directly under the repository root")
        .to_path_buf()
}

fn tail(text: &str, lines: usize) -> String {
    let all: Vec<&str> = text.lines().collect();
    all[all.len().saturating_sub(lines)..].join("\n")
}

struct Run {
    ok: bool,
    output: String,
}

/// Runs a command; `Err` means the tool itself is unavailable.
fn run(root: &Path, program: &str, args: &[&str]) -> Result<Run, String> {
    let out = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot start '{program}': {e}"))?;
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let missing = !out.status.success()
        && (output.contains("no such command") || output.contains("is not installed"));
    if missing {
        return Err(tail(&output, 3));
    }
    Ok(Run {
        ok: out.status.success(),
        output,
    })
}

fn command_check(
    root: &Path,
    name: &str,
    install_hint: &str,
    program: &str,
    args: &[&str],
) -> (Check, String) {
    let cmd = format!("{program} {}", args.join(" "));
    match run(root, program, args) {
        Err(why) => (
            Check::new(
                "offline",
                name,
                Status::Blocked,
                format!("tool unavailable ({why}); install with: {install_hint}"),
            ),
            String::new(),
        ),
        Ok(r) if r.ok => (
            Check::new("offline", name, Status::Pass, format!("`{cmd}` exit 0")),
            r.output,
        ),
        Ok(r) => (
            Check::new(
                "offline",
                name,
                Status::Fail,
                with_failed_names(
                    format!("`{cmd}` failed:\n{}", tail(&r.output, 25)),
                    &r.output,
                ),
            ),
            r.output,
        ),
    }
}

fn offline(root: &Path) -> Vec<Check> {
    let mut checks = Vec::new();
    let fmt_hint = "rustup component add rustfmt";
    let clippy_hint = "rustup component add clippy";
    checks.push(command_check(root, "fmt", fmt_hint, "cargo", &["fmt", "--all", "--check"]).0);
    checks.push(
        command_check(
            root,
            "clippy",
            clippy_hint,
            "cargo",
            &[
                "clippy",
                "--locked",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        )
        .0,
    );
    let (test_check, test_output) = command_check(
        root,
        "test",
        "rustup toolchain install",
        "cargo",
        &["test", "--locked", "--workspace"],
    );
    let tests_ran = test_check.status == Status::Pass;
    checks.push(test_check);
    checks.push(required_cases_check(root, tests_ran, &test_output));
    checks.push(
        command_check(
            root,
            "deny",
            "cargo install --locked cargo-deny",
            "cargo",
            &["deny", "check"],
        )
        .0,
    );
    checks.push(link_check(root));
    checks
}

/// Appends the names of failed tests so CI logs identify them.
fn with_failed_names(detail: String, output: &str) -> String {
    let failed: Vec<String> = inventory::parse_test_output(output)
        .into_iter()
        .filter(|r| r.outcome == inventory::Outcome::Failed)
        .map(|r| r.name)
        .collect();
    if failed.is_empty() {
        detail
    } else {
        let mut text = format!("{detail}\nfailed tests:\n  {}", failed.join("\n  "));
        for name in &failed {
            if let Some(section) = failure_section(output, name) {
                text.push_str(&format!("\n--- {name}:\n{section}"));
            }
        }
        text
    }
}

/// The captured output of one failed test (its panic message), from cargo's `failures:` block.
fn failure_section(output: &str, name: &str) -> Option<String> {
    let head = format!("---- {name} stdout ----");
    let mut lines = output.lines().skip_while(|l| l.trim_end() != head).skip(1);
    let mut body = Vec::new();
    for l in lines.by_ref() {
        if l.starts_with("---- ") || l.trim_end() == "failures:" {
            break;
        }
        body.push(l);
    }
    while body.last().is_some_and(|l| l.trim().is_empty()) {
        body.pop();
    }
    body.truncate(20);
    (!body.is_empty()).then(|| body.join("\n"))
}

/// Windows native tier: runs the workspace tests and verifies the native cases.
fn native(root: &Path) -> Vec<Check> {
    if !cfg!(windows) {
        return blocked_tier(root, "native", "native cases need a Windows host");
    }
    match run(root, "cargo", &["test", "--locked", "--workspace"]) {
        Err(why) => vec![Check::new(
            "native",
            "tier",
            Status::Blocked,
            format!("tool unavailable ({why}); install with: rustup toolchain install"),
        )],
        Ok(r) => vec![native_check(root, r.ok, &r.output)],
    }
}

fn native_check(root: &Path, tests_ok: bool, output: &str) -> Check {
    let name = "tier";
    let text = match std::fs::read_to_string(root.join("tests").join("required-cases.toml")) {
        Ok(t) => t,
        Err(e) => {
            return Check::new(
                "native",
                name,
                Status::Fail,
                format!("cannot read tests/required-cases.toml: {e}"),
            )
        }
    };
    let inv = match inventory::parse(&text) {
        Ok(i) => i,
        Err(e) => return Check::new("native", name, Status::Fail, e),
    };
    let results = inventory::parse_test_output(output);
    let mut problems = inventory::check_required(&inv, "native", &results);
    if !tests_ok {
        problems.push(with_failed_names(
            format!("`cargo test --workspace` failed:\n{}", tail(output, 25)),
            output,
        ));
    }
    if problems.is_empty() {
        let listed = inv.case.iter().filter(|c| c.tier == "native").count();
        Check::new(
            "native",
            name,
            Status::Pass,
            format!(
                "{listed} native case(s) verified against {} test result(s)",
                results.len()
            ),
        )
    } else {
        Check::new("native", name, Status::Fail, problems.join("\n"))
    }
}

fn required_cases_check(root: &Path, tests_ran: bool, test_output: &str) -> Check {
    let name = "required-cases";
    let text = match std::fs::read_to_string(root.join("tests").join("required-cases.toml")) {
        Ok(t) => t,
        Err(e) => {
            return Check::new(
                "offline",
                name,
                Status::Fail,
                format!("cannot read tests/required-cases.toml: {e}"),
            )
        }
    };
    let inv = match inventory::parse(&text) {
        Ok(i) => i,
        Err(e) => return Check::new("offline", name, Status::Fail, e),
    };
    if !tests_ran {
        return Check::new(
            "offline",
            name,
            Status::Skipped,
            "`cargo test --workspace` did not pass, so listed cases could not be verified",
        );
    }
    let results = inventory::parse_test_output(test_output);
    let problems = inventory::check_required(&inv, "offline", &results);
    if problems.is_empty() {
        let listed = inv.case.iter().filter(|c| c.tier == "offline").count();
        Check::new(
            "offline",
            name,
            Status::Pass,
            format!(
                "{listed} listed offline case(s) verified against {} test result(s)",
                results.len()
            ),
        )
    } else {
        Check::new("offline", name, Status::Fail, problems.join("\n"))
    }
}

fn link_check(root: &Path) -> Check {
    match links::check_repo(root) {
        Err(links::LinkError::GitUnavailable(e)) => Check::new(
            "offline",
            "markdown-links",
            Status::Blocked,
            format!("git is required to list tracked Markdown files ({e}); install git"),
        ),
        Err(links::LinkError::Io(e)) => Check::new(
            "offline",
            "markdown-links",
            Status::Fail,
            format!("io error: {e}"),
        ),
        Ok((n, problems)) if problems.is_empty() => Check::new(
            "offline",
            "markdown-links",
            Status::Pass,
            format!("{n} Markdown file(s), no broken relative links"),
        ),
        Ok((_, problems)) => Check::new(
            "offline",
            "markdown-links",
            Status::Fail,
            format!("broken relative links:\n{}", problems.join("\n")),
        ),
    }
}

fn blocked_tier(root: &Path, tier: &str, reason: &str) -> Vec<Check> {
    let listed = std::fs::read_to_string(root.join("tests").join("required-cases.toml"))
        .ok()
        .and_then(|t| inventory::parse(&t).ok())
        .map(|i| i.case.iter().filter(|c| c.tier == tier).count())
        .unwrap_or(0);
    let suffix = if listed > 0 {
        format!(" ({listed} listed case(s) cannot be run)")
    } else {
        String::new()
    };
    vec![Check::new(
        tier,
        "tier",
        Status::Blocked,
        format!("{reason}{suffix}"),
    )]
}

fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match parse_args(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(2);
        }
    };
    let root = repo_root();
    // A missing report means the run did not complete; never leave a stale one behind.
    let report_dir = root.join("target").join("verify");
    let report_path = report_dir.join("report.json");
    if let Err(e) = std::fs::remove_file(&report_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            eprintln!("cannot remove stale {}: {e}", report_path.display());
            return ExitCode::from(1);
        }
    }
    let mut checks = Vec::new();
    for tier in &opts.tiers {
        match tier.as_str() {
            "offline" => checks.extend(offline(&root)),
            "native" => checks.extend(native(&root)),
            _ => checks.extend(blocked_tier(
                &root,
                "provider",
                "requires D-08 Dropbox test folder and enrolled peers",
            )),
        }
    }
    for c in &checks {
        println!("[{:<7}] {}/{}", c.status.label(), c.tier, c.name);
        if c.status != Status::Pass {
            for line in c.detail.lines() {
                println!("          {line}");
            }
        }
    }
    let meta = report::Meta {
        git_sha: git_output(&root, &["rev-parse", "HEAD"]),
        git_dirty: git_output(&root, &["status", "--porcelain"]).map(|s| !s.is_empty()),
        unix_secs: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    };
    let rep = report::build(meta, &opts.tiers, opts.allow_blocked, &checks);
    println!(
        "overall: {} (allow-blocked: {}) -> exit {}",
        rep.overall.label(),
        opts.allow_blocked,
        if rep.exit_ok { 0 } else { 1 }
    );
    let written = std::fs::create_dir_all(&report_dir).and_then(|_| {
        std::fs::write(
            &report_path,
            serde_json::to_string_pretty(&rep).expect("report serializes"),
        )
    });
    if let Err(e) = written {
        eprintln!("cannot write target/verify/report.json: {e}");
        return ExitCode::from(1);
    }
    if rep.exit_ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn tier_all_expands_and_flags_parse() {
        let o = parse_args(&a(&["verify", "--tier", "all", "--allow-blocked"])).unwrap();
        assert_eq!(o.tiers, ["offline", "native", "provider"]);
        assert!(o.allow_blocked);
        let o = parse_args(&a(&["verify", "--tier=native"])).unwrap();
        assert_eq!(o.tiers, ["native"]);
        assert!(!o.allow_blocked);
    }

    #[test]
    fn rejects_bad_tier_and_subcommand() {
        assert!(parse_args(&a(&["verify", "--tier", "bogus"])).is_err());
        assert!(parse_args(&a(&["build"])).is_err());
        assert!(parse_args(&a(&["verify", "--wat"])).is_err());
    }

    #[test]
    fn unavailable_tool_is_blocked_with_install_hint() {
        let root = repo_root();
        let (c, _) = command_check(
            &root,
            "x",
            "install-me",
            "xtask-definitely-missing-tool",
            &[],
        );
        assert_eq!(c.status, Status::Blocked);
        assert!(c.detail.contains("install-me"));
    }

    #[test]
    fn missing_native_test_fails_the_tier() {
        let root = repo_root();
        let text = std::fs::read_to_string(root.join("tests").join("required-cases.toml")).unwrap();
        let inv = inventory::parse(&text).unwrap();
        let names: Vec<&String> = inv
            .case
            .iter()
            .filter(|c| c.tier == "native")
            .flat_map(|c| &c.tests)
            .collect();
        assert!(names.len() >= 2, "native cases must be listed");
        let all: String = names.iter().map(|n| format!("test {n} ... ok\n")).collect();
        assert_eq!(native_check(&root, true, &all).status, Status::Pass);
        let missing: String = names[1..]
            .iter()
            .map(|n| format!("test {n} ... ok\n"))
            .collect();
        let c = native_check(&root, true, &missing);
        assert_eq!(c.status, Status::Fail);
        assert!(c.detail.contains("did not run"));
    }

    #[test]
    fn failed_test_names_are_printed() {
        let out = "test a::one ... ok\ntest b::flaky ... FAILED\n";
        let d = with_failed_names("x".into(), out);
        assert!(d.contains("failed tests:") && d.contains("b::flaky"));
        assert_eq!(with_failed_names("x".into(), "test a ... ok\n"), "x");
    }

    #[test]
    fn each_failed_test_prints_its_panic_message() {
        let out = "test b::flaky ... FAILED\ntest c::fine ... ok\n\nfailures:\n\n\
---- b::flaky stdout ----\nthread 'b::flaky' panicked at src/b.rs:9:5:\ntimed out waiting for the retry\n\
note: run with `RUST_BACKTRACE=1`\n\n---- d::other stdout ----\nunrelated\n\nfailures:\n    b::flaky\n";
        let d = with_failed_names("x".into(), out);
        assert!(d.contains("timed out waiting for the retry"), "{d}");
        assert!(d.contains("src/b.rs:9:5"));
        assert!(!d.contains("unrelated"));
    }

    #[test]
    fn required_cases_skipped_when_tests_did_not_pass() {
        let c = required_cases_check(&repo_root(), false, "");
        assert_eq!(c.status, Status::Skipped);
    }
}
