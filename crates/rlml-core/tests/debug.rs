//! Breakpoints and stepping (M 5.7, RLML-kehitystyokalut.md step 9): a
//! breakpoint on a source line stops before its statement with the body's
//! names, a line without one moves to the next, steps go into, over and
//! out of calls, and a hook in another thread answers through a channel.

use std::path::Path;
use std::sync::{Arc, Mutex};

use rlml_core::load::MemFiles;
use rlml_core::model::Model;
use rlml_core::rt::debug::{self, Debugger, Step, Stop};
use rlml_core::rt::run::{Runner, Status};
use rlml_core::rt::value::CommandValue;
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

const GAME: &str = r#"game T
- start HALL
- TALLY

include lib/std.rlml
include logic.rogue

settings
- style ascii
- language fi

component TALLY
- count number

map HALL

```
#####
#S..#
#####
```

legend:
- # WALL
- . FLOOR
- S FLOOR as START
"#;

/// Line numbers matter here: the breakpoints name them.
const LOGIC: &str = "fn TWICE(n: number) -> number:
    m = n * 2

    return m

command COUNT() cost 0:
    a = 1
    b = TWICE(a)
    game.TALLY.count += b
    return true
";

fn model() -> Model {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut f = MemFiles::default();
    read_tree(&repo, &repo.join("lib"), &mut f.engine);
    f.game.insert("game.rlml".into(), GAME.into());
    f.game.insert("logic.rogue".into(), LOGIC.into());
    let (m, r) = rlml_core::check_game(&f, "game.rlml");
    assert!(
        !r.has_errors(),
        "{:#?}",
        r.errors().map(|d| d.to_string()).collect::<Vec<_>>()
    );
    m
}

/// A debugger whose hook keeps each stop and answers from `steps`, then
/// `Continue`.
fn recorder(steps: Vec<Step>) -> (Debugger, Arc<Mutex<Vec<Stop>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let kept = seen.clone();
    let mut steps = steps.into_iter();
    let d = Debugger::new(Box::new(move |w, stop| {
        // the world can be read in the stop
        assert!(w.s.player.is_some());
        kept.lock().unwrap().push(stop.clone());
        steps.next().unwrap_or(Step::Continue)
    }));
    (d, seen)
}

fn start(m: &Model) -> Runner<'_> {
    let mut w = World::new_game(m, 7).unwrap();
    w.begin().unwrap();
    let mut r = Runner::new(w);
    assert_eq!(r.run(0).unwrap(), Status::Input);
    r
}

fn count(r: &mut Runner) {
    let n = r.awaiting().unwrap();
    r.input(
        n,
        &CommandValue {
            name: "COUNT".into(),
            args: vec![],
        },
    )
    .unwrap();
    r.run(0).unwrap();
}

/// Each stop as `LINE NAME depth`.
fn places(seen: &Arc<Mutex<Vec<Stop>>>) -> Vec<String> {
    seen.lock()
        .unwrap()
        .iter()
        .map(|s| format!("{} {} {}", s.line, s.name, s.depth))
        .collect()
}

#[test]
fn a_breakpoint_stops_with_the_bodys_names() {
    let m = model();
    let (mut d, seen) = recorder(vec![]);
    // line 3 is blank: the breakpoint moves to the `return`
    assert_eq!(
        d.add(&m, "logic.rogue:3").unwrap(),
        ("logic.rogue".into(), 4)
    );
    assert_eq!(d.add(&m, "logic.rogue:2").unwrap().1, 2);
    let mut r = start(&m);
    r.w.debug = Some(d.shared());
    count(&mut r);
    assert_eq!(places(&seen), ["2 TWICE 1", "4 TWICE 1"]);
    let stops = seen.lock().unwrap().clone();
    assert!(
        stops[0].locals.contains(&("n".into(), "1".into())),
        "{:?}",
        stops[0].locals
    );
    assert!(
        stops[1].locals.contains(&("m".into(), "2".into())),
        "{:?}",
        stops[1].locals
    );
    assert_eq!(stops[0].calls, ["COUNT", "TWICE"]);
    assert_eq!(stops[0].breakpoint, Some(("logic.rogue".into(), 2)));
    // the game went on as without the debugger
    let mut plain = start(&m);
    count(&mut plain);
    assert!(r.w.s == plain.w.s);
    for (bad, says) in [
        ("logic.rogue", "FILE:LINE"),
        ("ei.rogue:3", "has no RogueScript"),
        ("logic.rogue:200", "no statement there or after it"),
    ] {
        let e = d_err(&m, bad);
        assert!(e.contains(says), "{bad}: {e}");
    }
}

fn d_err(m: &Model, spec: &str) -> String {
    debug::statement_at(m, spec).unwrap_err()
}

#[test]
fn steps_go_into_over_and_out_of_calls() {
    let m = model();
    // into from `a = 1`: the call's line, then into TWICE
    let (mut d, seen) = recorder(vec![Step::Into, Step::Into, Step::Into, Step::Out]);
    d.add(&m, "logic.rogue:7").unwrap();
    let mut r = start(&m);
    r.w.debug = Some(d.shared());
    count(&mut r);
    assert_eq!(
        places(&seen),
        [
            "7 COUNT 0",
            "8 COUNT 0",
            "2 TWICE 1",
            "4 TWICE 1",
            "9 COUNT 0"
        ]
    );
    // over goes past the call
    let (mut d, seen) = recorder(vec![Step::Over, Step::Over, Step::Over]);
    d.add(&m, "logic.rogue:7").unwrap();
    let mut r = start(&m);
    r.w.debug = Some(d.shared());
    count(&mut r);
    assert_eq!(
        places(&seen),
        ["7 COUNT 0", "8 COUNT 0", "9 COUNT 0", "10 COUNT 0"]
    );
    assert_eq!(
        seen.lock().unwrap()[2]
            .locals
            .iter()
            .find(|(n, _)| n == "b")
            .unwrap()
            .1,
        "2"
    );
}

#[test]
fn a_hook_in_another_thread_answers_through_a_channel() {
    let m: &'static Model = Box::leak(Box::new(model()));
    let (hook, stops, steps) = debug::channel();
    let mut d = Debugger::new(hook);
    d.add(m, "logic.rogue:8").unwrap();
    let game = std::thread::spawn(move || {
        let mut r = start(m);
        r.w.debug = Some(d.shared());
        count(&mut r);
        r.w.s.game.components[0].get("count").cloned()
    });
    let first = stops.recv().unwrap();
    assert_eq!((first.line, first.name.as_str()), (8, "COUNT"));
    steps.send(Step::Into).unwrap();
    let second = stops.recv().unwrap();
    assert_eq!((second.line, second.name.as_str()), (2, "TWICE"));
    steps.send(Step::Continue).unwrap();
    let count = game.join().unwrap();
    assert_eq!(count, Some(rlml_core::rt::value::Value::Number(2)));
}
