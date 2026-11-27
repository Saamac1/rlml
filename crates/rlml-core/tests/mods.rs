//! The player's `overrides/` (D 3.2) and development mods (D 3.6,
//! RLML-kehitystyokalut.md step 5): places that change rows, mods in
//! activation order with their headers, conflicts, the origin of a
//! patched row, saves and logs that know their mods, and `update existing`.

use std::path::Path;

use rlml_core::load::{Layers, MemFiles};
use rlml_core::model::Model;
use rlml_core::rt::input::{InputLog, Replay};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::save::Save;
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

/// The minigame with its files, a mod `vaikea` that makes rats tougher
/// and updates the ones there are, and a mod `hurja` that changes the
/// same row.
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
        "mod VAIKEA\n- about \"Rotat ovat sitkeämpiä.\"\n- update existing\n".into(),
    );
    f.game.insert(
        "dev/vaikea/rotat.rlml".into(),
        "patch entity RAT\n- HP now 9\n".into(),
    );
    f.game.insert(
        "dev/hurja/mod.rlml".into(),
        "mod HURJA\n- about \"Vielä sitkeämpiä.\"\n".into(),
    );
    f.game.insert(
        "dev/hurja/rotat.rlml".into(),
        "patch entity RAT\n- HP now 20\n".into(),
    );
    f
}

fn model_with(f: &MemFiles, mods: &[&str], overrides: bool) -> (Model, Vec<String>, Vec<String>) {
    let layers = Layers {
        mods: mods.iter().map(|m| m.to_string()).collect(),
        overrides,
    };
    let (m, r) = rlml_core::check_game_with(f, "game.rlml", &layers);
    let errors = r.errors().map(|d| d.to_string()).collect();
    let warnings = r.warnings().map(|d| d.to_string()).collect();
    (m, errors, warnings)
}

fn model(f: &MemFiles, mods: &[&str]) -> Model {
    let (m, errors, _) = model_with(f, mods, false);
    assert!(errors.is_empty(), "{errors:#?}");
    m
}

fn start(m: &Model) -> Runner<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    r
}

/// The rats' `HP.now`, in creation order.
fn rats(r: &Runner) -> Vec<i64> {
    let s = &r.w.s;
    (0..s.entities.len())
        .filter(|&e| s.get(e).alive && s.get(e).kind == "RAT")
        .map(|e| {
            s.get(e)
                .get("STD.HP")
                .and_then(|c| c.get("now"))
                .and_then(|v| v.as_number())
                .unwrap()
        })
        .collect()
}

fn set_rat_hp(r: &mut Runner, i: usize, hp: i64) {
    let s = &mut r.w.s;
    let e = (0..s.entities.len())
        .filter(|&e| s.get(e).alive && s.get(e).kind == "RAT")
        .nth(i)
        .unwrap();
    let hp_comp = s
        .get_mut(e)
        .components
        .iter_mut()
        .find(|c| c.name == "STD.HP")
        .unwrap();
    hp_comp.set("now", rlml_core::rt::value::Value::Number(hp));
}

#[test]
fn overrides_are_places_that_need_new_for_new_rows() {
    let mut f = files();
    f.user.insert(
        "overrides/keys.rlml".into(),
        "keys GAME\n- MOVE.NORTH w\n- new look o\n\nentity RAT\n- HP now 1\n".into(),
    );
    // `play` reads overrides/, the tools do not (D 3.2)
    let (plain, errors, _) = model_with(&f, &[], false);
    assert!(errors.is_empty(), "{errors:#?}");
    let (over, errors, _) = model_with(&f, &[], true);
    assert!(errors.is_empty(), "{errors:#?}");
    let key = |m: &Model, action: &str| {
        m.contexts["GAME"]
            .iter()
            .flat_map(|b| b.items.iter())
            .find(|i| i.key() == action)
            .map(|i| i.rest(1).to_string())
    };
    assert_eq!(key(&plain, "MOVE.NORTH").as_deref(), Some("k up"));
    assert_eq!(
        key(&over, "MOVE.NORTH").as_deref(),
        Some("w"),
        "the row changed"
    );
    assert_eq!(
        key(&over, "MOVE.SOUTH").as_deref(),
        Some("j down"),
        "the others stayed"
    );
    assert_eq!(
        key(&over, "look").as_deref(),
        Some("o"),
        "`new` added a row"
    );
    let n = rats(&start(&plain)).len();
    assert_eq!(rats(&start(&over)), vec![1; n], "the entity's place");
    // an unknown row without `new` is a typo, and a profile is not the player's
    f.user.insert(
        "overrides/bad.rlml".into(),
        "keys GAME\n- LOOKK o\n\nprofile MINI\n- WINS\n".into(),
    );
    let (_, errors, _) = model_with(&f, &[], true);
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`LOOKK`") && e.contains("new LOOKK")),
        "{errors:#?}"
    );
    assert!(
        errors
            .iter()
            .any(|e| e.contains("`profile` block cannot be in `overrides/`")),
        "{errors:#?}"
    );
}

