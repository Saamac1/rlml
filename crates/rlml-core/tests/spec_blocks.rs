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
                    block = Some(RawCode {
                        file: name.clone(),
                        line: i + 2,
                        namespace: None,
                        lines: vec![],
                    });
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
    (
        "roguescript/02-yleinen-syntaksi.md",
        148,
        "a loop and statements",
    ),
    ("roguescript/02-yleinen-syntaksi.md", 182, "two statements"),
    (
        "roguescript/05-funktiot-ja-lausekkeet.md",
        47,
        "a `match` statement",
    ),
    ("roguescript/05-funktiot-ja-lausekkeet.md", 61, "a loop"),
    (
        "roguescript/05-funktiot-ja-lausekkeet.md",
        89,
        "two expressions",
    ),
    (
        "roguescript/05-funktiot-ja-lausekkeet.md",
        151,
        "a loop before a component",
    ),
    (
        "roguescript/09-kohtaukset-ja-aikajanat.md",
        90,
        "timeline rows",
    ),
];

/// A block that starts with `namespace` is read as a `.rogue` file, the
/// only place the grammar has that line (RS 12.1).
fn as_rogue_file(block: &RawCode) -> Option<RawCode> {
    let first = block.lines.iter().find(|l| !l.trim().is_empty())?;
    first.starts_with("namespace ").then(|| {
        let parsed = rlml_core::syntax::parse_rogue(&block.file, &block.lines.join("\n")).unwrap();
        let rlml_core::syntax::Entry::Code(mut code) = parsed.entries.into_iter().next().unwrap()
        else {
            unreachable!()
        };
        code.line = block.line;
        code
    })
}

#[test]
fn every_block_parses() {
    let mut errors = Vec::new();
    let mut snippets_seen = Vec::new();
    for block in corpus() {
        if let Some((_, _, _)) = SNIPPETS
            .iter()
            .find(|(f, l, _)| *f == block.file && *l == block.line)
        {
            snippets_seen.push((block.file.clone(), block.line));
            errors.extend(
                rlml_core::rs::parse::parse_snippet(&block)
                    .into_iter()
                    .map(|d| d.to_string()),
            );
            continue;
        }
        let code = as_rogue_file(&block).unwrap_or(block);
        errors.extend(
            rlml_core::rs::parse::parse(&code)
                .1
                .into_iter()
                .map(|d| d.to_string()),
        );
    }
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(
        snippets_seen.len(),
        SNIPPETS.len(),
        "a snippet moved: {snippets_seen:?}"
    );
}

