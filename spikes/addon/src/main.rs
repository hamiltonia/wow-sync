#![allow(dead_code)] // spike: compare/Order are exercised by unit tests only
mod toc;
mod version;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use toc::{parse_toc, toc_suffix, Toc, DOCUMENTED_SUFFIXES};
use version::{parse_version, Scheme};

struct Folder {
    name: String,
    /// (suffix, parsed toc) for every toc file; suffix None = plain.
    tocs: Vec<(Option<String>, Toc)>,
    unmatched_toc_names: Vec<String>,
}

impl Folder {
    fn primary(&self) -> Option<&Toc> {
        self.tocs
            .iter()
            .find(|(s, _)| s.is_none())
            .or(self.tocs.first())
            .map(|(_, t)| t)
    }
    fn versions(&self) -> BTreeSet<String> {
        self.tocs
            .iter()
            .filter_map(|(_, t)| t.get("version").map(String::from))
            .collect()
    }
    fn version(&self) -> Option<String> {
        self.primary()
            .and_then(|t| t.get("version").map(String::from))
    }
    fn ids(&self) -> BTreeSet<(String, String)> {
        self.tocs
            .iter()
            .flat_map(|(_, t)| t.package_ids())
            .collect()
    }
    fn deps(&self, hard_only: bool) -> BTreeSet<String> {
        let mut s = BTreeSet::new();
        for (_, t) in &self.tocs {
            if hard_only {
                for k in ["dependencies", "requireddeps", "dep1"] {
                    if let Some(v) = t.get(k) {
                        s.extend(
                            v.split(',')
                                .map(|x| x.trim().to_string())
                                .filter(|x| !x.is_empty()),
                        );
                    }
                }
            } else {
                s.extend(t.deps());
            }
        }
        s
    }
}

fn load(dir: &Path) -> Vec<Folder> {
    let mut out = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("read dir")
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name().to_string_lossy().to_string();
        let mut f = Folder {
            name: name.clone(),
            tocs: vec![],
            unmatched_toc_names: vec![],
        };
        let mut files: Vec<_> = fs::read_dir(e.path())
            .expect("read folder")
            .filter_map(Result::ok)
            .map(|x| x.file_name().to_string_lossy().to_string())
            .filter(|n| n.to_ascii_lowercase().ends_with(".toc"))
            .collect();
        files.sort();
        for file in files {
            match toc_suffix(&name, &file) {
                Ok(suffix) => {
                    // Read as bytes + lossy: tolerant of non-UTF-8, never executed.
                    let bytes = fs::read(e.path().join(&file)).unwrap_or_default();
                    f.tocs
                        .push((suffix, parse_toc(&String::from_utf8_lossy(&bytes))));
                }
                Err(()) => f.unmatched_toc_names.push(file),
            }
        }
        out.push(f);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_cover_every_scheme() {
        let fs = load(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures")));
        let scheme = |n: &str| {
            let f = fs.iter().find(|f| f.name == n).expect(n);
            parse_version(f.version().as_deref()).scheme
        };
        assert_eq!(scheme("SynSemver"), Scheme::Semver);
        assert_eq!(scheme("SynPre"), Scheme::Semver);
        assert_eq!(scheme("SynDate"), Scheme::Date);
        assert_eq!(scheme("SynDescribe"), Scheme::GitDescribe);
        assert_eq!(scheme("SynToken"), Scheme::Token);
        assert_eq!(scheme("SynMissing"), Scheme::Missing);
        assert_eq!(scheme("SynInteger"), Scheme::Integer);
        assert_eq!(scheme("SynRev"), Scheme::Revision);
        assert_eq!(scheme("SynHash"), Scheme::GitHash);
        assert_eq!(scheme("SynOther"), Scheme::Other);
        assert_eq!(scheme("SynFlavor"), Scheme::NumericDotted);
        let flavor = fs.iter().find(|f| f.name == "SynFlavor").unwrap();
        assert_eq!(flavor.tocs.len(), 3);
        assert_eq!(flavor.versions().len(), 2);
        assert_eq!(flavor.tocs[0].1.interfaces(), vec![11500]);
    }
}

struct Dsu(Vec<usize>);
impl Dsu {
    fn find(&mut self, x: usize) -> usize {
        if self.0[x] != x {
            let r = self.find(self.0[x]);
            self.0[x] = r;
        }
        self.0[x]
    }
    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        self.0[a] = b;
    }
}

fn first_token(n: &str) -> &str {
    n.split(['_', '-']).next().unwrap_or(n)
}

