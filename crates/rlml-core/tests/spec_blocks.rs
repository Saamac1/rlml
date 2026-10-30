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