#[test]
fn every_type_reads() {
    use rlml_core::rs::ast::{ItemKind, Param, ReturnType};
    let mut errors = Vec::new();
    let mut count = 0;
    let mut read = |p: &[Param], file: &str, errors: &mut Vec<String>| {
        for p in p {
            count += 1;
            if let Err(e) =
                rlml_core::rs::types::parse(&p.ty.to_string(), &|n| rlml_core::names::is_name(n))
            {
                errors.push(format!("{file}:{}: {e}", p.pos.line));
            }
        }
    };
    for block in corpus().into_iter().filter(|b| {
        !SNIPPETS
            .iter()
            .any(|(f, l, _)| *f == b.file && *l == b.line)
    }) {
        let code = as_rogue_file(&block).unwrap_or(block);
        for item in rlml_core::rs::parse::parse(&code).0 {
            match &item.kind {
                ItemKind::Component { fields: p, .. }
                | ItemKind::Event { params: p }
                | ItemKind::Command { params: p, .. }
                | ItemKind::Generator { params: p, .. }
                | ItemKind::Scene { params: p, .. }
                | ItemKind::Timeline { params: p, .. } => read(p, &code.file, &mut errors),
                ItemKind::Function { params: p, ret, .. } => {
                    read(p, &code.file, &mut errors);
                    if let Some(ReturnType::Type(t)) = ret
                        && let Err(e) = rlml_core::rs::types::parse(&t.to_string(), &|n| {
                            rlml_core::names::is_name(n)
                        })
                    {
                        errors.push(format!("{}:{}: {e}", code.file, item.pos.line));
                    }
                }
                _ => {}
            }
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
    assert!(count > 150, "{count} parameters");
}

/// Every function M 9.1 lists has a signature (M 9.3).
#[test]
fn every_engine_function_has_a_signature() {
    let src =
        std::fs::read_to_string(repo().join("moottori/09-moottorin-funktiot-ja-tapahtumat.md"))
            .unwrap();
    let groups = [
        "tila ja ilmentymät",
        "ruutujoukot",
        "listat",
        "satunnaisuus",
        "pakat ja taulut",
        "kyselyt",
        "generointi",
    ];
    let mut missing = Vec::new();
    let mut seen = 0;
    for line in src.lines() {
        let Some(group) = groups
            .iter()
            .find(|g| line.starts_with(&format!("| {g} |")))
        else {
            continue;
        };
        let cells = line.split('|').nth(2).unwrap();
        for name in cells.split('`').skip(1).step_by(2) {
            seen += 1;
            if rlml_core::rs::sigs::versions(name).is_empty() {
                missing.push(format!("{group}: {name}"));
            }
        }
    }
    assert!(seen > 50, "{seen} names read from M 9.1");
    assert!(missing.is_empty(), "{missing:#?}");
    // and every signature is in the vocabulary, which guides load checks (M 9.1)
    let vocab = &src[src.find("## 9.1").unwrap()..src.find("## 9.2").unwrap()];
    let unlisted: Vec<&str> = rlml_core::rs::sigs::all()
        .iter()
        .map(|s| s.name.as_str())
        .filter(|n| !vocab.contains(&format!("`{n}`")))
        .collect();
    assert!(unlisted.is_empty(), "not in M 9.1: {unlisted:?}");
}

/// The examples of each specification document, checked with the standard
/// library: undefined names are unknown, everything else must hold. STD's
/// own chapters are left out, since lib/std/ is assembled from them.
#[test]
fn spec_examples_type_check() {
    use rlml_core::error::Report;
    use rlml_core::load::MemFiles;
    let root = repo();
    let mut engine = std::collections::BTreeMap::new();
    let mut lib = Vec::new();
    files(&root.join("lib"), ".rlml", &mut lib);
    for p in lib {
        engine.insert(
            p.strip_prefix(&root).unwrap().to_string_lossy().to_string(),
            std::fs::read_to_string(&p).unwrap(),
        );
    }
    let mut by_doc: std::collections::BTreeMap<String, Vec<RawCode>> = Default::default();
    for block in corpus()
        .into_iter()
        .filter(|b| !b.file.starts_with("vakiokirjasto/") && !b.file.starts_with("lib/"))
    {
        if SNIPPETS
            .iter()
            .any(|(f, l, _)| *f == block.file && *l == block.line)
        {
            continue;
        }
        let code = as_rogue_file(&block).unwrap_or(block);
        by_doc.entry(code.file.clone()).or_default().push(code);
    }
    let mut errors = Vec::new();
    let mut checked = 0;
    for (doc, blocks) in by_doc {
        // one file per document; each block keeps its own line numbers
        let mut text = String::from("include lib/std.rlml\n\n");
        let mut lines = Vec::new();
        for b in &blocks {
            text.push_str("```roguescript\n");
            lines.push((text.lines().count() + 1, b.line));
            for l in &b.lines {
                text.push_str(l);
                text.push('\n');
            }
            text.push_str("```\n\n");
        }
        let files = MemFiles {
            game: [("doc.rlml".to_string(), text)].into(),
            engine: engine.clone(),
        };
        let (loaded, mut report) = rlml_core::load::load(&files, "doc.rlml");
        let m = rlml_core::model::build(loaded, &mut report);
        let mut r = Report::default();
        rlml_core::rs::check::check_examples(&m, &mut r);
        checked += m.code.iter().filter(|d| d.file == "doc.rlml").count();
        for d in report.errors().chain(r.errors()) {
            // map the line back to the document
            let line = lines
                .iter()
                .rev()
                .find(|(start, _)| d.line >= *start)
                .map(|(start, orig)| orig + (d.line - start))
                .unwrap_or(d.line);
            errors.push(format!("{doc}:{line}: {}", d.message));
        }
    }
    assert!(
        errors.is_empty(),
        "{} errors:\n{}",
        errors.len(),
        errors.join("\n")
    );
    assert!(checked > 70, "{checked} example declarations checked");
}

/// Lenient checking still finds type errors.
#[test]
fn example_checking_finds_type_errors() {
    use rlml_core::error::Report;
    use rlml_core::load::MemFiles;
    let root = repo();
    let mut lib = Vec::new();
    files(&root.join("lib"), ".rlml", &mut lib);
    let engine = lib
        .iter()
        .map(|p| {
            (
                p.strip_prefix(&root).unwrap().to_string_lossy().to_string(),
                std::fs::read_to_string(p).unwrap(),
            )
        })
        .collect();
    let src = "include lib/std.rlml\n\n```roguescript\nfn PROBE():\n    HURT(player, \"5\")\n    UNDEFINED_THING(1)\n```\n";
    let files = MemFiles {
        game: [("doc.rlml".to_string(), src.to_string())].into(),
        engine,
    };
    let (loaded, mut report) = rlml_core::load::load(&files, "doc.rlml");
    let m = rlml_core::model::build(loaded, &mut report);
    let mut r = Report::default();
    rlml_core::rs::check::check_examples(&m, &mut r);
    let msgs: Vec<String> = r.errors().map(|d| d.message.clone()).collect();
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(msgs[0].contains("`amount` takes a `number`"), "{msgs:?}");
}