fn name_related(a: &str, b: &str) -> bool {
    let (ta, tb) = (first_token(a), first_token(b));
    if ta.len() >= 3
        && ta.eq_ignore_ascii_case(tb)
        && (a.contains(['_', '-']) || b.contains(['_', '-']) || a == ta || b == tb)
    {
        return true;
    }
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    short.len() >= 4
        && long.len() > short.len()
        && long.starts_with(short)
        && long[short.len()..]
            .chars()
            .next()
            .is_some_and(|c| c == '_' || c == '-' || c.is_ascii_uppercase())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: addon-spike <AddOns dir> [--csv out.csv] [--report out.md]");
        std::process::exit(2);
    }
    let folders = load(Path::new(&args[1]));
    let opt = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
    };
    let n = folders.len();
    let idx: BTreeMap<String, usize> = folders
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.to_ascii_lowercase(), i))
        .collect();

    // Confident groups: shared package id or hard dependency on an installed folder.
    let mut dsu = Dsu((0..n).collect());
    let mut by_id: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (i, f) in folders.iter().enumerate() {
        for id in f.ids() {
            by_id.entry(id).or_default().push(i);
        }
    }
    for v in by_id.values() {
        for w in v.windows(2) {
            dsu.union(w[0], w[1]);
        }
    }
    let mut dep_edges = 0;
    for (i, f) in folders.iter().enumerate() {
        for d in f.deps(true) {
            if let Some(&j) = idx.get(&d.to_ascii_lowercase()) {
                if i != j {
                    dsu.union(i, j);
                    dep_edges += 1;
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..n {
        let r = dsu.find(i);
        groups.entry(r).or_default().push(i);
    }
    let root = |i: usize, d: &mut Dsu| d.find(i);
    // Name-only relatives outside the confident group.
    let mut weak: Vec<usize> = vec![];
    for i in 0..n {
        let ri = root(i, &mut dsu);
        let mut has = false;
        for j in 0..n {
            if i != j && root(j, &mut dsu) != ri && name_related(&folders[i].name, &folders[j].name)
            {
                has = true;
            }
        }
        if has {
            weak.push(i);
        }
    }

    // Per-folder CSV rows.
    let mut csv =
        String::from("folder,toc_files,interface,version,scheme,x_ids,hard_deps,group,groupsize\n");
    let mut scheme_count: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    let mut iface_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut suffix_count: BTreeMap<String, usize> = BTreeMap::new();
    let mut inconsistent = vec![];
    let mut no_toc = vec![];
    let mut unmatched = vec![];
    let mut with_id = 0;
    let mut no_plain = vec![];
    let mut iface_by_suffix: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for (i, f) in folders.iter().enumerate() {
        if f.tocs.is_empty() {
            no_toc.push(f.name.clone());
        }
        if !f.unmatched_toc_names.is_empty() {
            unmatched.push(f.name.clone());
        }
        if !f.tocs.is_empty() && f.tocs.iter().all(|(s, _)| s.is_some()) {
            no_plain.push(f.name.clone());
        }
        let ver = f.version();
        let p = parse_version(ver.as_deref());
        scheme_count
            .entry(p.scheme.name())
            .or_default()
            .push(f.name.clone());
        if f.versions().len() > 1 {
            inconsistent.push(f.name.clone());
        }
        let mut ifs = BTreeSet::new();
        for (s, t) in &f.tocs {
            let key = s.clone().unwrap_or_else(|| "(plain)".into());
            *suffix_count.entry(key.clone()).or_default() += 1;
            for v in t.interfaces() {
                ifs.insert(v);
                iface_by_suffix.entry(key.clone()).or_default().insert(v);
            }
        }
        for v in &ifs {
            *iface_count.entry(v.to_string()).or_default() += 1;
        }
        let ids = f.ids();
        if !ids.is_empty() {
            with_id += 1;
        }
        let g = &groups[&dsu.find(i)];
        let files: Vec<String> = f
            .tocs
            .iter()
            .map(|(s, _)| s.clone().unwrap_or_else(|| "-".into()))
            .collect();
        csv += &format!(
            "{},{},{},{},{},{},{},{},{}\n",
            f.name,
            files.join("|"),
            ifs.iter().map(u32::to_string).collect::<Vec<_>>().join("|"),
            ver.unwrap_or_default().replace(',', ";"),
            p.scheme.name(),
            ids.iter()
                .map(|(k, v)| format!("{}={}", k.trim_start_matches("x-"), v))
                .collect::<Vec<_>>()
                .join("|"),
            f.deps(true).into_iter().collect::<Vec<_>>().join("|"),
            folders[g[0]].name,
            g.len()
        );
    }
    if let Some(p) = opt("--csv") {
        fs::write(p, &csv).expect("write csv");
    }

    let mut rep = String::new();
    let pct = |a: usize| format!("{} ({:.1}%)", a, 100.0 * a as f64 / n.max(1) as f64);
    rep += &format!("# Addon coverage report\n\nFolders scanned: {n}\n\n## Version schemes\n\n| scheme | folders | orderable vs same scheme |\n|---|---|---|\n");
    let mut orderable = 0;
    for (name, v) in &scheme_count {
        let sch = match *name {
            "missing" => Scheme::Missing,
            "unsubstituted-token" => Scheme::Token,
            "semver-like" => Scheme::Semver,
            "numeric-dotted" => Scheme::NumericDotted,
            "date" => Scheme::Date,
            "git-describe" => Scheme::GitDescribe,
            "integer-build" => Scheme::Integer,
            "revision-rN" => Scheme::Revision,
            "git-hash" => Scheme::GitHash,
            "tagged-numeric" => Scheme::Tagged,
            _ => Scheme::Other,
        };
        let o = version::orderable(&sch);
        if o {
            orderable += v.len();
        }
        rep += &format!(
            "| {} | {} | {} |\n",
            name,
            v.len(),
            if o { "yes" } else { "no" }
        );
    }
    rep += &format!(
        "\nFolders with an orderable version string: {}\n",
        pct(orderable)
    );
    rep += &format!(
        "Folders with at least one external package id (X-Curse/WoWI/Wago/GitHub): {}\n",
        pct(with_id)
    );
    rep += &format!(
        "Folders with no external id (folder name is the only key): {}\n",
        pct(n - with_id)
    );
    rep += &format!("Folders without any .toc: {}\n", no_toc.len());
    rep += &format!(
        "Folders with a .toc name not matching the folder: {} {:?}\n",
        unmatched.len(),
        unmatched
    );
    rep += &format!(
        "Folders with only flavor-suffixed tocs (no plain toc): {} {:?}\n",
        no_plain.len(),
        no_plain
    );
    rep += &format!(
        "Folders whose tocs disagree on Version: {} {:?}\n",
        inconsistent.len(),
        inconsistent
    );

    rep += "\n## Version strings per scheme (derived metadata)\n\n";
    for (name, _) in &scheme_count {
        let mut vs: Vec<String> = folders
            .iter()
            .filter(|f| parse_version(f.version().as_deref()).scheme.name() == *name)
            .map(|f| format!("{}={}", f.name, f.version().unwrap_or_default()))
            .collect();
        vs.sort();
        rep += &format!(
            "### {name}\n\n{}\n\n",
            if vs.len() > 60 {
                format!("{} (first 60 of {})", vs[..60].join("; "), vs.len())
            } else {
                vs.join("; ")
            }
        );
    }

    let multi: Vec<&Vec<usize>> = groups.values().filter(|g| g.len() > 1).collect();
    let grouped: usize = multi.iter().map(|g| g.len()).sum();
    rep += &format!("## Package grouping\n\nEvidence: shared X-* package id ({} distinct shared-id keys), hard dependency on an installed folder ({} edges).\n\n", by_id.values().filter(|v| v.len() > 1).count(), dep_edges);
    rep += &format!(
        "Multi-folder groups: {}, covering {} folders. Singletons: {}.\n\n",
        multi.len(),
        pct(grouped),
        n - grouped
    );
    for g in &multi {
        rep += &format!(
            "- {} ({}): {}\n",
            folders[g[0]].name,
            g.len(),
            g.iter()
                .map(|&i| folders[i].name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    rep += "\n### Version consistency inside groups (distinct versions)\n\n";
    for g in &multi {
        let vs: BTreeSet<String> = g
            .iter()
            .map(|&i| folders[i].version().unwrap_or_else(|| "<none>".into()))
            .collect();
        if vs.len() > 1 {
            rep += &format!(
                "- {}: {}\n",
                folders[g[0]].name,
                vs.into_iter().collect::<Vec<_>>().join(" | ")
            );
        }
    }
    rep += &format!("\nFolders that look name-related to a folder outside their confident group (cannot be grouped confidently): {}\n\n", pct(weak.len()));
    for &i in &weak {
        let ri = dsu.find(i);
        let rel: Vec<&str> = (0..n)
            .filter(|&j| {
                j != i && dsu.find(j) != ri && name_related(&folders[i].name, &folders[j].name)
            })
            .map(|j| folders[j].name.as_str())
            .collect();
        rep += &format!("- {} ~ {}\n", folders[i].name, rel.join(", "));
    }

    rep += "\n## Interface values\n\n| interface | folders |\n|---|---|\n";
    for (k, v) in &iface_count {
        rep += &format!("| {k} | {v} |\n");
    }
    rep += "\n## TOC file suffixes (toc files) and Interface values seen in each\n\n| suffix | toc files | interface values |\n|---|---|---|\n";
    for (k, v) in &suffix_count {
        let documented =
            k == "(plain)" || DOCUMENTED_SUFFIXES.contains(&k.to_ascii_lowercase().as_str());
        rep += &format!(
            "| {}{} | {} | {} |\n",
            k,
            if documented { "" } else { " (undocumented)" },
            v,
            iface_by_suffix
                .get(k)
                .map(|s| s.iter().map(u32::to_string).collect::<Vec<_>>().join(", "))
                .unwrap_or_default()
        );
    }
    print!("{rep}");
    if let Some(p) = opt("--report") {
        fs::write(p, &rep).expect("write report");
    }
}
