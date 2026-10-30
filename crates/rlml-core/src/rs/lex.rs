//! RogueScript tokens (RS 2.2, RS 12).
//!
//! * A line is one statement; indentation by four spaces opens a block
//!   (`Indent`) and going back closes it (`Dedent`). A tab is a load error.
//!   Blank lines and `//` comments are skipped. A line does not continue
//!   inside brackets: RS 2.2 has no line continuation.
//! * Names are upper case (`HP`, `MOVE`), vocabulary and local names lower
//!   case (`actor`, `now`); a dotted name is several tokens joined by `.`.
//!   Keywords are lower-case words, and the parser decides by place, so the
//!   context words `map`, `ai`, `from` and `count` stay words (RS 2.4).
//! * Literals: `12`, `1d20`, `500ms`, `1s`, `50%`, `40x20` and `"text"`.
//!   A minus sign is always an operator; the parser makes `-1` and `-1d3`.
//! * `ITEM.*` and `ITEM.**` written without spaces are wildcards (RS 12.3).

use crate::error::Diagnostic;
use crate::syntax::RawCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    /// Upper-case name part: `HP`, `MOVE`, `GLYPH`.
    Name(String),
    /// Lower-case word: keyword, local name, field or engine name.
    Word(String),
    Int(i64),
    /// `NdM`.
    Dice(u32, u32),
    /// Milliseconds; `1s` is 1000 (written as in the source in `text`).
    Time(u64),
    Percent(i64),
    Size(u32, u32),
    Str(String),
    /// Operators and punctuation: `==`, `+=`, `->`, `..`, `(`, `:`, `.`, …
    Op(&'static str),
    /// `.*` right after a name.
    Wild,
    /// `.**` right after a name.
    WildDeep,
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub tok: Tok,
    /// The token as written.
    pub text: String,
    pub line: usize,
    /// 1-based character column.
    pub col: usize,
}

/// Longest first, so `**` wins over `*` and `..` over `.`.
const OPS: &[&str] = &[
    "==", "!=", "<=", ">=", "+=", "-=", "->", "..", "(", ")", "[", "]", "{", "}", ",", ":", ".", "+", "-", "*", "/", "%", "<", ">", "=",
    "|",
];

/// Splits a code block into tokens. Problems are reported and skipped, so
/// one bad character does not hide the rest.
pub fn lex(code: &RawCode) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    let mut indents = vec![0usize];
    let mut last_line = code.line;
    for (i, raw) in code.lines.iter().enumerate() {
        let line = code.line + i;
        let err = |col: usize, m: String| Diagnostic::error(&code.file, line, format!("column {col}: {m}"));
        let body = strip_comment(raw);
        if body.trim().is_empty() {
            continue;
        }
        if let Some(col) = raw.find('\t').filter(|&b| b < body.len()) {
            errors.push(err(raw[..col].chars().count() + 1, "tab in RogueScript: indent with four spaces (RS 2.2)".into()));
            continue;
        }
        last_line = line;
        let indent = body.len() - body.trim_start().len();
        let top = *indents.last().unwrap();
        if indent > top {
            if indent != top + 4 {
                errors.push(err(indent + 1, format!("a block is indented by four spaces more than its header, not {}", indent - top)));
            }
            indents.push(indent);
            out.push(Token { tok: Tok::Indent, text: String::new(), line, col: 1 });
        } else {
            while indent < *indents.last().unwrap() {
                indents.pop();
                out.push(Token { tok: Tok::Dedent, text: String::new(), line, col: 1 });
            }
            if indent != *indents.last().unwrap() {
                errors.push(err(indent + 1, "the indentation does not match any open block".into()));
                indents.push(indent);
            }
        }
        line_tokens(body, indent, line, &mut out, &mut errors, &code.file);
        out.push(Token { tok: Tok::Newline, text: String::new(), line, col: body.chars().count() + 1 });
    }
    while indents.len() > 1 {
        indents.pop();
        out.push(Token { tok: Tok::Dedent, text: String::new(), line: last_line, col: 1 });
    }
    out.push(Token { tok: Tok::Eof, text: String::new(), line: last_line, col: 1 });
    (out, errors)
}

/// The line without a `//` comment outside strings.
fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut prev = '\0';
    for (i, c) in line.char_indices() {
        match c {
            '"' if prev != '\\' => in_str = !in_str,
            '/' if !in_str && prev == '/' => return &line[..i - 1],
            _ => {}
        }
        prev = if prev == '\\' && c == '\\' { '\0' } else { c };
    }
    line
}

