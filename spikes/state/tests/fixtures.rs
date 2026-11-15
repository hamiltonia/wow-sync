use state_spike::classify::{Class, Inventory, Reason, Scope, classify};
use state_spike::scanner::{ScanError, scan};
use state_spike::toc::{AddonDecl, parse_toc};
use std::path::PathBuf;
use std::time::Instant;

fn fixture(name: &str) -> Vec<u8> {
    let p: PathBuf = [env!("CARGO_MANIFEST_DIR"), "fixtures", name]
        .iter()
        .collect();
    std::fs::read(p).unwrap()
}

fn names(name: &str) -> Vec<String> {
    scan(&fixture(name)).unwrap().names
}

#[test]
fn simple_literals() {
    let s = scan(&fixture("simple.lua")).unwrap();
    assert_eq!(
        s.names,
        [
            "SynthDB",
            "SynthOther",
            "SynthFlag",
            "SynthNumber",
            "SynthText"
        ]
    );
    assert!(s.non_literal.is_empty() && s.duplicates.is_empty());
    assert_eq!(s.max_depth, 3);
}

#[test]
fn long_strings_hide_fake_assignments() {
    assert_eq!(names("long_strings.lua"), ["SynthDB", "SynthTail"]);
}

#[test]
fn comments_hide_fake_assignments() {
    assert_eq!(names("comments.lua"), ["SynthDB"]);
}

#[test]
fn escaped_quotes() {
    let s = scan(&fixture("escaped_quotes.lua")).unwrap();
    assert_eq!(s.names, ["SynthDB", "SynthTail"]);
    assert!(s.non_literal.is_empty());
}

#[test]
fn nested_and_numeric_forms() {
    let s = scan(&fixture("nested.lua")).unwrap();
    assert_eq!(s.names, ["SynthDB"]);
    assert!(s.non_literal.is_empty(), "{:?}", s.non_literal);
}

#[test]
fn code_is_reported_not_accepted() {
    let s = scan(&fixture("code_in_table.lua")).unwrap();
    assert!(s.non_literal.iter().any(|(_, t)| t == "function"));
    let s = scan(&fixture("table_name_value.lua")).unwrap();
    assert_eq!(s.non_literal.len(), 1);
    for f in [
        "code_top_level.lua",
        "local_top_level.lua",
        "name_value.lua",
    ] {
        assert!(
            matches!(scan(&fixture(f)), Err(ScanError::UnexpectedTopLevel { .. })),
            "{f}"
        );
    }
}

#[test]
fn malformed_files_fail_closed() {
    assert!(matches!(
        scan(&fixture("unterminated_string.lua")),
        Err(ScanError::UnterminatedString { .. })
    ));
    assert!(matches!(
        scan(&fixture("unterminated_long.lua")),
        Err(ScanError::UnterminatedLongBracket { .. })
    ));
    assert!(matches!(
        scan(&fixture("unbalanced.lua")),
        Err(ScanError::UnbalancedBracket { .. })
    ));
    assert!(matches!(
        scan(&fixture("mismatched.lua")),
        Err(ScanError::UnbalancedBracket { .. })
    ));
    assert!(matches!(
        scan(b"SynthDB = {} \x01"),
        Err(ScanError::InvalidByte { .. })
    ));
    assert!(matches!(
        scan(b"SynthDB ="),
        Err(ScanError::MissingValue { .. })
    ));
}

#[test]
fn duplicates_and_empty() {
    let s = scan(&fixture("duplicate.lua")).unwrap();
    assert_eq!(s.duplicates, ["SynthDB"]);
    assert!(scan(&fixture("comment_only.lua")).unwrap().names.is_empty());
    assert!(scan(b"").unwrap().names.is_empty());
}

#[test]
fn bom_and_crlf() {
    let s = scan(b"\xEF\xBB\xBFSynthDB = {\r\n\t[\"a\"] = \"x\\\r\ny\",\r\n}\r\nSynthB = 1\r\n")
        .unwrap();
    assert_eq!(s.names, ["SynthDB", "SynthB"]);
}

#[test]
fn utf8_inside_strings_only() {
    assert!(scan("SynthDB = { [\"n\"] = \"Ünïcödé\" }".as_bytes()).is_ok());
    assert!(matches!(
        scan("Ünï = 1".as_bytes()),
        Err(ScanError::InvalidByte { .. })
    ));
}

#[test]
fn huge_and_deep_files_are_linear_and_stack_safe() {
    let mut big = String::from("SynthDB = {\n");
    for i in 0..400_000 {
        big.push_str(&format!(
            "\t[\"k{i}\"] = {{ [1] = \"v \\\" {i}\", [2] = [[x]], [3] = -{i}.5e3 }}, -- c\n"
        ));
    }
    big.push_str("}\nSynthTail = 1\n");
    let t = Instant::now();
    let s = scan(big.as_bytes()).unwrap();
    let elapsed = t.elapsed();
    assert_eq!(s.names, ["SynthDB", "SynthTail"]);
    eprintln!("huge: {} bytes in {elapsed:?}", big.len());

    let depth = 200_000;
    let deep = format!("SynthDB = {}{}", "{".repeat(depth), "}".repeat(depth));
    let s = scan(deep.as_bytes()).unwrap();
    assert_eq!(s.max_depth, depth);
}