#[test]
fn mods_load_in_order_with_their_headers() {
    let f = files();
    let base = model(&f, &[]);
    let vaikea = model(&f, &["vaikea"]);
    let n = rats(&start(&base)).len();
    assert!(n > 0);
    assert_eq!(rats(&start(&base)), vec![3; n]);
    assert_eq!(rats(&start(&vaikea)), vec![9; n], "the mod's patch");
    assert_eq!(vaikea.mods.len(), 1);
    assert_eq!(vaikea.mod_infos[0].about, "Rotat ovat sitkeämpiä.");
    assert_ne!(vaikea.source_hash, base.source_hash);
    assert_eq!(vaikea.base_hash, base.source_hash, "the game's own files");
    // two mods on the same row: the later wins, with a warning
    let (both, errors, warnings) = model_with(&f, &["vaikea", "hurja"], false);
    assert!(errors.is_empty(), "{errors:#?}");
    assert_eq!(rats(&start(&both)), vec![20; n]);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("mods `vaikea` and `hurja` both change `HP` of `RAT`")),
        "{warnings:#?}"
    );
    // the inspector tells where the row came from
    let r = start(&both);
    let rat = (0..r.w.s.entities.len())
        .find(|&e| r.w.s.get(e).kind == "RAT")
        .unwrap();
    let lines = r
        .inspect(&format!("entity#{rat}.patched"))
        .unwrap()
        .lines("");
    assert_eq!(lines, ["HP: dev/hurja/rotat.rlml:2"]);
    assert!(
        r.inspect("")
            .unwrap()
            .lines("")
            .contains(&"mods: vaikea, hurja".to_string())
    );
    // a header's `base` from another version, a mod without a header, a
    // mod that is not there
    let mut g = files();
    g.game.insert(
        "dev/vaikea/mod.rlml".into(),
        "mod VAIKEA\n- about \"x\"\n- base 0123456789abcdef\n".into(),
    );
    g.game.remove("dev/hurja/mod.rlml");
    let (_, errors, warnings) = model_with(&g, &["vaikea", "hurja", "ei_ole"], false);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("made on another version")),
        "{warnings:#?}"
    );
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("`hurja` has no `mod` header")),
        "{warnings:#?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("no mod `ei_ole`")),
        "{errors:#?}"
    );
}

#[test]
fn saves_and_logs_know_their_mods() {
    let f = files();
    let base = model(&f, &[]);
    let vaikea = model(&f, &["vaikea"]);
    let r = start(&vaikea);
    let save = Save::read(&r.save().write()).unwrap();
    assert_eq!(save.mods.len(), 1);
    let l = Runner::load(&base, &save).unwrap();
    assert!(
        l.w.warnings
            .iter()
            .any(|w| w.contains("played with the mods vaikea and is loaded with none")),
        "{:?}",
        l.w.warnings
    );
    let text = r.input_log().write().unwrap();
    assert!(text.contains("\nmods vaikea\n"), "{text}");
    let log = InputLog::read(&text).unwrap();
    assert!(Replay::run(&vaikea, &log).is_ok());
    let e = Replay::run(&base, &log).err().expect("other mods");
    assert!(e.contains("played with the mods [vaikea]"), "{e}");
}

#[test]
fn update_existing_changes_the_fields_the_game_has_not() {
    let f = files();
    let base = model(&f, &[]);
    let vaikea = model(&f, &["vaikea"]);
    let hurja = model(&f, &["hurja"]);
    let mut r = start(&base);
    let n = rats(&r).len();
    assert!(n >= 2, "two rats at least");
    set_rat_hp(&mut r, 0, 2);
    // a mod without `update existing` leaves the rats there are alone
    let mut plain = r.clone();
    plain.reload(&hurja).unwrap();
    assert_eq!(rats(&plain)[1], 3);
    // with it, the unhurt rats take the new HP and the hurt one keeps its
    let notes = r.reload(&vaikea).unwrap();
    assert_eq!(rats(&r)[0], 2, "the game changed this one");
    assert!(rats(&r)[1..].iter().all(|&hp| hp == 9), "{:?}", rats(&r));
    assert!(
        notes.iter().any(|n| n.contains("`update existing`")),
        "{notes:?}"
    );
}

#[test]
fn the_inspectors_mod_row_gives_a_new_game_the_value() {
    let mut f = files();
    let base = model(&f, &[]);
    let mut r = start(&base);
    let rat = (0..r.w.s.entities.len())
        .find(|&e| r.w.s.get(e).kind == "RAT")
        .unwrap();
    let path = format!("entity#{rat}.HP.now");
    let row = r.inspect_patch(&path, "7").unwrap();
    r.inspect_set(&path, "7").unwrap();
    f.game.insert(
        "dev/omat/mod.rlml".into(),
        "mod OMAT\n- about \"Inspectorin muutokset.\"\n".into(),
    );
    f.game.insert("dev/omat/inspector.rlml".into(), row);
    let omat = model(&f, &["omat"]);
    let n = rats(&start(&omat)).len();
    assert_eq!(rats(&start(&omat)), vec![7; n], "every new rat");
    // an instance's own state is no definition's
    assert!(
        r.inspect_patch(&format!("wait#{rat}.locals.x"), "1")
            .is_err()
    );
}
