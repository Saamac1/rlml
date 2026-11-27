//! `rlml mod merge` (D 3.6, RLML-kehitystyokalut.md step 8): a mod's
//! `patch` rows go into the rows they change as point fixes, `drop`
//! removes a line and a new row follows the block's last, the rest is
//! exported as an include file, and the merged game's resolved
//! definitions are the game's with the mod.

use std::path::Path;

use rlml_core::devmod;
use rlml_core::load::{Layers, MemFiles};
use rlml_core::model::Model;

fn read_tree(root: &Path, dir: &Path, out: &mut std::collections::BTreeMap<String, String>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if e.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        if p.is_dir() {
            read_tree(root, &p, out);
        } else {
            out.insert(
                p.strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
                std::fs::read_to_string(&p).unwrap(),
            );
        }
    }
}

/// The minigame with a comment on the rat's HP, and a mod `vaikea`
/// with `rows` in its `rotat.rlml`.
fn files(rows: &str) -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    read_tree(
        &repo.join("esimerkit/mini"),
        &repo.join("esimerkit/mini"),
        &mut f.game,
    );
    let game = f.game["game.rlml"].replace("- HP now 3\n", "- HP now 3 **heikko**\n");
    f.game.insert("game.rlml".into(), game);
    f.game.insert(
        "dev/vaikea/mod.rlml".into(),
        "mod VAIKEA\n- about \"Rotat ovat sitkeämpiä.\"\n".into(),
    );
    f.game.insert("dev/vaikea/rotat.rlml".into(), rows.into());
    f
}

fn model(f: &MemFiles, mods: &[&str]) -> Model {
    let layers = Layers {
        mods: mods.iter().map(|m| m.to_string()).collect(),
        overrides: false,
    };
    let (m, r) = rlml_core::check_game_with(f, "game.rlml", &layers);
    let errors: Vec<String> = r.errors().map(|d| d.to_string()).collect();
    assert!(errors.is_empty(), "{errors:#?}");
    m
}

/// The merge written into `f`, as the command line writes it.
fn apply(f: &mut MemFiles, m: &devmod::Merge) {
    for (p, _, t) in &m.changed {
        f.game.insert(p.clone(), t.clone());
    }
    let (from, to) = &m.archive;
    let moved: Vec<String> = f
        .game
        .keys()
        .filter(|p| p.starts_with(&format!("{from}/")))
        .cloned()
        .collect();
    for p in moved {
        let t = f.game.remove(&p).unwrap();
        f.game.insert(format!("{to}{}", &p[from.len()..]), t);
    }
}

fn same(f: &MemFiles, mut g: MemFiles, m: &devmod::Merge) -> Model {
    let with = model(f, &["vaikea"]);
    apply(&mut g, m);
    let after = model(&g, &[]);
    assert!(devmod::differences(&devmod::resolved(&with), &devmod::resolved(&after)).is_empty());
    after
}

#[test]
fn patch_rows_are_point_fixes() {
    let f = files(
        "patch entity RAT\n- HP now 9\n- drop hearing\n- new light radius 2\n- sight radius 8\n\npatch keys GAME\n- MOVE.NORTH w\n",
    );
    let m = devmod::merge(&f, "game.rlml", "vaikea").unwrap();
    assert!(m.residual.is_empty(), "{:?}", m.residual);
    let paths: Vec<&str> = m.changed.iter().map(|(p, _, _)| p.as_str()).collect();
    assert_eq!(paths, ["game.rlml", "ui/main.rlml"], "nothing exported");
    let game = &m.changed[0].2;
    assert!(
        game.contains("entity RAT\n- HP now 9 **heikko**\n- MELEE 1d2\n"),
        "only the value changed, and the comment stays:\n{game}"
    );
    assert!(!game.contains("- hearing\n"), "the dropped row");
    assert!(game.contains("- sight radius 8\n"));
    assert!(
        game.contains("- render GLYPH.RAT\n- light radius 2\n"),
        "the new row after the last:\n{game}"
    );
    assert!(m.changed[1].2.contains("- MOVE.NORTH w\n"));
    let d = devmod::unified("game.rlml", &m.changed[0].1, game, 1);
    assert!(
        d.contains("-- HP now 3 **heikko**\n+- HP now 9 **heikko**\n"),
        "{d}"
    );
    let after = same(&f, f.clone(), &m);
    assert_eq!(after.mods.len(), 0);
}

