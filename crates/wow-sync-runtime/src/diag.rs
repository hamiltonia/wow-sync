//! Diagnostics export entry points (ops.md §5). Only a user action may call these; nothing in
//! the sync, update or notification paths does. The text itself is built by
//! `wow_sync_core::diagnostics`.

// Not yet reachable from `lib.rs` (R4-integrate re-exports it); drop this allowance then.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use wow_sync_core::diagnostics::{
    build, DiagnosticsInput, Export, ExportOptions, ExportRefused, RedactionContext,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveError {
    /// The destination is inside the shared folder or an install root.
    ForbiddenLocation,
    /// The destination has no existing parent folder or no file name.
    BadDestination,
    Io(std::io::ErrorKind),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    Refused(ExportRefused),
    Save(SaveError),
}

/// Builds the preview text. Fails closed; see `wow_sync_core::diagnostics::build`.
pub fn export_diagnostics(
    input: &DiagnosticsInput,
    ctx: &RedactionContext,
    options: ExportOptions,
) -> Result<Export, ExportRefused> {
    build(input, ctx, options)
}

/// Lowercased, `/`-separated, with a trailing separator, so `a/b` does not match `a/bc`.
fn key(p: &Path) -> String {
    let mut s = p.to_string_lossy().replace('\\', "/").to_lowercase();
    if let Some(rest) = s.strip_prefix("//?/") {
        s = rest.to_string();
    }
    if !s.ends_with('/') {
        s.push('/');
    }
    s
}

fn resolved(p: &Path) -> PathBuf {
    p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
}

/// Whether `dest` (a file path) would land inside any of `forbidden`.
fn is_forbidden(dest_parent: &Path, forbidden: &[PathBuf]) -> bool {
    let parent = key(&resolved(dest_parent));
    forbidden
        .iter()
        .filter(|f| !f.as_os_str().is_empty())
        .any(|f| parent.starts_with(&key(&resolved(f))) || parent.starts_with(&key(f)))
}

/// Writes exactly `export.text` to `dest`, refusing the shared folder and install roots.
/// Temp file in the destination folder, then rename.
pub fn save_export(export: &Export, dest: &Path, forbidden: &[PathBuf]) -> Result<(), SaveError> {
    let name = dest.file_name().ok_or(SaveError::BadDestination)?;
    let parent = match dest.parent() {
        Some(p) if !p.as_os_str().is_empty() => p,
        _ => Path::new("."),
    };
    if !parent.is_dir() {
        return Err(SaveError::BadDestination);
    }
    if is_forbidden(parent, forbidden) {
        return Err(SaveError::ForbiddenLocation);
    }
    let target = resolved(parent).join(name);
    if target.is_dir() {
        return Err(SaveError::BadDestination);
    }
    let tmp = target.with_file_name(format!(".{}.tmp", name.to_string_lossy()));
    let write = || -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(export.text.as_bytes())?;
        f.sync_all()?;
        std::fs::rename(&tmp, &target)
    };
    write().map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        SaveError::Io(e.kind())
    })
}

