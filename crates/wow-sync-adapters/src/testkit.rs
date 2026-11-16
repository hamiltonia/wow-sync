//! Synthetic installs and temp directories. Everything lives under the OS temp
//! directory; real WoW or provider folders are never touched.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// `<temp>/wowsync-test-<random>`, removed on drop.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub fn new() -> TempDir {
        let base = std::env::temp_dir();
        // macOS temp lives behind a symlink; Windows keeps the path as is (verbatim paths
        // would defeat the policy checks) but expands short names.
        #[cfg(not(windows))]
        let base = std::fs::canonicalize(&base).unwrap_or(base);
        #[cfg(windows)]
        let base = crate::winproc::long_path(&base);
        let path = base.join(format!("wowsync-test-{}", crate::sysinfo::random_id_hex()));
        std::fs::create_dir_all(&path).expect("create temp dir");
        TempDir { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn join(&self, rel: &str) -> PathBuf {
        self.path.join(rel)
    }
}

impl Default for TempDir {
    fn default() -> Self {
        TempDir::new()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub struct SyntheticInstall {
    pub root: PathBuf,
    pub variant_dir: PathBuf,
    pub addons: PathBuf,
}

impl SyntheticInstall {
    pub fn new_forever(parent: &Path, name: &str) -> SyntheticInstall {
        SyntheticInstall::with_product(parent, name, "_forever_", "wowf")
    }

    /// `<parent>/<name>/` with `.build.info`, `<folder>/.flavor.info`,
    /// `<folder>/Interface/AddOns` and the slice marker `.wowsync-synthetic`.
    pub fn with_product(
        parent: &Path,
        name: &str,
        folder: &str,
        product: &str,
    ) -> SyntheticInstall {
        let root = parent.join(name);
        let variant_dir = root.join(folder);
        let addons = variant_dir.join("Interface").join("AddOns");
        std::fs::create_dir_all(&addons).expect("create synthetic install");
        std::fs::write(
            root.join(".build.info"),
            format!(
                "Branch!STRING:0|Active!DEC:1|Product!STRING:0|Version!STRING:0\nus|1|{product}|1.0.0.1\n"
            ),
        )
        .unwrap();
        std::fs::write(
            variant_dir.join(".flavor.info"),
            format!("Product Flavor!STRING:0\n{product}\n"),
        )
        .unwrap();
        std::fs::write(root.join(".wowsync-synthetic"), "wowsync-synthetic/1").unwrap();
        SyntheticInstall {
            root,
            variant_dir,
            addons,
        }
    }

    pub fn add_addon(&self, folder: &str, toc_name: &str, toc_body: &str, files: &[(&str, &[u8])]) {
        let dir = self.addons.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(toc_name), toc_body).unwrap();
        for (rel, bytes) in files {
            let p = rel.split('/').fold(dir.clone(), |a, s| a.join(s));
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, bytes).unwrap();
        }
    }

    /// Moves every mtime in the install beyond the scan stability window.
    pub fn set_old_mtimes(&self) {
        set_old_mtimes_under(&self.root);
    }
}

pub fn set_old_mtimes_under(dir: &Path) {
    let old = SystemTime::now() - Duration::from_secs(3600);
    fn walk(p: &Path, old: SystemTime) {
        if let Ok(rd) = std::fs::read_dir(p) {
            for e in rd.flatten() {
                let q = e.path();
                if q.is_dir() {
                    walk(&q, old);
                }
                set_mtime(&q, old);
            }
        }
    }
    walk(dir, old);
    set_mtime(dir, old);
}