#[test]
fn what_cannot_merge_is_exported() {
    // a row of the standard library's, and a definition of the mod's own
    let f = files(
        "patch entity RAT\n- HP now 9\n\npatch entity STD.DEFAULT.PLAYER\n- sight radius 3\n\nentity BIG_RAT extends RAT\n- HP now 20\n",
    );
    let m = devmod::merge(&f, "game.rlml", "vaikea").unwrap();
    assert!(
        m.residual
            .iter()
            .any(|r| r.contains("sight radius 3") && r.contains("<engine>")),
        "{:?}",
        m.residual
    );
    assert!(
        m.residual
            .iter()
            .any(|r| r.contains("`entity BIG_RAT` is not a patch")),
        "{:?}",
        m.residual
    );
    let rest = m
        .changed
        .iter()
        .find(|(p, _, _)| p == "mods/vaikea/rotat.rlml")
        .map(|(_, _, t)| t.as_str())
        .unwrap();
    assert_eq!(
        rest,
        "patch entity STD.DEFAULT.PLAYER\n- sight radius 3\n\nentity BIG_RAT extends RAT\n- HP now 20\n",
        "the merged rows are out of it"
    );
    let main = &m
        .changed
        .iter()
        .find(|(p, _, _)| p == "game.rlml")
        .unwrap()
        .2;
    assert!(main.contains("- HP now 9 **heikko**\n"));
    assert!(main.ends_with("include mods/vaikea/**\n"), "{main}");
    same(&f, f.clone(), &m);
}

#[test]
fn a_merge_that_changes_the_game_is_refused() {
    // the rat's HP from a later patch of the game's own: the merged row
    // would be overwritten by it, unlike the mod's that loads last
    let mut f = files("patch entity RAT\n- HP now 9\n");
    let main = f.game["game.rlml"].clone();
    f.game.insert(
        "game.rlml".into(),
        format!("{main}\ninclude myohemmin.rlml\n"),
    );
    f.game.insert(
        "myohemmin.rlml".into(),
        "patch entity RAT\n- MELEE 1d3\n".into(),
    );
    // a later patch on another row is fine
    let m = devmod::merge(&f, "game.rlml", "vaikea").unwrap();
    same(&f, f.clone(), &m);
    // a row the game patches already is changed where it is written now
    f.game.insert(
        "myohemmin.rlml".into(),
        "patch entity RAT\n- HP now 4\n".into(),
    );
    let m = devmod::merge(&f, "game.rlml", "vaikea").unwrap();
    let later = &m
        .changed
        .iter()
        .find(|(p, _, _)| p == "myohemmin.rlml")
        .unwrap()
        .2;
    assert_eq!(later, "patch entity RAT\n- HP now 9\n");
    same(&f, f.clone(), &m);
    // a new row would go after the block's last own row, but the game's
    // later patch has added one after it: the order would change
    f.game.insert(
        "myohemmin.rlml".into(),
        "patch entity RAT\n- new light radius 1\n".into(),
    );
    f.game.insert(
        "dev/vaikea/rotat.rlml".into(),
        "patch entity RAT\n- drop hearing\n- new hearing\n".into(),
    );
    let e = devmod::merge(&f, "game.rlml", "vaikea").unwrap_err();
    assert!(
        e[0].contains("would change the game's definitions"),
        "{e:?}"
    );
    assert!(e.iter().any(|l| l.contains("RAT")), "{e:?}");
    let e = devmod::merge(&f, "game.rlml", "ei_ole").unwrap_err();
    assert!(e[0].contains("no mod"), "{e:?}");
}