/// Build, then save. When the build is refused nothing is written.
pub fn export_and_save(
    input: &DiagnosticsInput,
    ctx: &RedactionContext,
    options: ExportOptions,
    dest: &Path,
    forbidden: &[PathBuf],
) -> Result<Export, ExportError> {
    let export = export_diagnostics(input, ctx, options).map_err(ExportError::Refused)?;
    save_export(&export, dest, forbidden).map_err(ExportError::Save)?;
    Ok(export)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_sync_adapters::testkit::TempDir;
    use wow_sync_core::diagnostics::*;

    fn input() -> DiagnosticsInput {
        DiagnosticsInput {
            app_version: "0.1.0",
            build_commit: "abc12345",
            toggles: Toggles {
                update_check: true,
                notify_summaries: true,
                notify_needs_me: true,
            },
            schema_majors: vec![("authority", 1)],
            os: OsFamily::Windows,
            os_version: None,
            arch: CpuArch::X64,
            process_memory_mib: None,
            wow_running: Tri::Unknown,
            variants: vec![],
            computers: vec![],
            addons: vec![],
            hold_scope: HoldKind::None,
            plan_blocks: vec![],
            journals: vec![],
            activity: vec![],
            errors: vec![],
        }
    }

    fn dirs(t: &TempDir) -> (PathBuf, PathBuf, PathBuf) {
        let shared = t.path().join("Shared");
        let install = t.path().join("WoW");
        let out = t.path().join("Documents");
        for d in [&shared, &install, &out] {
            std::fs::create_dir_all(d).unwrap();
        }
        (shared, install, out)
    }

    fn entries(d: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(d)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn saving_writes_exactly_the_previewed_bytes_outside_forbidden_roots() {
        let t = TempDir::new();
        let (shared, install, out) = dirs(&t);
        let ctx = RedactionContext::default();
        let preview = export_diagnostics(&input(), &ctx, ExportOptions::default()).unwrap();
        let dest = out.join("report.txt");
        let saved = export_and_save(
            &input(),
            &ctx,
            ExportOptions::default(),
            &dest,
            &[shared, install],
        )
        .unwrap();
        assert_eq!(saved, preview);
        assert_eq!(std::fs::read(&dest).unwrap(), preview.text.as_bytes());
        assert_eq!(
            entries(&out),
            vec!["report.txt"],
            "no temp file left behind"
        );
    }

    #[test]
    fn the_shared_folder_and_install_roots_are_refused_however_they_are_spelled() {
        let t = TempDir::new();
        let (shared, install, out) = dirs(&t);
        let sub = shared.join("deep").join("er");
        std::fs::create_dir_all(&sub).unwrap();
        let forbidden = [shared.clone(), install.clone()];
        let ex = export_diagnostics(
            &input(),
            &RedactionContext::default(),
            ExportOptions::default(),
        )
        .unwrap();

        let upper = PathBuf::from(shared.to_string_lossy().to_uppercase());
        let via_dotdot = out.join("..").join("Shared").join("x.txt");
        let mut cases = vec![
            shared.join("x.txt"),
            sub.join("x.txt"),
            install.join("x.txt"),
            via_dotdot,
        ];
        // Only on a case-insensitive volume does this spelling name the same folder.
        if upper.is_dir() {
            cases.push(upper.join("x.txt"));
        }
        for dest in cases {
            assert_eq!(
                save_export(&ex, &dest, &forbidden),
                Err(SaveError::ForbiddenLocation),
                "{}",
                dest.display()
            );
        }
        assert!(entries(&shared).iter().all(|n| n != "x.txt"));
        assert_eq!(entries(&install), Vec::<String>::new());
        assert_eq!(entries(&sub), Vec::<String>::new());

        // A sibling whose name merely starts with the root's name is fine.
        let sibling = t.path().join("Shared2");
        std::fs::create_dir(&sibling).unwrap();
        save_export(&ex, &sibling.join("x.txt"), &forbidden).unwrap();
        assert!(sibling.join("x.txt").is_file());
    }

    #[test]
    fn bad_destinations_are_refused_without_writing() {
        let t = TempDir::new();
        let (_, _, out) = dirs(&t);
        let ex = export_diagnostics(
            &input(),
            &RedactionContext::default(),
            ExportOptions::default(),
        )
        .unwrap();
        assert_eq!(
            save_export(&ex, &out.join("missing").join("x.txt"), &[]),
            Err(SaveError::BadDestination)
        );
        std::fs::create_dir(out.join("adir")).unwrap();
        assert_eq!(
            save_export(&ex, &out.join("adir"), &[]),
            Err(SaveError::BadDestination)
        );
        assert_eq!(entries(&out), vec!["adir"]);
    }

    #[test]
    fn a_refused_export_saves_nothing() {
        let t = TempDir::new();
        let (_, _, out) = dirs(&t);
        let mut i = input();
        i.addons.push(AddonRow {
            folder_name: "zz_user_PLANT".into(),
            status: AddonStatus::Ok,
            candidate_count: 1,
            reason: ReasonKind::None,
            hold: HoldKind::None,
            source_text: SourceText::default(),
            fingerprint: None,
        });
        let ctx = RedactionContext {
            os_user_name: "zz_user_PLANT".into(),
            ..RedactionContext::default()
        };
        let r = export_and_save(&i, &ctx, ExportOptions::default(), &out.join("r.txt"), &[]);
        assert_eq!(
            r,
            Err(ExportError::Refused(ExportRefused::PrivateDataFound(
                PrivateKind::ContextString
            )))
        );
        assert_eq!(entries(&out), Vec::<String>::new());
    }

    #[test]
    fn no_scheduled_path_calls_the_export() {
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let names = [
            concat!("export_", "diagnostics"),
            concat!("export_", "and_save"),
            concat!("save_", "export"),
            concat!("diag", "::"),
        ];
        let mut scanned = 0;
        for f in [
            "updates.rs",
            "notices.rs",
            "activity.rs",
            "settings_cycle.rs",
            "removal_cycle.rs",
        ] {
            let Ok(text) = std::fs::read_to_string(src.join(f)) else {
                continue;
            };
            scanned += 1;
            let shipped = text.split("#[cfg(test)]").next().unwrap();
            for n in names {
                assert!(!shipped.contains(n), "{f} references {n}");
            }
        }
        assert!(scanned >= 3, "the scan found the scheduled-path files");
    }
}
