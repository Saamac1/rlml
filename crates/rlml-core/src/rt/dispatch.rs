//! The handlers of each event and phase, laid out once from the model
//! (RLML-kehitystyokalut.md step 2): general handlers and component
//! handlers in the order they run, priority first and then load order
//! (M 6.2). The model does not change after it is built, so the table is
//! a cache beside it; an event then asks only which component handlers
//! its subject has.

use std::collections::HashMap;
use std::sync::Arc;

use super::eval::HandlerRef;
use crate::model::Model;
use crate::names;
use crate::rs::ast::{ItemKind, Phase};

/// A handler in the table: its order keys and, for a component's
/// handler, the component the subject must have.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) component: Option<String>,
    pub(crate) handler: HandlerRef,
}

/// Every event's handlers by phase (`before`, `on`, `after`).
#[derive(Debug, Default)]
pub struct Dispatch {
    by_event: HashMap<String, [Vec<Entry>; 3]>,
}

/// A handler with its sort keys: priority, load order and its index in
/// its component.
type Keyed = (i64, usize, usize, Entry);

fn phase_index(p: Phase) -> usize {
    match p {
        Phase::Before => 0,
        Phase::On => 1,
        Phase::After => 2,
    }
}

/// An event's full name as a handler writes it: an engine event as it is,
/// a game's event resolved in the handler's namespace.
fn event_name(m: &Model, written: &str, ns: Option<&str>) -> String {
    if names::is_vocabulary(written) {
        return written.to_string();
    }
    m.resolve(ns, written)
        .map(|d| d.full.clone())
        .unwrap_or_else(|_| written.to_string())
}

impl Dispatch {
    pub fn build(m: &Model) -> Dispatch {
        let mut keyed: HashMap<String, [Vec<Keyed>; 3]> = HashMap::new();
        for (order, d) in m.code.iter().enumerate() {
            let ns = d.namespace.as_deref();
            match &d.item.kind {
                ItemKind::Handler(h) => {
                    // a named handler runs only as the definition that won:
                    // `override on … as NAME` replaces it (RS 4, D 4.5)
                    if !d.name.is_empty()
                        && let Some(def) = m.defs.get(&names::qualify(ns, &d.name))
                        && (def.file != d.file || def.line != d.line)
                    {
                        continue;
                    }
                    let entry = Entry {
                        component: None,
                        handler: HandlerRef {
                            handler: Arc::new(h.clone()),
                            component: None,
                            name: d.name.clone(),
                            ns: d.namespace.clone(),
                            file: d.file.clone(),
                        },
                    };
                    keyed.entry(event_name(m, &h.event, ns)).or_default()[phase_index(h.phase)]
                        .push((h.priority, order, 0, entry));
                }
                ItemKind::Component { handlers, .. } => {
                    let full = names::qualify(ns, &d.name);
                    for (k, h) in handlers.iter().enumerate() {
                        let entry = Entry {
                            component: Some(full.clone()),
                            handler: HandlerRef {
                                handler: Arc::new(h.clone()),
                                component: Some(full.clone()),
                                name: full.clone(),
                                ns: d.namespace.clone(),
                                file: d.file.clone(),
                            },
                        };
                        keyed.entry(event_name(m, &h.event, ns)).or_default()[phase_index(h.phase)]
                            .push((h.priority, order, k, entry));
                    }
                }
                _ => {}
            }
        }
        let by_event = keyed
            .into_iter()
            .map(|(event, phases)| {
                let phases = phases.map(|mut list| {
                    list.sort_by_key(|(p, o, k, _)| (-p, *o, *k));
                    list.into_iter().map(|(_, _, _, e)| e).collect()
                });
                (event, phases)
            })
            .collect();
        Dispatch { by_event }
    }

    /// An event phase's handlers in the order they run, component
    /// handlers among them for any subject.
    pub(crate) fn entries(&self, event: &str, phase: Phase) -> &[Entry] {
        self.by_event
            .get(event)
            .map(|p| p[phase_index(phase)].as_slice())
            .unwrap_or(&[])
    }
}
