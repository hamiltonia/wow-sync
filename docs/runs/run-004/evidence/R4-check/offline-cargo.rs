use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "deny") {
        args.insert(1, "--offline".into());
        eprintln!("R4-check: cargo deny is restricted to --offline");
    }
    let cargo = std::path::PathBuf::from(std::env::var_os("USERPROFILE").unwrap())
        .join(".cargo")
        .join("bin")
        .join("cargo.exe");
    match Command::new(cargo).args(args).status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("R4-check cargo proxy could not launch: {error}");
            ExitCode::FAILURE
        }
    }
}
