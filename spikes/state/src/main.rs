//! S-state spike CLI.
//!
//! `state-spike scan <file>...`         print top-level names of saved-variable files
//! `state-spike survey <AddOns> <WTF>`  classify every saved-variable file; output is
//!                                       redacted (account/realm/character replaced)
//!
//! Read-only. Never executes Lua.

use state_spike::classify::{Class, Inventory, Scope, classify};
use state_spike::scanner::scan;
use state_spike::toc::{AddonDecl, parse_toc};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("scan") => {
            for f in &args[1..] {
                match fs::read(f) {
                    Ok(b) => println!("{f}: {:?}", scan(&b)),
                    Err(e) => println!("{f}: io error {e}"),
                }
            }
        }
        Some("survey") if args.len() == 3 => survey(Path::new(&args[1]), Path::new(&args[2])),
        _ => {
            eprintln!("usage: state-spike scan <file>... | survey <AddOns dir> <WTF dir>");
            std::process::exit(2);
        }
    }
}

fn entries(dir: &Path) -> Vec<fs::DirEntry> {
    let mut v: Vec<_> = fs::read_dir(dir)
        .map(|r| r.flatten().collect())
        .unwrap_or_default();
    v.sort_by_key(|e| e.file_name());
    v
}

fn inventory(addons: &Path) -> Inventory {
    let mut inv = Inventory::default();
    for e in entries(addons) {
        if !e.path().is_dir() {
            continue;
        }
        let tocs: Vec<_> = entries(&e.path())
            .into_iter()
            .filter(|t| {
                t.file_name()
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .ends_with(".toc")
            })
            .filter_map(|t| fs::read(t.path()).ok().map(|b| parse_toc(&b)))
            .collect();
        if !tocs.is_empty() {
            inv.insert(
                &e.file_name().to_string_lossy(),
                AddonDecl::from_tocs(&tocs),
            );
        }
    }
    inv
}

#[derive(Default)]
struct Redactor {
    map: BTreeMap<(char, String), usize>,
}

impl Redactor {
    fn get(&mut self, kind: char, name: &str) -> String {
        let n = self.map.keys().filter(|(k, _)| *k == kind).count();
        let id = *self.map.entry((kind, name.to_string())).or_insert(n + 1);
        match kind {
            'A' => format!("<ACCOUNT-{id}>"),
            'R' => format!("<REALM-{id}>"),
            _ => format!("<CHARACTER-{id}>"),
        }
    }
}

struct Ctx {
    inv: Inventory,
    counts: BTreeMap<String, usize>,
    bytes: u64,
    files: usize,
}

impl Ctx {
    fn count(&mut self, key: String) {
        *self.counts.entry(key).or_default() += 1;
    }

    fn sv_dir(&mut self, dir: &Path, scope: Scope, shown: &str) {
        for e in entries(dir) {
            let name = e.file_name().to_string_lossy().to_string();
            let lower = name.to_ascii_lowercase();
            let (stem, bak) = if let Some(s) = lower.strip_suffix(".lua.bak") {
                (&name[..s.len()], true)
            } else if let Some(s) = lower.strip_suffix(".lua") {
                (&name[..s.len()], false)
            } else {
                println!("{scope:?}\t{shown}\\SavedVariables\\<other file>\tunexpected");
                self.count(format!("{scope:?}\tunexpected-file"));
                continue;
            };
            let bytes = fs::read(e.path()).unwrap_or_default();
            self.bytes += bytes.len() as u64;
            self.files += 1;
            let result = scan(&bytes);
            let o = classify(&self.inv, scope, stem, &result);
            let mut flags = Vec::new();
            if o.flavor_dependent {
                flags.push("flavor-dependent".to_string());
            }
            if o.case_mismatch {
                flags.push("case-mismatch".to_string());
            }
            let detail = match &o.class {
                Class::Shared { others, .. } => format!("others={}", others.join(",")),
                Class::Unknown(r) => format!("{r:?}"),
                _ => String::new(),
            };
            let shown_stem = if o.class == Class::Orphan {
                "<orphan>"
            } else {
                stem
            };
            let ext = if bak { ".lua.bak" } else { ".lua" };
            println!(
                "{scope:?}\t{shown}\\SavedVariables\\{shown_stem}{ext}\t{}\t{}\t{detail}",
                o.class.label(),
                flags.join(",")
            );
            let kind = if bak { "bak" } else { "lua" };
            self.count(format!("{scope:?}\t{kind}\t{}", o.class.label()));
            if o.flavor_dependent {
                self.count(format!("{scope:?}\t{kind}\tflag:flavor-dependent"));
            }
        }
    }
}

