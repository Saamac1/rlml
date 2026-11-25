//! The profile file (M 5.6, D 3.5): the game's state over sessions, which
//! the engine keeps in a store of its own beside the saves.
//!
//! The file holds components and a checksum of its text and the game's
//! name. A session reads it at its start (its snapshot) and writes the
//! fields it changed back at each action's end, merged into the file as
//! it is then, so another session's fields stay. A component the data no
//! longer has stays in the file, unseen, until it comes back.

use super::save::{read_value, write_value};
use super::store::Store;
use super::value::{ComponentValue, Value};
use super::world::RtResult;
use crate::model::Model;

/// The profile's name in its store, and the backup's.
pub const FILE: &str = "profile";
pub const BACKUP: &str = "profile.bak";

fn checksum(game: &str, body: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in game.bytes().chain([0]).chain(body.bytes()) {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// The file's text for `components`.
pub fn write(game: &str, components: &[ComponentValue]) -> String {
    let body = write_value(&components.to_vec());
    format!("rlml-profile 1\nsum {:016x}\n{body}\n", checksum(game, &body))
}

/// Why a file was not read.
#[derive(Debug, Clone, PartialEq)]
pub enum Unread {
    /// The checksum does not match: the file was edited, or is of another
    /// game.
    Changed,
    Broken(String),
}

/// The components of a profile file.
pub fn read(game: &str, text: &str) -> Result<Vec<ComponentValue>, Unread> {
    let mut lines = text.lines();
    if lines.next() != Some("rlml-profile 1") {
        return Err(Unread::Broken("not a profile: the first line is `rlml-profile 1`".into()));
    }
    let sum = lines.next().and_then(|l| l.strip_prefix("sum ")).and_then(|s| u64::from_str_radix(s, 16).ok()).ok_or_else(|| Unread::Broken("the profile has no `sum`".into()))?;
    let body = lines.next().unwrap_or("");
    if checksum(game, body) != sum {
        return Err(Unread::Changed);
    }
    read_value(body).map_err(Unread::Broken)
}

/// The game's profile in a store.
pub struct Profile<S: Store> {
    pub store: S,
    game: String,
}

fn game_name(m: &Model) -> String {
    m.named_game().map(|g| g.name.clone()).unwrap_or_default()
}

impl<S: Store> Profile<S> {
    pub fn new(store: S, m: &Model) -> Profile<S> {
        Profile { store, game: game_name(m) }
    }

    /// The file's components, or none without a file. A file whose
    /// checksum does not match is kept as the backup, and the profile
    /// starts from the data's values with the warning given (M 5.6).
    pub fn load(&mut self) -> (Vec<ComponentValue>, Option<String>) {
        let Some(text) = self.store.read(FILE) else { return (vec![], None) };
        match read(&self.game, &text) {
            Ok(c) => (c, None),
            Err(why) => {
                let why = match why {
                    Unread::Changed => "its checksum does not match".to_string(),
                    Unread::Broken(e) => e,
                };
                let kept = self.store.write(BACKUP, &text).is_ok();
                if kept {
                    self.store.remove(FILE);
                }
                (vec![], Some(format!("the profile was not read ({why}); it starts over{}", if kept { ", the old one kept as profile.bak" } else { "" })))
            }
        }
    }

    /// Merges a session's writes into the file as it is now (M 5.6).
    pub fn write(&mut self, writes: &[(String, String, Value)]) -> RtResult<()> {
        if writes.is_empty() {
            return Ok(());
        }
        let mut file = match self.store.read(FILE).map(|t| read(&self.game, &t)) {
            Some(Ok(c)) => c,
            _ => vec![],
        };
        for (comp, field, v) in writes {
            match file.iter_mut().find(|c| &c.name == comp) {
                Some(c) => c.set(field, v.clone()),
                None => file.push(ComponentValue { name: comp.clone(), fields: vec![(field.clone(), v.clone())] }),
            }
        }
        self.store.write(FILE, &write(&self.game, &file))
    }

    /// Resets the profile: the engine's menu does, never the logic (M 5.6).
    pub fn reset(&mut self) {
        self.store.remove(FILE);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn comp(name: &str, f: &str, n: i64) -> ComponentValue {
        ComponentValue { name: name.into(), fields: vec![(f.into(), Value::Number(n))] }
    }

    #[test]
    fn a_file_reads_back_and_an_edit_shows() {
        let c = vec![comp("UNLOCKS", "wizard", 1)];
        let text = write("ROGUE", &c);
        assert_eq!(read("ROGUE", &text), Ok(c));
        assert_eq!(read("OTHER", &text), Err(Unread::Changed), "the game's name is in the sum");
        let edited = text.replace("(n 1)", "(n 9)");
        assert_eq!(read("ROGUE", &edited), Err(Unread::Changed));
    }
}
