//! Developer helper (console): builds synthetic installs and a fake running client so the
//! shell can be exercised without real game data. Never touches a path it did not create.
//! The `.wowsync-synthetic` marker it writes is not required for real installs; it only
//! marks a folder as a test-mode install.
//!
//!   devkit make-install <root> [--folder _forever_] [--product wowf]
//!   devkit add-addon <root> <folder> <Name> <tag> [<version>] [--depends X]
//!   devkit fake-client <variant_dir>      (Windows; prints the PID, leaves it running)

use std::path::Path;

fn fail(msg: &str) -> ! {
    eprintln!("devkit: {msg}");
    std::process::exit(2);
}

fn flag(args: &[String], name: &str, default: &str) -> String {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
        .unwrap_or_else(|| default.to_string())
}

fn make_install(root: &Path, folder: &str, product: &str) {
    let variant = root.join(folder);
    std::fs::create_dir_all(variant.join("Interface").join("AddOns")).expect("create install");
    std::fs::write(
        root.join(".build.info"),
        format!(
            "Branch!STRING:0|Active!DEC:1|Product!STRING:0|Version!STRING:0\nus|1|{product}|1.0.0.1\n"
        ),
    )
    .expect("write .build.info");
    std::fs::write(
        variant.join(".flavor.info"),
        format!("Product Flavor!STRING:0\n{product}\n"),
    )
    .expect("write .flavor.info");
    std::fs::write(root.join(".wowsync-synthetic"), "wowsync-synthetic/1")
        .expect("write test-mode marker");
}

fn add_addon(root: &Path, folder: &str, name: &str, tag: &str, version: &str, depends: &str) {
    let dir = root
        .join(folder)
        .join("Interface")
        .join("AddOns")
        .join(name);
    std::fs::create_dir_all(&dir).expect("create add-on");
    let mut toc = format!("## Interface: 11508\n## Version: {version}\n");
    if !depends.is_empty() {
        toc.push_str(&format!("## Dependencies: {depends}\n"));
    }
    std::fs::write(dir.join(format!("{name}.toc")), toc).expect("write toc");
    std::fs::write(dir.join("main.lua"), tag).expect("write main.lua");
}

#[cfg(windows)]
fn stop_inheriting_std_handles() {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetStdHandle(which: u32) -> isize;
        fn SetHandleInformation(h: isize, mask: u32, flags: u32) -> i32;
    }
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const HANDLE_FLAG_INHERIT: u32 = 1;
    for which in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: plain Win32 calls; an invalid or absent handle just makes the call fail.
        unsafe {
            let h = GetStdHandle(which);
            if h != 0 && h != -1 {
                SetHandleInformation(h, HANDLE_FLAG_INHERIT, 0);
            }
        }
    }
}

#[cfg(windows)]
fn fake_client(variant: &Path) {
    let system = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    let ping = std::path::PathBuf::from(system)
        .join("System32")
        .join("PING.EXE");
    let exe = variant.join("Wow.exe");
    if !variant.join("Interface").is_dir() {
        fail("not a variant folder created by make-install");
    }
    std::fs::copy(&ping, &exe).expect("copy stand-in client");
    // A captured (piped) stdout or stderr must not stay open in the stand-in, or the capture
    // would wait for the stand-in to exit (RC2-4).
    stop_inheriting_std_handles();
    // Intentionally left running; the caller stops it by PID.
    #[allow(clippy::zombie_processes)]
    let child = std::process::Command::new(&exe)
        .args(["-t", "127.0.0.1"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("start stand-in client");
    {
        use std::io::Write;
        let mut out = std::io::stdout();
        let _ = writeln!(out, "{}", child.id());
        let _ = out.flush();
    }
}

#[cfg(not(windows))]
fn fake_client(_variant: &Path) {
    fail("fake-client is Windows only");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("make-install") if args.len() >= 2 => make_install(
            Path::new(&args[1]),
            &flag(&args, "--folder", "_forever_"),
            &flag(&args, "--product", "wowf"),
        ),
        Some("add-addon") if args.len() >= 5 => {
            let version = args
                .get(5)
                .filter(|v| !v.starts_with("--"))
                .cloned()
                .unwrap_or_else(|| "1.0".into());
            add_addon(
                Path::new(&args[1]),
                &args[2],
                &args[3],
                &args[4],
                &version,
                &flag(&args, "--depends", ""),
            );
        }
        Some("fake-client") if args.len() >= 2 => fake_client(Path::new(&args[1])),
        _ => fail("usage: make-install | add-addon | fake-client (see the file header)"),
    }
}