fn client_file(ctx: &mut Ctx, scope: &str, shown: &str, name: &str) {
    println!("{scope}\t{shown}\\{name}\tclient-file");
    ctx.count(format!("{scope}\tclient-file"));
}

fn survey(addons: &Path, wtf: &Path) {
    let start = Instant::now();
    let mut ctx = Ctx {
        inv: inventory(addons),
        counts: BTreeMap::new(),
        bytes: 0,
        files: 0,
    };
    let declaring = ctx
        .inv
        .folders
        .values()
        .filter(|(_, d)| d.union != Default::default())
        .count();
    let flavor = ctx
        .inv
        .folders
        .values()
        .filter(|(_, d)| d.flavor_dependent())
        .count();
    let mut red = Redactor::default();
    for e in entries(wtf) {
        let name = e.file_name().to_string_lossy().to_string();
        match (e.path().is_dir(), name.as_str()) {
            (true, "SavedVariables") => ctx.sv_dir(&e.path(), Scope::Machine, "WTF"),
            (true, "Account") => {
                for a in entries(&e.path()) {
                    let an = a.file_name().to_string_lossy().to_string();
                    if !a.path().is_dir() {
                        client_file(&mut ctx, "Account", "WTF\\Account", &an);
                        continue;
                    }
                    let ashown = format!("WTF\\Account\\{}", red.get('A', &an));
                    for r in entries(&a.path()) {
                        let rn = r.file_name().to_string_lossy().to_string();
                        if !r.path().is_dir() {
                            client_file(&mut ctx, "Account", &ashown, &rn);
                        } else if rn == "SavedVariables" {
                            ctx.sv_dir(&r.path(), Scope::Account, &ashown);
                        } else {
                            let rshown = format!("{ashown}\\{}", red.get('R', &rn));
                            for c in entries(&r.path()) {
                                let cn = c.file_name().to_string_lossy().to_string();
                                if !c.path().is_dir() {
                                    client_file(&mut ctx, "Realm", &rshown, &cn);
                                    continue;
                                }
                                let cshown = format!("{rshown}\\{}", red.get('C', &cn));
                                for f in entries(&c.path()) {
                                    let fname = f.file_name().to_string_lossy().to_string();
                                    if f.path().is_dir() && fname == "SavedVariables" {
                                        ctx.sv_dir(&f.path(), Scope::Character, &cshown);
                                    } else if f.path().is_dir() {
                                        println!("Character\t{cshown}\\<dir>\tunexpected");
                                        ctx.count("Character\tunexpected-dir".into());
                                    } else {
                                        client_file(&mut ctx, "Character", &cshown, &fname);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            (true, _) => {
                println!("Root\tWTF\\<dir>\tunexpected");
                ctx.count("Root\tunexpected-dir".into());
            }
            (false, _) => client_file(&mut ctx, "Root", "WTF", &name),
        }
    }
    println!();
    println!("# addon folders with .toc: {}", ctx.inv.folders.len());
    println!("# folders declaring saved variables: {declaring}");
    println!("# folders whose declarations differ across flavor .toc files: {flavor}");
    println!(
        "# saved-variable files scanned: {} ({} bytes) in {:?}",
        ctx.files,
        ctx.bytes,
        start.elapsed()
    );
    for (k, v) in &ctx.counts {
        println!("{v}\t{k}");
    }
}
