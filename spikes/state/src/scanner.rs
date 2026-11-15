//! Non-executing scanner for WoW saved-variable files.
//!
//! The scanner tokenizes Lua 5.1 lexical syntax and accepts only the data
//! shape the client writes: a sequence of top-level `Name = <literal|table>`
//! statements. It never evaluates anything. Anything outside that shape is
//! reported so the caller can classify the file as unknown instead of guessing.

use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanError {
    UnterminatedString { offset: usize },
    UnterminatedLongBracket { offset: usize },
    UnbalancedBracket { offset: usize },
    InvalidByte { offset: usize },
    UnexpectedTopLevel { offset: usize, found: String },
    MissingValue { offset: usize },
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Scan {
    /// Top-level assigned names in file order (first occurrence only).
    pub names: Vec<String>,
    /// Names assigned more than once at top level.
    pub duplicates: Vec<String>,
    /// Tokens inside table values that are not plain data (capped at 16).
    pub non_literal: Vec<(usize, String)>,
    /// Deepest table nesting (top-level table = 1).
    /// Deepest table nesting (top-level table = 1).
    pub max_depth: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Name,
    Str,
    Num,
    Op,
}

#[derive(Debug, Clone, Copy)]
struct Tok {
    kind: Kind,
    start: usize,
    end: usize,
}

const KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in",
    "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while",
];

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a [u8]) -> Self {
        let pos = if src.starts_with(b"\xEF\xBB\xBF") {
            3
        } else {
            0
        };
        Lexer { src, pos }
    }

    fn peek(&self, n: usize) -> Option<u8> {
        self.src.get(self.pos + n).copied()
    }

    /// At `[`, returns the level of a long bracket opener (`[[`, `[=[`, ...).
    fn long_bracket_level(&self, at: usize) -> Option<usize> {
        let mut i = at + 1;
        while self.src.get(i) == Some(&b'=') {
            i += 1;
        }
        (self.src.get(i) == Some(&b'[')).then_some(i - at - 1)
    }

    fn skip_long_bracket(&mut self, level: usize) -> Result<(), ScanError> {
        let open = self.pos;
        self.pos += level + 2;
        while self.pos < self.src.len() {
            if self.src[self.pos] == b']' {
                let mut i = self.pos + 1;
                while i < self.src.len() && self.src[i] == b'=' {
                    i += 1;
                }
                if i - self.pos - 1 == level && self.src.get(i) == Some(&b']') {
                    self.pos = i + 1;
                    return Ok(());
                }
            }
            self.pos += 1;
        }
        Err(ScanError::UnterminatedLongBracket { offset: open })
    }

    fn next(&mut self) -> Result<Option<Tok>, ScanError> {
        loop {
            let Some(c) = self.peek(0) else {
                return Ok(None);
            };
            match c {
                b' ' | b'\t' | b'\r' | b'\n' | 0x0B | 0x0C => self.pos += 1,
                b'-' if self.peek(1) == Some(b'-') => {
                    self.pos += 2;
                    if self.peek(0) == Some(b'[')
                        && let Some(level) = self.long_bracket_level(self.pos)
                    {
                        self.skip_long_bracket(level)?;
                        continue;
                    }
                    while let Some(c) = self.peek(0) {
                        if c == b'\n' || c == b'\r' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
        let start = self.pos;
        let c = self.src[start];
        let kind = match c {
            b'"' | b'\'' => {
                self.pos += 1;
                loop {
                    match self.peek(0) {
                        None | Some(b'\n') | Some(b'\r') => {
                            return Err(ScanError::UnterminatedString { offset: start });
                        }
                        Some(b'\\') => {
                            let esc = self.peek(1);
                            self.pos += 2;
                            if esc == Some(b'\r') && self.peek(0) == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        Some(q) if q == c => {
                            self.pos += 1;
                            break;
                        }
                        Some(_) => self.pos += 1,
                    }
                }
                Kind::Str
            }
            b'[' if self.long_bracket_level(start).is_some() => {
                let level = self.long_bracket_level(start).unwrap_or(0);
                self.skip_long_bracket(level)?;
                Kind::Str
            }
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                while matches!(
                    self.peek(0),
                    Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
                ) {
                    self.pos += 1;
                }
                Kind::Name
            }
            b'0'..=b'9' => {
                self.lex_number();
                Kind::Num
            }
            b'.' if matches!(self.peek(1), Some(b'0'..=b'9')) => {
                self.lex_number();
                Kind::Num
            }
            _ => {
                let rest = &self.src[start..];
                let len = if rest.starts_with(b"...") {
                    3
                } else if [
                    &b"=="[..],
                    b"~=",
                    b"<=",
                    b">=",
                    b"..",
                    b"::",
                    b"//",
                    b"<<",
                    b">>",
                ]
                .iter()
                .any(|op| rest.starts_with(op))
                {
                    2
                } else if b"+-*/%^#&~|<>=(){}[];:,.".contains(&c) {
                    1
                } else {
                    return Err(ScanError::InvalidByte { offset: start });
                };
                self.pos += len;
                Kind::Op
            }
        };
        Ok(Some(Tok {
            kind,
            start,
            end: self.pos,
        }))
    }

    fn lex_number(&mut self) {
        let hex = self.peek(0) == Some(b'0') && matches!(self.peek(1), Some(b'x' | b'X'));
        while let Some(c) = self.peek(0) {
            if c.is_ascii_alphanumeric() || c == b'.' || c == b'_' {
                self.pos += 1;
                let exp = if hex {
                    matches!(c, b'p' | b'P')
                } else {
                    matches!(c, b'e' | b'E')
                };
                if exp && matches!(self.peek(0), Some(b'+' | b'-')) {
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }
}

fn text<'a>(src: &'a [u8], t: &Tok) -> &'a str {
    std::str::from_utf8(&src[t.start..t.end]).unwrap_or("?")
}

fn is_op(src: &[u8], t: &Option<Tok>, op: &str) -> bool {
    matches!(t, Some(t) if t.kind == Kind::Op && text(src, t) == op)
}

/// Scans a saved-variable file without executing it.
pub fn scan(src: &[u8]) -> Result<Scan, ScanError> {
    let mut lx = Lexer::new(src);
    let mut out = Scan::default();
    let mut seen = HashSet::new();
    let mut dup_seen = HashSet::new();
    let mut pending: Option<Tok> = None;
    let next = |lx: &mut Lexer, pending: &mut Option<Tok>| -> Result<Option<Tok>, ScanError> {
        match pending.take() {
            Some(t) => Ok(Some(t)),
            None => lx.next(),
        }
    };

    loop {
        let Some(t) = next(&mut lx, &mut pending)? else {
            return Ok(out);
        };
        if t.kind == Kind::Op && text(src, &t) == ";" {
            continue;
        }
        let name = text(src, &t);
        if t.kind != Kind::Name || KEYWORDS.contains(&name) {
            return Err(ScanError::UnexpectedTopLevel {
                offset: t.start,
                found: name.to_string(),
            });
        }
        let eq = next(&mut lx, &mut pending)?;
        if !is_op(src, &eq, "=") {
            let found = eq
                .map(|e| text(src, &e).to_string())
                .unwrap_or_else(|| "<eof>".into());
            return Err(ScanError::UnexpectedTopLevel {
                offset: t.start,
                found: format!("{name} {found}"),
            });
        }
        if seen.insert(name.to_string()) {
            out.names.push(name.to_string());
        } else if dup_seen.insert(name.to_string()) {
            out.duplicates.push(name.to_string());
        }

        let Some(v) = next(&mut lx, &mut pending)? else {
            return Err(ScanError::MissingValue { offset: t.start });
        };
        let vt = text(src, &v);
        match v.kind {
            Kind::Str | Kind::Num => {}
            Kind::Name if matches!(vt, "nil" | "true" | "false") => {}
            Kind::Op if vt == "-" => {
                let n = next(&mut lx, &mut pending)?;
                if !matches!(
                    n,
                    Some(Tok {
                        kind: Kind::Num,
                        ..
                    })
                ) {
                    return Err(ScanError::UnexpectedTopLevel {
                        offset: v.start,
                        found: "-".into(),
                    });
                }
            }
            Kind::Op if vt == "{" => scan_table(src, &mut lx, v, &mut out)?,
            _ => {
                return Err(ScanError::UnexpectedTopLevel {
                    offset: v.start,
                    found: vt.to_string(),
                });
            }
        }
    }
}

/// Consumes a balanced table constructor whose `{` has been read.
fn scan_table(src: &[u8], lx: &mut Lexer, open: Tok, out: &mut Scan) -> Result<(), ScanError> {
    let mut stack: Vec<u8> = vec![b'{'];
    let mut tables = 1;
    out.max_depth = out.max_depth.max(tables);
    let mut prev_name: Option<Tok> = None;
    let note = |out: &mut Scan, t: &Tok| {
        if out.non_literal.len() < 16 {
            out.non_literal.push((t.start, text(src, t).to_string()));
        }
    };
    while !stack.is_empty() {
        let Some(t) = lx.next()? else {
            return Err(ScanError::UnbalancedBracket { offset: open.start });
        };
        let tt = text(src, &t);
        if let Some(n) = prev_name.take() {
            // A bare identifier inside a table is only data when it is a key (`name =`).
            if !(t.kind == Kind::Op && tt == "=") {
                note(out, &n);
            }
        }
        match t.kind {
            Kind::Str | Kind::Num => {}
            Kind::Name => match tt {
                "nil" | "true" | "false" => {}
                _ if KEYWORDS.contains(&tt) => note(out, &t),
                _ => prev_name = Some(t),
            },
            Kind::Op => match tt {
                "{" | "[" | "(" => {
                    if tt == "(" {
                        note(out, &t);
                    }
                    stack.push(tt.as_bytes()[0]);
                    if tt == "{" {
                        tables += 1;
                        out.max_depth = out.max_depth.max(tables);
                    }
                }
                "}" | "]" | ")" => {
                    let want = match tt {
                        "}" => b'{',
                        "]" => b'[',
                        _ => b'(',
                    };
                    if want == b'{' {
                        tables -= 1;
                    }
                    if stack.pop() != Some(want) {
                        return Err(ScanError::UnbalancedBracket { offset: t.start });
                    }
                }
                "=" | "," | ";" | "-" => {}
                _ => note(out, &t),
            },
        }
    }
    if let Some(n) = prev_name {
        note(out, &n);
    }
    Ok(())
}
