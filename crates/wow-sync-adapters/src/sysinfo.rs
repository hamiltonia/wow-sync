//! Clock and random identifiers.

pub fn now_unix_ms() -> i64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_millis() as i64,
        Err(e) => -(e.duration().as_millis() as i64),
    }
}

/// 16 random bytes as 32 lowercase hex characters. Panics if the OS random
/// source fails; an unpredictable id cannot be replaced by a weaker one.
pub fn random_id_hex() -> String {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).expect("operating system random source failed");
    crate::hex(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_ids_are_32_hex_and_distinct() {
        let ids: Vec<String> = (0..64).map(|_| random_id_hex()).collect();
        for id in &ids {
            assert_eq!(id.len(), 32);
            assert!(id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
        }
        let mut s = ids.clone();
        s.sort();
        s.dedup();
        assert_eq!(s.len(), ids.len());
        assert!(now_unix_ms() > 1_700_000_000_000);
    }
}
