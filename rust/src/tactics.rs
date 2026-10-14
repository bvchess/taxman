//! The tactics: every piece of the algorithm that can be switched, with its name, value and default.
//!
//! A *tactic* is one choice the solver makes that has a measurable effect on the answers' proof or on the time
//! they take: a shortcut (`prime_shortcut`, `same_search`), or a way to bound or prune the search
//! (`match_refine`, `nogoods`, `lagrange`, ...).  DESIGN.md's table of tactics ("Tactics") grades each as proven or
//! sound and says what it does; this module is the one place in the code that knows their names.  The ones that act
//! inside the search are [`SearchTactics`]; the others are only used by the per-game pipeline ([`crate::solver`]).
//!
//! A tactic has a value: an on/off switch, or the depth of `match_refine` ([`Setting`]).  [`Tactics::set`]
//! reads `name=value` (what `--tactic` takes), [`Tactics::description`] writes the changes from the defaults in
//! the same form, and a result line records both that and every value ([`Tactics::to_json`]).

use crate::json::Json;
use crate::search::SearchTactics;
use std::fmt;

/// The value of every tactic.  The ones that act inside the search are [`SearchTactics`].  `tests/docs.rs` checks
/// that DESIGN.md's table of tactics lists these, in this order, with these defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tactics {
    pub search: SearchTactics,
    /// A prime `n` plays the game of `n − 1` with its first move replaced by `n` (proven).
    pub prime_shortcut: bool,
    /// Conclude a game whose search problem an earlier game of the chain already solved with that game's
    /// promotions, without a search (proven, given the earlier game's proof; see `crate::solver`).
    pub same_search: bool,
    /// Stop the search as soon as the incumbent reaches `n + opt(n − 1) − inTheBag` (proven if `opt(n − 1)` is
    /// optimal).
    pub prev_cap: bool,
    /// Start the search from the better of the greedy set and the previous game's promotions (sound).
    pub seed_incumbent: bool,
}

impl Default for Tactics {
    fn default() -> Self {
        Tactics {
            search: SearchTactics::default(),
            prime_shortcut: true,
            same_search: true,
            prev_cap: true,
            seed_incumbent: true,
        }
    }
}

/// The number of tactics.
pub const TACTIC_COUNT: usize = 11;

/// The value of a tactic: an on/off switch, or the depth of `match_refine`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Switch(bool),
    /// `match_refine`: 0 to [`SearchTactics::MAX_MATCH_REFINE`].
    MatchRefine(u8),
}

/// The value as the command line spells it (`on`, `off`, `2`).
impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Setting::Switch(on) => f.write_str(if *on { "on" } else { "off" }),
            Setting::MatchRefine(depth) => write!(f, "{depth}"),
        }
    }
}

impl Setting {
    fn to_json(self) -> Json {
        match self {
            Setting::Switch(on) => Json::Bool(on),
            Setting::MatchRefine(depth) => Json::from(u32::from(depth)),
        }
    }
}

/// Where a tactic's value is kept, so that it can be assigned from text.
enum Slot<'a> {
    Switch(&'a mut bool),
    MatchRefine(&'a mut u8),
}

impl Slot<'_> {
    fn get(&self) -> Setting {
        match self {
            Slot::Switch(on) => Setting::Switch(**on),
            Slot::MatchRefine(depth) => Setting::MatchRefine(**depth),
        }
    }

    /// Reads `text` as this kind of value and takes it; leaves the value alone if `text` is not one.
    fn assign(&mut self, text: &str) -> Result<(), String> {
        match self {
            Slot::Switch(on) => {
                **on = match text {
                    "on" => true,
                    "off" => false,
                    _ => return Err(format!("expected on or off, got {text:?}")),
                }
            }
            Slot::MatchRefine(depth) => {
                let max = SearchTactics::MAX_MATCH_REFINE;
                **depth = (text.parse().ok().filter(|&depth| depth <= max))
                    .ok_or_else(|| format!("expected a whole number from 0 to {max}, got {text:?}"))?;
            }
        }
        Ok(())
    }
}

