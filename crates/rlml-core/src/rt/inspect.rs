//! The inspector, reading (RLML-kehitystyokalut.md step 3): the game's
//! state by path, as a tree the terminal, the command line and later the
//! MCP server show. It only reads, so it can look at any moment and the
//! game and its replay go on the same.
//!
//! Paths use the names RogueScript uses: `game.turn`, `game.STORY`,
//! `entity#12.HP.now`, `wait#12.locals.target`. A component is named by
//! its full name or by its last part when that is unique (`HP` for
//! `STD.HP`). The roots are `game`, `maps`, `map.NAME`, `entities`,
//! `entity#N`, `seats`, `wait`, `wait#N` and `scenes`; the empty path
//! lists them.

use super::run::Runner;
use super::state::Place;
use super::value::{ComponentValue, EntityId, Value};
use super::world::RtResult;

/// A part of the state: a value, or named parts.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Leaf(String),
    Branch(Vec<(String, Node)>),
}

impl Node {
    fn leaf(v: impl ToString) -> Node {
        Node::Leaf(v.to_string())
    }

    /// The tree as indented lines under `name`, two spaces a level.
    pub fn lines(&self, name: &str) -> Vec<String> {
        let mut out = Vec::new();
        self.write(name, 0, &mut out);
        out
    }

    fn write(&self, name: &str, depth: usize, out: &mut Vec<String>) {
        let pad = "  ".repeat(depth);
        match self {
            Node::Leaf(v) if name.is_empty() => out.push(format!("{pad}{v}")),
            Node::Leaf(v) => out.push(format!("{pad}{name}: {v}")),
            Node::Branch(parts) => {
                let inner = if name.is_empty() {
                    depth
                } else {
                    out.push(format!("{pad}{name}"));
                    depth + 1
                };
                for (n, p) in parts {
                    p.write(n, inner, out);
                }
            }
        }
    }

    /// The part at `segs`: a branch's named part, segment by segment.
    fn at(self, segs: &[&str], path: &str) -> RtResult<Node> {
        let mut node = self;
        for s in segs {
            node = match node {
                Node::Branch(parts) => parts
                    .into_iter()
                    .find(|(n, _)| n == s)
                    .map(|(_, p)| p)
                    .ok_or_else(|| format!("inspect: no `{s}` in `{path}`"))?,
                Node::Leaf(_) => return Err(format!("inspect: `{path}` goes past a value")),
            };
        }
        Ok(node)
    }
}

fn components(cs: &[ComponentValue]) -> Node {
    Node::Branch(cs.iter().map(|c| (c.name.clone(), fields(c))).collect())
}

fn fields(c: &ComponentValue) -> Node {
    Node::Branch(
        c.fields
            .iter()
            .map(|(n, v)| (n.clone(), Node::Leaf(show(v))))
            .collect(),
    )
}

