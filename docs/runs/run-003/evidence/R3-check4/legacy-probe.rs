use wow_sync_core::records::{decode, encode, ApplyJournal, JournalPhase, Record};
use wow_sync_core::status::{game_halted, journal_status};

fn main() {
    let path = std::env::args().nth(1).expect("legacy journal path");
    let bytes = std::fs::read(path).expect("read retained pre-fix2 journal");
    assert!(!String::from_utf8_lossy(&bytes).contains("halted_by_game"));
    let journal: ApplyJournal = decode(&bytes, ApplyJournal::LIMIT).expect("legacy decode");
    assert_eq!(journal.package.as_str(), "labbeta");
    assert_eq!(journal.phase, JournalPhase::Staging);
    assert!(!journal.halted_by_game);
    assert!(game_halted(std::slice::from_ref(&journal)).is_empty());
    let (interrupted, help) = journal_status(std::slice::from_ref(&journal));
    assert_eq!(interrupted.len(), 1);
    assert!(help.is_empty());
    let roundtrip: ApplyJournal =
        decode(&encode(&journal), ApplyJournal::LIMIT).expect("legacy roundtrip");
    assert_eq!(roundtrip, journal);
    let mut recorded = journal;
    recorded.halted_by_game = true;
    let back: ApplyJournal =
        decode(&encode(&recorded), ApplyJournal::LIMIT).expect("recorded roundtrip");
    assert!(back.halted_by_game);
    assert_eq!(game_halted(&[back]).len(), 1);
    println!("PASS retained R3-check3 pre-fix2 journal decodes, remains interrupted with unknown cause, roundtrips; explicit recorded cause survives separately.");
}
