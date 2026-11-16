//! Parser for Blizzard pipe-separated metadata (`.build.info`, `.flavor.info`).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bpsv {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BpsvError {
    TooLarge,
    NotUtf8,
    NoHeader,
    Unparseable(String),
}

const LIMIT: usize = 1024 * 1024;

pub fn parse_bpsv(bytes: &[u8]) -> Result<Bpsv, BpsvError> {
    if bytes.len() > LIMIT {
        return Err(BpsvError::TooLarge);
    }
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| BpsvError::NotUtf8)?;
    let mut columns: Option<Vec<String>> = None;
    let mut rows = Vec::new();
    for (i, raw) in text.split('\n').enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() || line.starts_with("##") {
            continue;
        }
        let fields: Vec<&str> = line.split('|').collect();
        match &columns {
            None => {
                let names: Vec<String> = fields
                    .iter()
                    .map(|f| f.split('!').next().unwrap_or("").to_string())
                    .collect();
                columns = Some(names);
            }
            Some(cols) => {
                if fields.len() != cols.len() {
                    return Err(BpsvError::Unparseable(format!(
                        "line {} has {} fields, header has {}",
                        i + 1,
                        fields.len(),
                        cols.len()
                    )));
                }
                rows.push(fields.into_iter().map(str::to_string).collect());
            }
        }
    }
    let columns = columns.ok_or(BpsvError::NoHeader)?;
    Ok(Bpsv { columns, rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_and_rows() {
        let src = b"\xEF\xBB\xBF## seqn = 1\r\nProduct!STRING:0|Active!DEC:1|Version!STRING:0\r\n\r\nwowf|1|1.15.8\r\nwow|0|\r\n";
        let b = parse_bpsv(src).unwrap();
        assert_eq!(b.columns, vec!["Product", "Active", "Version"]);
        assert_eq!(b.rows.len(), 2);
        assert_eq!(b.rows[0], vec!["wowf", "1", "1.15.8"]);
        assert_eq!(b.rows[1][2], "");
    }

    #[test]
    fn wrong_field_count_is_unparseable() {
        assert!(matches!(
            parse_bpsv(b"A!STRING:0|B!STRING:0\nx\n"),
            Err(BpsvError::Unparseable(_))
        ));
        assert_eq!(parse_bpsv(b"## only comments\n"), Err(BpsvError::NoHeader));
        assert_eq!(parse_bpsv(&[0xff, 0xfe]), Err(BpsvError::NotUtf8));
        assert_eq!(parse_bpsv(&vec![b'a'; LIMIT + 1]), Err(BpsvError::TooLarge));
    }
}
