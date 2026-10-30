//! Every RogueScript block of the specification and of `lib/std/` as a
//! test corpus (RS 12).

use std::path::{Path, PathBuf};

use rlml_core::rs::lex::lex;
use rlml_core::syntax::RawCode;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            files(&p, ext, out);
        } else if p.to_string_lossy().ends_with(ext) {
            out.push(p);
        }
    }
}

/// The `roguescript` code blocks of the specification documents and the
/// standard library, with the file and first line of each.
fn corpus() -> Vec<RawCode> {
    let root = repo();
    let mut paths = Vec::new();
    for doc in ["dataformaatit", "roguescript", "moottori", "vakiokirjasto"] {
        files(&root.join(doc), ".md", &mut paths);
    }
    files(&root.join("lib"), ".rlml", &mut paths);
    paths.sort();
    let mut out = Vec::new();
    for p in paths {
        let src = std::fs::read_to_string(&p).unwrap();
        let name = p.strip_prefix(&root).unwrap().to_string_lossy().to_string();
        let mut block: Option<RawCode> = None;
        for (i, line) in src.lines().enumerate() {
            match &mut block {
                None if line.trim() == "```roguescript" => {
                    block = Some(RawCode { file: name.clone(), line: i + 2, namespace: None, lines: vec![] });
                }
                Some(_) if line.trim_start().starts_with("```") => out.push(block.take().unwrap()),
                Some(b) => b.lines.push(line.to_string()),
                None => {}
            }
        }
    }
    out
}

#[test]
fn corpus_is_found() {
    let c = corpus();
    assert!(c.len() > 60, "{} blocks", c.len());
    assert!(c.iter().any(|b| b.file.starts_with("lib/std/")));
}

#[test]
fn every_block_lexes() {
    let mut errors = Vec::new();
    for block in corpus() {
        errors.extend(lex(&block).1.into_iter().map(|d| d.to_string()));
    }
    assert!(errors.is_empty(), "{errors:#?}");
}


/// Documentation snippets: statements, expressions or timeline rows
/// without a declaration around them. Parsed with `parse_snippet`.
const SNIPPETS: &[(&str, usize, &str)] = &[
    ("roguescript/02-yleinen-syntaksi.md", 148, "a loop and statements"),
    ("roguescript/02-yleinen-syntaksi.md", 182, "two statements"),
    ("roguescript/05-funktiot-ja-lausekkeet.md", 47, "a `match` statement"),
    ("roguescript/05-funktiot-ja-lausekkeet.md", 61, "a loop"),
    ("roguescript/05-funktiot-ja-lausekkeet.md", 89, "two expressions"),
    ("roguescript/05-funktiot-ja-lausekkeet.md", 151, "a loop before a component"),
    ("roguescript/09-kohtaukset-ja-aikajanat.md", 89, "timeline rows"),
];

/// A block that starts with `namespace` is read as a `.rogue` file, the
/// only place the grammar has that line (RS 12.1).
fn as_rogue_file(block: &RawCode) -> Option<RawCode> {
    let first = block.lines.iter().find(|l| !l.trim().is_empty())?;
    first.starts_with("namespace ").then(|| {
        let parsed = rlml_core::syntax::parse_rogue(&block.file, &block.lines.join("\n")).unwrap();
        let rlml_core::syntax::Entry::Code(mut code) = parsed.entries.into_iter().next().unwrap() else { unreachable!() };
        code.line = block.line;
        code
    })
}

#[test]
fn every_block_parses() {
    let mut errors = Vec::new();
    let mut snippets_seen = Vec::new();
    for block in corpus() {
        if let Some((_, _, _)) = SNIPPETS.iter().find(|(f, l, _)| *f == block.file && *l == block.line) {
            snippets_seen.push((block.file.clone(), block.line));
            errors.extend(rlml_core::rs::parse::parse_snippet(&block).into_iter().map(|d| d.to_string()));
            continue;
        }
        let code = as_rogue_file(&block).unwrap_or(block);
        errors.extend(rlml_core::rs::parse::parse(&code).1.into_iter().map(|d| d.to_string()));
    }
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(snippets_seen.len(), SNIPPETS.len(), "a snippet moved: {snippets_seen:?}");
}

