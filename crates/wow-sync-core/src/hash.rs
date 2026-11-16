//! SHA-256 wrapper and tree/package hash composition (publication.md §3).

use crate::ids::Sha256;
use crate::inventory::FileEntry;
use crate::paths::FolderKey;
use sha2::Digest;

/// Incremental SHA-256.
pub struct Hasher(sha2::Sha256);

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher {
    pub fn new() -> Self {
        Self(sha2::Sha256::new())
    }
    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }
    pub fn finish(self) -> Sha256 {
        Sha256(self.0.finalize().into())
    }
}

pub fn sha256(bytes: &[u8]) -> Sha256 {
    let mut h = Hasher::new();
    h.update(bytes);
    h.finish()
}

/// Tree hash over files sorted by path bytes; input order is irrelevant.
pub fn tree_hash(files: &[FileEntry]) -> Sha256 {
    let mut sorted: Vec<&FileEntry> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.as_str().as_bytes().cmp(b.path.as_str().as_bytes()));
    let mut h = Hasher::new();
    h.update(b"wowsync-tree-v1\n");
    for f in sorted {
        h.update(format!("{}\t{}\t{}\n", f.path, f.size, f.sha256).as_bytes());
    }
    h.finish()
}

/// Package hash over member folders sorted by key.
pub fn package_hash(folders: &[(FolderKey, Sha256)]) -> Sha256 {
    let mut sorted: Vec<&(FolderKey, Sha256)> = folders.iter().collect();
    sorted.sort_by(|a, b| a.0.as_str().as_bytes().cmp(b.0.as_str().as_bytes()));
    let mut h = Hasher::new();
    h.update(b"wowsync-package-v1\n");
    for (k, t) in sorted {
        h.update(format!("{k}\t{t}\n").as_bytes());
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::{validate_folder_name, validate_rel_path};

    fn fe(path: &str, content: &[u8]) -> FileEntry {
        FileEntry {
            path: validate_rel_path(path).unwrap(),
            size: content.len() as u64,
            sha256: sha256(content),
        }
    }

    #[test]
    fn tree_hash_independent_of_input_order() {
        let a = fe("A.lua", b"abc");
        let b = fe("sub/b.txt", b"");
        let c = fe("z.toc", b"x");
        assert_eq!(
            tree_hash(&[a.clone(), b.clone(), c.clone()]),
            tree_hash(&[c, a, b])
        );
    }

    #[test]
    fn tree_hash_changes_with_path_size_or_content() {
        let base = tree_hash(&[fe("a.lua", b"abc")]);
        assert_ne!(base, tree_hash(&[fe("b.lua", b"abc")]));
        assert_ne!(base, tree_hash(&[fe("a.lua", b"abd")]));
        let mut resized = fe("a.lua", b"abc");
        resized.size += 1;
        assert_ne!(base, tree_hash(&[resized]));
        assert_ne!(base, tree_hash(&[fe("A.lua", b"abc")]));
    }

    #[test]
    fn tree_and_package_hash_golden_vectors() {
        // Independently computed with Python hashlib from the publication.md §3 definitions.
        assert_eq!(
            sha256(b"abc").to_string(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let t1 = tree_hash(&[fe("sub/b.txt", b""), fe("A.lua", b"abc")]);
        assert_eq!(
            t1.to_string(),
            "04b7278e36326039968a5824a4d5d0e232736ca1cae8c61a325f90fb57d59bd0"
        );
        let t2 = tree_hash(&[]);
        assert_eq!(
            t2.to_string(),
            "d2ce751e320a9a3e7c72bfccc170747926322ced8e50254d3cf5ae3c09c4692a"
        );
        let p = package_hash(&[
            (validate_folder_name("dbm-gui").unwrap(), t2),
            (validate_folder_name("DBM-Core").unwrap(), t1),
        ]);
        assert_eq!(
            p.to_string(),
            "5e3fc92df8c80ee2332616205f06d4d5e96d3f52790c9db6fb9098f952fa772f"
        );
    }
}
