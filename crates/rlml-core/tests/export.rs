//! `rlml mod export` (D 3.6, RLML-kehitystyokalut.md step 7): the mod's
//! files go under `mods/NAME/` with an include line, the game as exported
//! has the same resolved definitions as the game with the mod, and an
//! export that would change them is refused.

use std::path::Path;

use rlml_core::devmod;
use rlml_core::load::{Layers, MemFiles};
use rlml_core::model::Model;
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::world::World;

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

/// The minigame with a mod `vaikea` that makes rats tougher.
fn files() -> MemFiles {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    read_tree(
        &repo.join("esimerkit/mini"),
        &repo.join("esimerkit/mini"),
        &mut f.game,
    );
    f.game.insert(
        "dev/vaikea/mod.rlml".into(),
        "mod VAIKEA\n- about \"Rotat ovat sitkeämpiä.\"\n".into(),
    );
    f.game.insert(
        "dev/vaikea/rotat.rlml".into(),
        "> Sitkeämmät rotat\npatch entity RAT\n- HP now 9\n".into(),
    );
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

/// The rats' `HP.now` in a new game.
fn rats(m: &Model) -> Vec<i64> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    let s = &r.w.s;
    (0..s.entities.len())
        .filter(|&e| s.get(e).kind == "RAT")
        .map(|e| {
            s.get(e)
                .get("STD.HP")
                .and_then(|c| c.get("now"))
                .and_then(|v| v.as_number())
                .unwrap()
        })
        .collect()
}

/// The export written into `f`, as the command line writes it on disk.
fn apply(f: &mut MemFiles, e: &devmod::Export) {
    for (p, t) in &e.files {
        f.game.insert(p.clone(), t.clone());
    }
    f.game.insert(e.main.0.clone(), e.main.1.clone());
    let (from, to) = &e.archive;
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

#[test]
fn an_exported_mod_is_the_games_own() {
    let mut f = files();
    let with = model(&f, &["vaikea"]);
    let n = rats(&with).len();
    assert_eq!(rats(&with), vec![9; n]);
    let e = devmod::export(&f, "game.rlml", "vaikea").unwrap();
    let paths: Vec<&str> = e.files.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(paths, ["mods/vaikea/rotat.rlml"], "the header stays");
    assert!(
        e.files[0].1.starts_with("> Sitkeämmät rotat\n"),
        "as it was"
    );
    assert!(
        e.main
            .1
            .ends_with("\n> Modi vaikea: Rotat ovat sitkeämpiä.\ninclude mods/vaikea/**\n"),
        "{}",
        e.main.1
    );
    assert_eq!(
        e.archive,
        ("dev/vaikea".into(), "dev/.arkisto/vaikea".into())
    );
    apply(&mut f, &e);
    // the game without mods is the game with it
    let after = model(&f, &[]);
    assert_eq!(rats(&after), vec![9; n]);
    assert!(devmod::differences(&devmod::resolved(&with), &devmod::resolved(&after)).is_empty());
    assert_ne!(after.base_hash, with.base_hash, "a new version");
    // the archive is out of the way, and the mod cannot go twice
    let gone = rlml_core::check_game_with(
        &f,
        "game.rlml",
        &Layers {
            mods: vec!["vaikea".into()],
            overrides: false,
        },
    )
    .1;
    assert!(
        gone.errors()
            .any(|d| d.to_string().contains("no mod `vaikea`"))
    );
    f.game.insert(
        "dev/vaikea/rotat.rlml".into(),
        "patch entity RAT\n- HP now 9\n".into(),
    );
    let again = devmod::export(&f, "game.rlml", "vaikea").unwrap_err();
    assert!(
        again[0].contains("mods/vaikea/ is there already"),
        "{again:?}"
    );
}

#[test]
fn an_export_that_changes_the_game_is_refused() {
    // the game takes everything under mods/ early, and a later file of its
    // own changes the rats: the exported patch would load before it
    let mut f = files();
    let main = f.game["game.rlml"].clone();
    f.game.insert(
        "game.rlml".into(),
        format!("{main}\ninclude mods/**\ninclude myohemmin.rlml\n"),
    );
    f.game
        .insert("mods/vanha/tyhja.rlml".into(), "> ei mitään\n".into());
    f.game.insert(
        "myohemmin.rlml".into(),
        "patch entity RAT\n- HP now 5\n".into(),
    );
    let with = model(&f, &["vaikea"]);
    assert!(rats(&with).iter().all(|&hp| hp == 9));
    let e = devmod::export(&f, "game.rlml", "vaikea").unwrap_err();
    assert!(
        e[0].contains("would change the game's definitions"),
        "{e:?}"
    );
    assert!(e.iter().any(|l| l.contains("RAT")), "{e:?}");
}

#[test]
fn exports_that_cannot_be() {
    let f = files();
    let e = devmod::export(&f, "game.rlml", "ei_ole").unwrap_err();
    assert!(e[0].contains("no mod `ei_ole`"), "{e:?}");
    // a header with other blocks in its file
    let mut g = files();
    g.game.insert(
        "dev/vaikea/mod.rlml".into(),
        "mod VAIKEA\n- about \"x\"\n\npatch entity RAT\n- HP max 9\n".into(),
    );
    let e = devmod::export(&g, "game.rlml", "vaikea").unwrap_err();
    assert!(e[0].contains("shares its file"), "{e:?}");
    // a mod with errors does not go
    let mut h = files();
    h.game.insert(
        "dev/vaikea/rotat.rlml".into(),
        "patch entity EI_OLE\n- HP now 9\n".into(),
    );
    assert!(devmod::export(&h, "game.rlml", "vaikea").is_err());
}