pub fn set_mtime(path: &Path, t: SystemTime) {
    let mut oo = std::fs::OpenOptions::new();
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_WRITE_ATTRIBUTES: u32 = 0x100;
        const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
        oo.access_mode(FILE_WRITE_ATTRIBUTES)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS);
    }
    #[cfg(not(windows))]
    oo.read(true);
    let f = oo.open(path).expect("open for set_modified");
    f.set_modified(t).expect("set_modified");
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Deterministic synthetic files under `dir`; the total size never exceeds `total_bytes`.
pub fn synthetic_dataset(dir: &Path, seed: u64, total_bytes: u64) -> Vec<PathBuf> {
    let mut rng = Rng(seed);
    let mut remaining = total_bytes;
    let mut out = Vec::new();
    let mut idx = 0u32;
    while remaining > 0 {
        let want = 1024 + rng.next() % (256 * 1024);
        let size = want.min(remaining);
        let sub = dir.join(format!("d{}", idx % 4));
        std::fs::create_dir_all(&sub).expect("create dataset dir");
        let mut bytes = Vec::with_capacity(size as usize);
        while (bytes.len() as u64) < size {
            bytes.extend_from_slice(&rng.next().to_le_bytes());
        }
        bytes.truncate(size as usize);
        let p = sub.join(format!("f{idx:05}.bin"));
        std::fs::write(&p, &bytes).expect("write dataset file");
        out.push(p);
        remaining -= size;
        idx += 1;
    }
    out
}

/// Stand-in for "WoW running": a copy of `ping.exe` named `Wow.exe` in the variant
/// folder, killed on drop.
#[cfg(windows)]
pub struct FakeClient {
    child: std::process::Child,
}

#[cfg(windows)]
impl FakeClient {
    pub fn start(variant_dir: &Path) -> FakeClient {
        let ping = Path::new(&std::env::var("SystemRoot").expect("SystemRoot"))
            .join("System32")
            .join("ping.exe");
        let exe = variant_dir.join("Wow.exe");
        std::fs::copy(ping, &exe).expect("copy ping.exe");
        let child = std::process::Command::new(&exe)
            .args(["-n", "30", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("start fake client");
        FakeClient { child }
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

#[cfg(windows)]
impl Drop for FakeClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_scan::{list_variant_folders, read_variant_files, FileRead};

    #[test]
    fn synthetic_install_has_valid_variant_metadata() {
        let t = TempDir::new();
        let inst = SyntheticInstall::new_forever(t.path(), "wow");
        let v = read_variant_files(&inst.root, "_forever_");
        match (&v.flavor_info, &v.build_info) {
            (FileRead::Bytes(f), FileRead::Bytes(b)) => {
                assert!(String::from_utf8_lossy(f).lines().any(|l| l == "wowf"));
                let b = String::from_utf8_lossy(b);
                assert!(b.contains("Product!STRING:0") && b.contains("us|1|wowf|"));
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(v.addons_dir, crate::fs_scan::DirKind::PlainDir);
        assert!(inst.addons.is_dir());
        let other = SyntheticInstall::with_product(t.path(), "wow2", "_classic_", "wow_classic");
        assert_eq!(
            list_variant_folders(&other.root),
            vec!["_classic_".to_string()]
        );
        assert!(std::fs::read_to_string(other.root.join(".build.info"))
            .unwrap()
            .contains("wow_classic"));
    }

    #[test]
    fn synthetic_install_has_marker() {
        let t = TempDir::new();
        let inst = SyntheticInstall::new_forever(t.path(), "wow");
        assert_eq!(
            std::fs::read_to_string(inst.root.join(".wowsync-synthetic")).unwrap(),
            "wowsync-synthetic/1"
        );
        assert!(matches!(
            read_variant_files(&inst.root, "_forever_").synthetic_marker,
            FileRead::Bytes(b) if b == b"wowsync-synthetic/1"
        ));
    }

    #[test]
    fn synthetic_dataset_respects_cap_and_is_deterministic() {
        let a = TempDir::new();
        let b = TempDir::new();
        let cap = 700_000u64;
        let fa = synthetic_dataset(a.path(), 42, cap);
        let fb = synthetic_dataset(b.path(), 42, cap);
        let total: u64 = fa.iter().map(|p| std::fs::metadata(p).unwrap().len()).sum();
        assert_eq!(total, cap);
        assert_eq!(fa.len(), fb.len());
        for (x, y) in fa.iter().zip(&fb) {
            assert_eq!(std::fs::read(x).unwrap(), std::fs::read(y).unwrap());
        }
        let c = TempDir::new();
        let fc = synthetic_dataset(c.path(), 43, cap);
        assert_ne!(
            std::fs::read(&fa[0]).unwrap(),
            std::fs::read(&fc[0]).unwrap()
        );
        assert!(synthetic_dataset(c.path(), 1, 0).is_empty());
    }
}