/// A value in full: lists, collections and dictionaries with their
/// members, where a value's own text only counts them.
fn show(v: &Value) -> String {
    match v {
        Value::List(items) | Value::Collection(items) => {
            format!(
                "[{}]",
                items.iter().map(show).collect::<Vec<_>>().join(", ")
            )
        }
        Value::Dict(pairs) => format!(
            "{{{}}}",
            pairs
                .iter()
                .map(|(k, v)| format!("{}: {}", show(k), show(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        v => v.to_string(),
    }
}

/// The component `segs` names, by full name or by a unique last part,
/// and the segments it took.
fn component<'a>(cs: &'a [ComponentValue], segs: &[&str]) -> Option<(&'a ComponentValue, usize)> {
    for take in (1..=segs.len()).rev() {
        let name = segs[..take].join(".");
        if let Some(c) = cs.iter().find(|c| c.name == name) {
            return Some((c, take));
        }
        let short: Vec<&ComponentValue> = cs
            .iter()
            .filter(|c| c.name.rsplit('.').next() == Some(name.as_str()))
            .collect();
        if let [c] = short.as_slice() {
            return Some((c, take));
        }
    }
    None
}

/// `#N` or `N` as an entity handle.
fn handle(s: &str) -> Option<EntityId> {
    s.trim_start_matches('#').parse().ok()
}

impl Runner<'_> {
    /// The part of the state at `path` (module docs).
    pub fn inspect(&self, path: &str) -> RtResult<Node> {
        let segs: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        let Some((&root, rest)) = segs.split_first() else {
            return Ok(self.inspect_roots());
        };
        let (root, id) = match root.split_once('#') {
            Some((r, n)) => (r, Some(n)),
            None => (root, None),
        };
        match (root, id) {
            ("game", None) => self.inspect_game(rest, path),
            ("maps", None) => self.inspect_maps().at(rest, path),
            ("map", None) => {
                let (name, rest) = rest.split_first().ok_or("inspect: `map.NAME`")?;
                self.inspect_map(name)?.at(rest, path)
            }
            ("entities", None) => self.inspect_entities().at(rest, path),
            ("entity", Some(n)) => {
                let e = handle(n).ok_or_else(|| format!("inspect: `{n}` is no handle"))?;
                self.inspect_entity(e, rest, path)
            }
            ("seats", None) => self.inspect_seats().at(rest, path),
            ("wait", None) => self.inspect_waits().at(rest, path),
            ("wait", Some(n)) => {
                let e = handle(n).ok_or_else(|| format!("inspect: `{n}` is no handle"))?;
                self.inspect_wait(e)?.at(rest, path)
            }
            ("scenes", None) => self.inspect_scenes().at(rest, path),
            _ => Err(format!(
                "inspect: no `{root}`: the roots are game, maps, map.NAME, entities, entity#N, seats, wait, wait#N and scenes"
            )),
        }
    }

    fn inspect_roots(&self) -> Node {
        let s = &self.w.s;
        let map = s.active_map().map(|m| s.maps[m].name.clone());
        Node::Branch(vec![
            (
                "game".into(),
                Node::leaf(format!(
                    "turn {}, time {}, seed {}",
                    s.game.turn(),
                    s.game.time,
                    s.game.seed
                )),
            ),
            (
                "map".into(),
                Node::leaf(map.unwrap_or_else(|| "none".into())),
            ),
            (
                "player".into(),
                Node::leaf(
                    s.player
                        .map(|p| format!("#{p} {}", s.get(p).kind))
                        .unwrap_or_else(|| "none".into()),
                ),
            ),
            (
                "entities".into(),
                Node::leaf(format!(
                    "{} alive",
                    s.entities.iter().filter(|e| e.alive).count()
                )),
            ),
            ("seats".into(), Node::leaf(s.seats.len())),
            (
                "wait".into(),
                Node::leaf(format!("{} waiting", self.waiting.len())),
            ),
            (
                "mods".into(),
                Node::leaf({
                    let names: Vec<&str> = self.w.m.mods.iter().map(|(n, _)| n.as_str()).collect();
                    if names.is_empty() {
                        "none".to_string()
                    } else {
                        names.join(", ")
                    }
                }),
            ),
            (
                "scenes".into(),
                Node::leaf(format!(
                    "{} in the foreground, {} queued, {} in the background",
                    self.scene.is_some() as usize,
                    s.scene_queue.len(),
                    s.backgrounds.len()
                )),
            ),
        ])
    }

    fn inspect_game(&self, rest: &[&str], path: &str) -> RtResult<Node> {
        let g = &self.w.s.game;
        match rest {
            [] => {
                let mut parts = vec![
                    ("turn".into(), Node::leaf(g.turn())),
                    ("time".into(), Node::leaf(g.time)),
                    ("seed".into(), Node::leaf(g.seed)),
                    (
                        "start".into(),
                        Node::leaf(g.start.clone().unwrap_or_else(|| "none".into())),
                    ),
                ];
                if let Node::Branch(cs) = components(&g.components) {
                    parts.extend(cs);
                }
                Ok(Node::Branch(parts))
            }
            ["turn", more @ ..] => Node::leaf(g.turn()).at(more, path),
            ["time", more @ ..] => Node::leaf(g.time).at(more, path),
            ["seed", more @ ..] => Node::leaf(g.seed).at(more, path),
            _ => {
                let (c, took) = component(&g.components, rest)
                    .ok_or_else(|| format!("inspect: no `{}` in game", rest.join(".")))?;
                fields(c).at(&rest[took..], path)
            }
        }
    }

    fn inspect_maps(&self) -> Node {
        let s = &self.w.s;
        Node::Branch(
            s.maps
                .iter()
                .enumerate()
                .map(|(i, m)| {
                    let on = s.entities_on(i).len();
                    let state = if m.frozen_at.is_some() {
                        "frozen"
                    } else {
                        "active"
                    };
                    (
                        m.name.clone(),
                        Node::leaf(format!("{}×{}, {state}, {on} entities", m.width, m.height)),
                    )
                })
                .collect(),
        )
    }

    fn inspect_map(&self, name: &str) -> RtResult<Node> {
        let s = &self.w.s;
        let id = s
            .maps
            .iter()
            .position(|m| m.name == name || m.name.rsplit('.').next() == Some(name))
            .ok_or_else(|| format!("inspect: no map `{name}`"))?;
        let m = &s.maps[id];
        let labels = Node::Branch(
            m.labels
                .iter()
                .map(|(l, cells)| {
                    let cells: Vec<String> =
                        cells.iter().map(|(x, y)| format!("{x},{y}")).collect();
                    (l.clone(), Node::leaf(cells.join(" ")))
                })
                .collect(),
        );
        let vars = Node::Branch(
            m.vars
                .iter()
                .map(|(n, v)| (n.clone(), Node::Leaf(show(v))))
                .collect(),
        );
        let entities = Node::Branch(
            s.entities_on(id)
                .into_iter()
                .map(|e| (format!("#{e}"), Node::leaf(self.entity_line(e))))
                .collect(),
        );
        Ok(Node::Branch(vec![
            (
                "size".into(),
                Node::leaf(format!("{}×{}", m.width, m.height)),
            ),
            ("layers".into(), Node::leaf(m.layers.join(" "))),
            ("clock".into(), Node::leaf(m.clock(s.game.time))),
            ("frozen".into(), Node::leaf(m.frozen_at.is_some())),
            (
                "remembered".into(),
                Node::leaf(format!("{} cells", m.remembered.len())),
            ),
            ("labels".into(), labels),
            ("vars".into(), vars),
            ("entities".into(), entities),
        ]))
    }

    /// One entity on a line: its kind and where it is.
    fn entity_line(&self, e: EntityId) -> String {
        let s = &self.w.s;
        let i = s.get(e);
        let place = match &i.place {
            Place::Nowhere => "nowhere".to_string(),
            Place::Map(p) => format!("{} {},{}", s.maps[p.map].name, p.x, p.y),
            Place::In {
                holder,
                component,
                field,
                slot,
            } => format!("in #{holder} {component}.{field}[{slot}]"),
        };
        let dead = if i.alive { "" } else { ", destroyed" };
        format!("{} at {place}{dead}", i.kind)
    }

    fn inspect_entities(&self) -> Node {
        let s = &self.w.s;
        Node::Branch(
            (0..s.entities.len())
                .filter(|&e| s.get(e).alive)
                .map(|e| (format!("#{e}"), Node::leaf(self.entity_line(e))))
                .collect(),
        )
    }

    fn inspect_entity(&self, e: EntityId, rest: &[&str], path: &str) -> RtResult<Node> {
        let s = &self.w.s;
        if e >= s.entities.len() {
            return Err(format!("inspect: no entity #{e}"));
        }
        let i = s.get(e);
        if rest.is_empty() {
            // where the definition comes from: its file and line
            let origin = self
                .w
                .m
                .defs
                .get(&i.kind)
                .map(|d| format!("{}:{}", d.file, d.line))
                .unwrap_or_else(|| "none".into());
            let mut parts = vec![
                ("is".into(), Node::leaf(self.entity_line(e))),
                ("defined".into(), Node::leaf(origin)),
            ];
            // the rows a patch or a mod changed, and where (D 3.6)
            if let Some(d) = self.w.m.defs.get(&i.kind)
                && !d.patched.is_empty()
            {
                parts.push((
                    "patched".into(),
                    Node::Branch(
                        d.patched
                            .iter()
                            .map(|(k, f, l)| (k.clone(), Node::leaf(format!("{f}:{l}"))))
                            .collect(),
                    ),
                ));
            }
            if let Some(n) = s.seat_of(e) {
                parts.push(("seat".into(), Node::leaf(n)));
            }
            if let Some(w) = s.wake_of(e) {
                parts.push((
                    "wake".into(),
                    Node::leaf(format!("at {}, seq {}", w.at, w.seq)),
                ));
            }
            if let Node::Branch(cs) = components(&i.components) {
                parts.extend(cs);
            }
            return Ok(Node::Branch(parts));
        }
        // the definition's patched rows by their own path (D 3.6)
        if let ["patched", more @ ..] = rest
            && let Node::Branch(parts) = self.inspect_entity(e, &[], path)?
            && let Some((_, node)) = parts.into_iter().find(|(n, _)| n == "patched")
        {
            return node.at(more, path);
        }
        let (c, took) = component(&i.components, rest)
            .ok_or_else(|| format!("inspect: #{e} has no `{}`", rest.join(".")))?;
        fields(c).at(&rest[took..], path)
    }

    fn inspect_seats(&self) -> Node {
        let s = &self.w.s;
        Node::Branch(
            s.seats
                .iter()
                .map(|seat| {
                    let members: Vec<String> =
                        seat.members.iter().map(|m| format!("#{m}")).collect();
                    let active = seat
                        .active
                        .map(|a| format!("#{a}"))
                        .unwrap_or_else(|| "none".into());
                    (
                        format!("{}", seat.n),
                        Node::leaf(format!("active {active}, members {}", members.join(" "))),
                    )
                })
                .collect(),
        )
    }

    fn inspect_waits(&self) -> Node {
        Node::Branch(
            self.waiting
                .iter()
                .map(|(e, st)| {
                    (
                        format!("#{e}"),
                        Node::leaf(format!("{} at wait point {}", st.command, st.point)),
                    )
                })
                .collect(),
        )
    }

    fn inspect_wait(&self, e: EntityId) -> RtResult<Node> {
        let st = self
            .waiting
            .get(&e)
            .ok_or_else(|| format!("inspect: #{e} has no waiting command"))?;
        // every name the body has set so far; the names in sight at the
        // wait point come with the body's name table (step 2)
        let locals = Node::Branch(
            st.env
                .locals
                .iter()
                .map(|(n, v)| (n.clone(), Node::Leaf(show(v))))
                .collect(),
        );
        let wake = self
            .w
            .s
            .wake_of(e)
            .map(|w| format!("at {}", w.at))
            .unwrap_or_else(|| "none: it waits for a scene".into());
        Ok(Node::Branch(vec![
            ("command".into(), Node::leaf(&st.command)),
            ("point".into(), Node::leaf(st.point)),
            ("wake".into(), Node::leaf(wake)),
            ("locals".into(), locals),
        ]))
    }

    fn inspect_scenes(&self) -> Node {
        let s = &self.w.s;
        let foreground = match (&self.scene, self.w.pause.as_ref()) {
            (None, _) => Node::leaf("none"),
            (Some(_), Some(p)) => Node::leaf(format!("waits: {p:?}")),
            (Some(_), None) => Node::leaf("runs on its clock"),
        };
        let queue = Node::Branch(
            s.scene_queue
                .iter()
                .enumerate()
                .map(|(i, q)| (format!("{i}"), Node::leaf(&q.call.name)))
                .collect(),
        );
        let backgrounds = Node::Branch(
            s.backgrounds
                .iter()
                .map(|b| {
                    let clock = match b.base {
                        Some(base) if b.realtime => format!("realtime, from {base}"),
                        Some(base) => format!("from time {base}, clock {}", b.st.clock),
                        None => "starts at the next round".into(),
                    };
                    let scope = if b.scope_map { ", scope: map" } else { "" };
                    let bind: Vec<String> = b.bind.iter().map(|e| format!("#{e}")).collect();
                    let bind = if bind.is_empty() {
                        String::new()
                    } else {
                        format!(", bind: {}", bind.join(" "))
                    };
                    (
                        format!("scene#{}", b.handle),
                        Node::leaf(format!("{}: {clock}{scope}{bind}", b.call.name)),
                    )
                })
                .collect(),
        );
        Node::Branch(vec![
            ("foreground".into(), foreground),
            ("queue".into(), queue),
            ("background".into(), backgrounds),
            ("starts".into(), Node::leaf(s.scene_starts)),
            (
                "rng".into(),
                Node::leaf(super::save::write_value(&self.w.rng)),
            ),
        ])
    }
}