impl Tactics {
    /// The names of the tactics paired with where their values are kept, in the order of DESIGN.md's table.  The
    /// single place in the code that knows the names.
    fn slots_mut(&mut self) -> [(&'static str, Slot<'_>); TACTIC_COUNT] {
        [
            ("prime_shortcut", Slot::Switch(&mut self.prime_shortcut)),
            ("same_search", Slot::Switch(&mut self.same_search)),
            ("match_refine", Slot::MatchRefine(&mut self.search.match_refine)),
            ("fix_forced", Slot::Switch(&mut self.search.fix_forced)),
            ("nogoods", Slot::Switch(&mut self.search.nogoods)),
            ("forget_nogoods", Slot::Switch(&mut self.search.forget_nogoods)),
            ("lagrange", Slot::Switch(&mut self.search.lagrange)),
            ("gf2", Slot::Switch(&mut self.search.gf2)),
            ("symmetry", Slot::Switch(&mut self.search.symmetry)),
            ("prev_cap", Slot::Switch(&mut self.prev_cap)),
            ("seed_incumbent", Slot::Switch(&mut self.seed_incumbent)),
        ]
    }

    /// Every tactic with its name and value.
    pub fn settings(&self) -> [(&'static str, Setting); TACTIC_COUNT] {
        let mut copy = *self;
        copy.slots_mut().map(|(name, slot)| (name, slot.get()))
    }

    /// The names of the tactics, in the order of DESIGN.md's table.
    pub fn names() -> [&'static str; TACTIC_COUNT] {
        Tactics::default().settings().map(|(name, _)| name)
    }

    /// Applies `name=value`: `on` or `off` for a switch, `0` to `7` for `match_refine`.  Nothing changes if it is an
    /// error.
    pub fn set(&mut self, assignment: &str) -> Result<(), String> {
        let (name, text) = assignment.split_once('=').ok_or_else(|| {
            format!("expected name=value, such as match_refine=0 or prev_cap=off, got {assignment:?}")
        })?;
        let Some((_, mut slot)) = self.slots_mut().into_iter().find(|(known, _)| *known == name) else {
            return Err(format!("unknown tactic {name:?}; the tactics are {}", Tactics::names().join(", ")));
        };
        slot.assign(text).map_err(|e| format!("tactic {name}: {e}"))
    }

    /// `default`, or the tactics that differ from the defaults (`match_refine=0,prev_cap=off`).
    pub fn description(&self) -> String {
        let defaults = Tactics::default().settings();
        let changed: Vec<String> = (self.settings().into_iter().zip(defaults))
            .filter(|((_, value), (_, default))| value != default)
            .map(|((name, value), _)| format!("{name}={value}"))
            .collect();
        if changed.is_empty() {
            "default".to_owned()
        } else {
            changed.join(",")
        }
    }

    /// All tactics, by name, as a JSON object: booleans for the switches and a number for `match_refine`.
    pub fn to_json(&self) -> Json {
        Json::object(self.settings().map(|(name, value)| (name, value.to_json())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tactics_defaults_and_settings() {
        let t = Tactics::default();
        assert_eq!(t.description(), "default");
        assert_eq!(
            Tactics::names(),
            [
                "prime_shortcut",
                "same_search",
                "match_refine",
                "fix_forced",
                "nogoods",
                "forget_nogoods",
                "lagrange",
                "gf2",
                "symmetry",
                "prev_cap",
                "seed_incumbent"
            ]
        );
        let values: Vec<String> = t.settings().iter().map(|s| s.1.to_string()).collect();
        let expected = ["on", "on", "1", "on", "on", "on", "off", "on", "on", "on", "on"];
        assert_eq!(values, expected);
        // The values of the search tactics are those of `SearchTactics::default()`, not copies of them.
        assert_eq!(t.search, SearchTactics::default());
    }

    #[test]
    fn tactics_set_by_name() {
        let mut t = Tactics::default();
        t.set("nogoods=off").unwrap();
        t.set("gf2=off").unwrap();
        t.set("prev_cap=off").unwrap();
        assert!(!t.search.nogoods && !t.search.gf2 && !t.prev_cap);
        assert_eq!(t.description(), "nogoods=off,gf2=off,prev_cap=off");
        assert!(t.set("nogood=off").unwrap_err().contains("unknown tactic"));
        assert!(t.set("nogoods=maybe").unwrap_err().contains("on or off"));
        assert!(t.set("nogoods").unwrap_err().contains("name=value"));
        let json = t.to_json();
        assert_eq!(json.get("nogoods").and_then(Json::as_bool), Some(false));
        assert_eq!(json.get("seed_incumbent").and_then(Json::as_bool), Some(true));
    }

    #[test]
    fn tactics_with_values_other_than_on_and_off() {
        let mut t = Tactics::default();
        t.set("match_refine=0").unwrap();
        assert_eq!(t.search.match_refine, 0);
        assert_eq!(t.description(), "match_refine=0");
        let json = t.to_json();
        assert_eq!(json.get("match_refine").and_then(Json::as_u32), Some(0));
        assert_eq!(json.get("prev_cap").and_then(Json::as_bool), Some(true));
        assert_eq!(json.as_obj().map(<[_]>::len), Some(TACTIC_COUNT));

        // The default value is not mentioned, and setting a tactic back to it drops it from the description.
        t.set("match_refine=7").unwrap();
        assert_eq!(t.description(), "match_refine=7");
        t.set("match_refine=1").unwrap();
        assert_eq!(t.description(), "default");

        // Bad values are refused with a message that says what is allowed, and change nothing.
        for (assignment, expected) in [
            ("match_refine=8", "tactic match_refine: expected a whole number from 0 to 7, got \"8\""),
            ("match_refine=-1", "tactic match_refine: expected a whole number from 0 to 7"),
            ("match_refine=1.5", "tactic match_refine: expected a whole number from 0 to 7"),
            ("match_refine=on", "tactic match_refine: expected a whole number from 0 to 7"),
            ("match_refine=", "tactic match_refine: expected a whole number from 0 to 7"),
            ("nogoods=2", "tactic nogoods: expected on or off"),
            ("nogoods=largest", "tactic nogoods: expected on or off"),
        ] {
            let err = t.set(assignment).unwrap_err();
            assert!(err.starts_with(expected), "{assignment}: {err}");
            assert_eq!(t, Tactics::default(), "{assignment} changed something");
        }
        let err = t.set("match_refines=1").unwrap_err();
        assert!(err.contains("unknown tactic") && err.contains("same_search, match_refine"), "{err}");
    }

    #[test]
    fn every_setting_reads_back_as_it_is_shown() {
        // Setting each tactic to the value `settings` shows gives the tactics that show it, and so do the
        // changes in the description; the JSON holds the same values.
        let mut t = Tactics::default();
        let mut all = vec![Tactics::default()];
        for depth in 0..=SearchTactics::MAX_MATCH_REFINE {
            t.search = SearchTactics { match_refine: depth, ..t.search };
            all.push(t);
        }
        all.push(Tactics {
            search: SearchTactics {
                match_refine: 5,
                fix_forced: false,
                nogoods: false,
                forget_nogoods: false,
                lagrange: false,
                gf2: true,
                symmetry: false,
            },
            prime_shortcut: false,
            same_search: false,
            prev_cap: false,
            seed_incumbent: false,
        });
        for tactics in all {
            let mut again = Tactics::default();
            for (name, value) in tactics.settings() {
                again.set(&format!("{name}={value}")).unwrap();
            }
            assert_eq!(again, tactics, "{}", tactics.description());
            // The description alone is enough to rebuild the tactics from the defaults.
            let mut from_description = Tactics::default();
            if tactics.description() != "default" {
                for assignment in tactics.description().split(',') {
                    from_description.set(assignment).unwrap();
                }
            }
            assert_eq!(from_description, tactics, "{}", tactics.description());
            let json = Json::parse(&tactics.to_json().to_string()).unwrap();
            for (name, value) in tactics.settings() {
                assert_eq!(json.get(name), Some(&value.to_json()), "{name}");
            }
        }
    }
}