fn line_tokens(body: &str, start: usize, line: usize, out: &mut Vec<Token>, errors: &mut Vec<Diagnostic>, file: &str) {
    let chars: Vec<(usize, char)> = body.char_indices().collect();
    let col_of = |idx: usize| idx + 1;
    let mut i = chars.iter().position(|(b, _)| *b >= start).unwrap_or(chars.len());
    let push = |out: &mut Vec<Token>, tok: Tok, from: usize, to: usize| {
        let text: String = chars[from..to].iter().map(|(_, c)| c).collect();
        out.push(Token { tok, text, line, col: col_of(from) });
    };
    while i < chars.len() {
        let c = chars[i].1;
        if c == ' ' {
            i += 1;
            continue;
        }
        let from = i;
        let err = |m: String| Diagnostic::error(file, line, format!("column {}: {m}", col_of(from)));
        if c == '"' {
            let mut s = String::new();
            i += 1;
            let mut closed = false;
            while i < chars.len() {
                match chars[i].1 {
                    '\\' if i + 1 < chars.len() && matches!(chars[i + 1].1, '"' | '\\') => {
                        s.push(chars[i + 1].1);
                        i += 2;
                    }
                    '"' => {
                        closed = true;
                        i += 1;
                        break;
                    }
                    ch => {
                        s.push(ch);
                        i += 1;
                    }
                }
            }
            if !closed {
                errors.push(err("unterminated string: text stays on one line".into()));
            }
            push(out, Tok::Str(s), from, i);
            continue;
        }
        if c.is_ascii_digit() {
            while i < chars.len() && (chars[i].1.is_ascii_alphanumeric() || chars[i].1 == '_') {
                i += 1;
            }
            // `5%` takes the sign; `1..20` stops at the dots
            if i < chars.len() && chars[i].1 == '%' {
                i += 1;
            }
            let word: String = chars[from..i].iter().map(|(_, c)| c).collect();
            match number(&word) {
                Ok(tok) => push(out, tok, from, i),
                Err(m) => {
                    errors.push(err(m));
                    push(out, Tok::Int(0), from, i);
                }
            }
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            while i < chars.len() && (chars[i].1.is_ascii_alphanumeric() || chars[i].1 == '_') {
                i += 1;
            }
            let word: String = chars[from..i].iter().map(|(_, c)| c).collect();
            let tok = if crate::names::is_part(&word) {
                Tok::Name(word.clone())
            } else if crate::names::is_vocabulary(&word) {
                Tok::Word(word.clone())
            } else {
                errors.push(err(format!("`{word}`: names are upper case (`HP`) and other words lower case (`now`) (D 2.1)")));
                Tok::Word(word.to_lowercase())
            };
            push(out, tok, from, i);
            // a wildcard follows a name directly: `ITEM.*`, `LOOT.**`
            if i + 1 < chars.len() && chars[i].1 == '.' && chars[i + 1].1 == '*' {
                let deep = i + 2 < chars.len() && chars[i + 2].1 == '*';
                let to = i + if deep { 3 } else { 2 };
                push(out, if deep { Tok::WildDeep } else { Tok::Wild }, i, to);
                i = to;
            }
            continue;
        }
        let rest: String = chars[i..].iter().map(|(_, c)| c).collect();
        match OPS.iter().find(|op| rest.starts_with(**op)) {
            Some(op) => {
                i += op.chars().count();
                push(out, Tok::Op(op), from, i);
            }
            None => {
                errors.push(err(format!("unexpected character `{c}`")));
                i += 1;
            }
        }
    }
}