fn decl(toc: &str) -> AddonDecl {
    AddonDecl::from_tocs(&[parse_toc(toc.as_bytes())])
}

fn inv() -> Inventory {
    let mut inv = Inventory::default();
    inv.insert("SynthAddon", decl("## Title: x\n## SavedVariables: SynthDB, SynthGlobal\n## SavedVariablesPerCharacter: SynthCharDB\n"));
    inv.insert(
        "SynthModule",
        decl("## Dependencies: SynthAddon\n## SavedVariables: SynthModuleDB\n"),
    );
    inv.insert("SynthNoSV", decl("## Title: no saved variables\n"));
    inv.insert("SynthCollider", decl("## SavedVariables: SynthGlobal\n"));
    inv.insert(
        "SynthFlavor",
        AddonDecl::from_tocs(&[
            parse_toc(b"## SavedVariables: FlavorDB, FlavorExtra\n"),
            parse_toc(b"\xEF\xBB\xBF## savedvariables :FlavorDB\r\n"),
        ]),
    );
    inv
}

fn cls(scope: Scope, stem: &str, src: &str) -> Class {
    classify(&inv(), scope, stem, &scan(src.as_bytes())).class
}

#[test]
fn toc_parsing() {
    let d = parse_toc(b"\xEF\xBB\xBF## Interface: 11508\r\n##SavedVariables:  A ,B,, C \r\n# ## SavedVariables: X\n");
    assert_eq!(d.account.iter().collect::<Vec<_>>(), ["A", "B", "C"]);
    assert!(d.character.is_empty());
}

#[test]
fn classification_rules() {
    let owned = Class::Owned {
        folder: "SynthAddon".into(),
    };
    assert_eq!(cls(Scope::Account, "SynthAddon", "SynthDB = {}"), owned);
    assert_eq!(
        cls(Scope::Character, "SynthAddon", "SynthCharDB = {}"),
        owned
    );
    assert_eq!(
        cls(Scope::Account, "SynthAddon", ""),
        Class::OwnedEmpty {
            folder: "SynthAddon".into()
        }
    );
    assert_eq!(
        cls(Scope::Account, "SynthModule", "SynthModuleDB = {}"),
        Class::Owned {
            folder: "SynthModule".into()
        }
    );
    assert_eq!(
        cls(Scope::Account, "SynthAddon", "SynthGlobal = 1"),
        Class::Shared {
            folder: "SynthAddon".into(),
            others: vec!["SynthCollider".into()]
        }
    );
    assert_eq!(
        cls(Scope::Account, "SynthAddon", "SynthCharDB = {}"),
        Class::Unknown(Reason::WrongScope(vec!["SynthCharDB".into()]))
    );
    assert_eq!(
        cls(Scope::Account, "SynthAddon", "SynthDB = {} Stray = 1"),
        Class::Unknown(Reason::UndeclaredNames(vec!["Stray".into()]))
    );
    assert_eq!(
        cls(Scope::Account, "SynthNoSV", "X = 1"),
        Class::Unknown(Reason::NoDeclaration)
    );
    assert_eq!(
        cls(Scope::Account, "RemovedAddon", "RemovedDB = {}"),
        Class::Orphan
    );
    assert_eq!(
        cls(Scope::Account, "RenamedAddon", "SynthModuleDB = {}"),
        Class::Unknown(Reason::DeclaredByOtherFolder(vec!["SynthModule".into()]))
    );
    assert_eq!(
        cls(Scope::Account, "Blizzard_Synth", "X = 1"),
        Class::Blizzard
    );
    assert_eq!(
        cls(Scope::Machine, "SynthAddon", "SynthDB = {}"),
        Class::Unknown(Reason::ThirdPartyMachineScope)
    );
    assert_eq!(
        cls(
            Scope::Account,
            "SynthAddon",
            "SynthDB = { f = function() end }"
        ),
        Class::Unknown(Reason::NonLiteral)
    );
    assert!(matches!(
        cls(Scope::Account, "SynthAddon", "SynthDB = {"),
        Class::Unknown(Reason::Parse(_))
    ));
}

#[test]
fn flavor_and_case_flags() {
    let i = inv();
    let o = classify(&i, Scope::Account, "SynthFlavor", &scan(b"FlavorDB = {}"));
    assert!(!o.flavor_dependent);
    let o = classify(
        &i,
        Scope::Account,
        "SynthFlavor",
        &scan(b"FlavorExtra = {}"),
    );
    assert!(o.flavor_dependent);
    assert_eq!(
        o.class,
        Class::Owned {
            folder: "SynthFlavor".into()
        }
    );
    let o = classify(&i, Scope::Account, "synthaddon", &scan(b"SynthDB = {}"));
    assert!(o.case_mismatch);
    assert_eq!(
        o.class,
        Class::Owned {
            folder: "SynthAddon".into()
        }
    );
}