/// `12`, `1d20`, `500ms`, `1s`, `50%`, `40x20`. Leading zeros are allowed
/// in times so time columns line up (D 2.4).
fn number(w: &str) -> Result<Tok, String> {
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let bad = || format!("`{w}` is not a number, dice, time, percentage or size");
    if digits(w) {
        return w.parse().map(Tok::Int).map_err(|_| format!("`{w}` is too large"));
    }
    if let Some(n) = w.strip_suffix('%').filter(|n| digits(n)) {
        return n.parse().map(Tok::Percent).map_err(|_| bad());
    }
    if let Some(n) = w.strip_suffix("ms").filter(|n| digits(n)) {
        return n.parse().map(Tok::Time).map_err(|_| bad());
    }
    if let Some(n) = w.strip_suffix('s').filter(|n| digits(n)) {
        return n.parse::<u64>().map(|s| Tok::Time(s * 1000)).map_err(|_| bad());
    }
    for (sep, make) in [('d', Tok::Dice as fn(u32, u32) -> Tok), ('x', Tok::Size)] {
        if let Some((a, b)) = w.split_once(sep)
            && digits(a)
            && digits(b)
        {
            return match (a.parse(), b.parse()) {
                (Ok(a), Ok(b)) => Ok(make(a, b)),
                _ => Err(bad()),
            };
        }
    }
    Err(bad())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(src: &str) -> RawCode {
        RawCode { file: "t".into(), line: 1, namespace: None, lines: src.lines().map(String::from).collect() }
    }

    fn toks(src: &str) -> Vec<Tok> {
        let (t, e) = lex(&code(src));
        assert!(e.is_empty(), "{e:?}");
        t.into_iter().map(|t| t.tok).collect()
    }

    fn errs(src: &str) -> Vec<String> {
        lex(&code(src)).1.into_iter().map(|d| d.to_string()).collect()
    }

    use Tok::*;

    fn n(s: &str) -> Tok {
        Name(s.into())
    }

    fn w(s: &str) -> Tok {
        Word(s.into())
    }

    #[test]
    fn blocks_and_newlines() {
        assert_eq!(
            toks("fn F():\n    if x:\n        return\n\n    // c\n    y = 1\nai A:\n    return WAIT()\n"),
            [
                w("fn"), n("F"), Op("("), Op(")"), Op(":"), Newline,
                Indent, w("if"), w("x"), Op(":"), Newline,
                Indent, w("return"), Newline,
                Dedent, w("y"), Op("="), Int(1), Newline,
                Dedent, w("ai"), n("A"), Op(":"), Newline,
                Indent, w("return"), n("WAIT"), Op("("), Op(")"), Newline,
                Dedent, Eof
            ]
        );
    }

    #[test]
    fn literals() {
        assert_eq!(toks("x = 1d20 + 0500ms + 1s + 50% + 40x20 + \"a \\\"b\\\"\""), [
            w("x"), Op("="), Dice(1, 20), Op("+"), Time(500), Op("+"), Time(1000), Op("+"), Percent(50), Op("+"), Size(40, 20), Op("+"), Str("a \"b\"".into()), Newline, Eof
        ]);
        assert_eq!(toks("r = -1..3"), [w("r"), Op("="), Op("-"), Int(1), Op(".."), Int(3), Newline, Eof]);
        assert_eq!(toks("t = GLYPH.NKI.1..20"), [w("t"), Op("="), n("GLYPH"), Op("."), n("NKI"), Op("."), Int(1), Op(".."), Int(20), Newline, Eof]);
    }

    #[test]
    fn operators_and_wildcards() {
        assert_eq!(toks("x += a.b[k] != c -> d"), [w("x"), Op("+="), w("a"), Op("."), w("b"), Op("["), w("k"), Op("]"), Op("!="), w("c"), Op("->"), w("d"), Newline, Eof]);
        assert_eq!(toks("say(choose(TIP.*))"), [w("say"), Op("("), w("choose"), Op("("), n("TIP"), Wild, Op(")"), Op(")"), Newline, Eof]);
        assert_eq!(toks("x = ITEM.**.pick(18)"), [w("x"), Op("="), n("ITEM"), WildDeep, Op("."), w("pick"), Op("("), Int(18), Op(")"), Newline, Eof]);
        assert_eq!(toks("x = a * b"), [w("x"), Op("="), w("a"), Op("*"), w("b"), Newline, Eof]);
        assert_eq!(toks("x = .LOOT"), [w("x"), Op("="), Op("."), n("LOOT"), Newline, Eof]);
    }

    #[test]
    fn comments_and_strings() {
        assert_eq!(toks("say(\"a // b\") // c"), [w("say"), Op("("), Str("a // b".into()), Op(")"), Newline, Eof]);
    }

    #[test]
    fn positions() {
        let (t, _) = lex(&RawCode { file: "t".into(), line: 10, namespace: None, lines: vec!["fn F():".into(), "    return x".into()] });
        let x = t.iter().find(|t| t.tok == w("x")).unwrap();
        assert_eq!((x.line, x.col, x.text.as_str()), (11, 12, "x"));
    }

    #[test]
    fn errors() {
        assert!(errs("fn F():\n\treturn\n")[0].contains("tab"));
        assert!(errs("fn F():\n      return\n")[0].contains("four spaces more"));
        assert!(errs("fn F():\n    if x:\n        y\n  z\n")[0].contains("does not match"));
        assert!(errs("x = Mixed")[0].contains("upper case"));
        assert!(errs("x = 3d")[0].contains("not a number"));
        assert!(errs("x = \"open")[0].contains("unterminated"));
        assert!(errs("x = a ~ b")[0].contains("column 7: unexpected character `~`"));
    }
}
