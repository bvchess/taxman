//! The tactics: every piece of the algorithm that can be switched, with its name and default.
//!
//! A *tactic* is one choice the solver makes that has a measurable effect on the answers' proof or on the time they
//! take: a shortcut (`prime_shortcut`, `same_search`), or a way to bound or prune the search (`fix_forced`,
//! `nogoods`, `symmetry`, ...).  the table of tactics (solver/docs/design/tactics.md) grades each as proven or sound
//! and says what it does; this module is the one place in the code that knows their names.  The ones that act inside
//! the search are [`SearchTactics`]; the others are only used by the per-game pipeline ([`crate::solver`]).
//!
//! A tactic is an on/off switch.  [`Tactics::set`] reads `name=value` (what `--tactic` takes), [`Tactics::description`]
//! writes the changes from the defaults in the same form, and a result line records every value
//! ([`Tactics::to_json`]).

use crate::json::Json;
use crate::search::SearchTactics;

/// The value of every tactic.  The ones that act inside the search are [`SearchTactics`].  `tests/docs.rs` checks
/// that the table of solver/docs/design/tactics.md lists these, in this order, with these defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tactics {
    pub search: SearchTactics,
    /// A prime `n` plays the game of `n − 1` with its first move replaced by `n` (proven).
    pub prime_shortcut: bool,
    /// Conclude a game whose search problem an earlier game of the chain already solved with that game's
    /// promotions, without a search (proven, given the earlier game's proof; see `crate::solver`).
    pub same_search: bool,
    /// Stop the search as soon as the incumbent reaches `n + opt(n − 1) − in_the_bag` (proven if `opt(n − 1)` is
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
pub const TACTIC_COUNT: usize = 14;

/// The value of a tactic as the command line spells it: `on` or `off`.
pub fn on_off(on: bool) -> &'static str {
    if on {
        "on"
    } else {
        "off"
    }
}

impl Tactics {
    /// The names of the tactics paired with where their values are kept, in the order of the table of tactics.  The
    /// single place in the code that knows the names.
    fn slots_mut(&mut self) -> [(&'static str, &mut bool); TACTIC_COUNT] {
        [
            ("prime_shortcut", &mut self.prime_shortcut),
            ("same_search", &mut self.same_search),
            ("fix_forced", &mut self.search.fix_forced),
            ("nogoods", &mut self.search.nogoods),
            ("forget_nogoods", &mut self.search.forget_nogoods),
            ("symmetry", &mut self.search.symmetry),
            ("conflict_branching", &mut self.search.conflict_branching),
            ("conflict_look", &mut self.search.conflict_look),
            ("near_repeat", &mut self.search.near_repeat),
            ("price_reuse", &mut self.search.price_reuse),
            ("carried_nogoods", &mut self.search.carried_nogoods),
            ("warm_start", &mut self.search.warm_start),
            ("prev_cap", &mut self.prev_cap),
            ("seed_incumbent", &mut self.seed_incumbent),
        ]
    }

    /// Every tactic with its name and value.
    pub fn settings(&self) -> [(&'static str, bool); TACTIC_COUNT] {
        let mut copy = *self;
        copy.slots_mut().map(|(name, slot)| (name, *slot))
    }

    /// The names of the tactics, in the order of the table of tactics.
    pub fn names() -> [&'static str; TACTIC_COUNT] {
        Tactics::default().settings().map(|(name, _)| name)
    }

    /// Applies `name=value`: `on` or `off`.  Nothing changes if it is an error.
    pub fn set(&mut self, assignment: &str) -> Result<(), String> {
        let (name, text) = assignment
            .split_once('=')
            .ok_or_else(|| format!("expected name=value, such as nogoods=off or prev_cap=off, got {assignment:?}"))?;
        let Some((_, slot)) = self.slots_mut().into_iter().find(|(known, _)| *known == name) else {
            return Err(format!("unknown tactic {name:?}; the tactics are {}", Tactics::names().join(", ")));
        };
        *slot = match text {
            "on" => true,
            "off" => false,
            _ => return Err(format!("tactic {name}: expected on or off, got {text:?}")),
        };
        Ok(())
    }

    /// `default`, or the tactics that differ from the defaults (`nogoods=off,prev_cap=off`).
    pub fn description(&self) -> String {
        let defaults = Tactics::default().settings();
        let changed: Vec<String> = (self.settings().into_iter().zip(defaults))
            .filter(|((_, value), (_, default))| value != default)
            .map(|((name, value), _)| format!("{name}={}", on_off(value)))
            .collect();
        if changed.is_empty() {
            "default".to_owned()
        } else {
            changed.join(",")
        }
    }

    /// All tactics, by name, as a JSON object of booleans.
    pub fn to_json(&self) -> Json {
        Json::object(self.settings().map(|(name, value)| (name, Json::Bool(value))))
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
                "fix_forced",
                "nogoods",
                "forget_nogoods",
                "symmetry",
                "conflict_branching",
                "conflict_look",
                "near_repeat",
                "price_reuse",
                "carried_nogoods",
                "warm_start",
                "prev_cap",
                "seed_incumbent"
            ]
        );
        let values: Vec<&str> = t.settings().iter().map(|s| on_off(s.1)).collect();
        let expected = ["on", "on", "on", "on", "on", "on", "on", "on", "on", "on", "off", "on", "on", "on"];
        assert_eq!(values, expected);
        // The values of the search tactics are those of `SearchTactics::default()`, not copies of them.
        assert_eq!(t.search, SearchTactics::default());
    }

    #[test]
    fn tactics_set_by_name() {
        let mut t = Tactics::default();
        t.set("nogoods=off").unwrap();
        t.set("symmetry=off").unwrap();
        t.set("prev_cap=off").unwrap();
        assert!(!t.search.nogoods && !t.search.symmetry && !t.prev_cap);
        assert_eq!(t.description(), "nogoods=off,symmetry=off,prev_cap=off");
        let json = t.to_json();
        assert_eq!(json.get("nogoods").and_then(Json::as_bool), Some(false));
        assert_eq!(json.get("seed_incumbent").and_then(Json::as_bool), Some(true));
        assert_eq!(json.as_obj().map(<[_]>::len), Some(TACTIC_COUNT));
        // The description alone is enough to rebuild the tactics from the defaults, and setting a tactic back to its
        // default drops it from the description.
        let mut again = Tactics::default();
        for assignment in t.description().split(',') {
            again.set(assignment).unwrap();
        }
        assert_eq!(again, t);
        t.set("nogoods=on").unwrap();
        assert_eq!(t.description(), "symmetry=off,prev_cap=off");
    }

    #[test]
    fn a_tactic_that_is_not_one_or_a_value_that_is_not_a_switch_is_refused_and_changes_nothing() {
        for (assignment, expected) in [
            ("nogood=off", "unknown tactic"),
            ("fix_forcd=on", "unknown tactic"),
            ("nogoods=maybe", "tactic nogoods: expected on or off"),
            ("nogoods=2", "tactic nogoods: expected on or off"),
            ("nogoods", "name=value"),
            // The tactics of the matching path, and the knobs of the search that were fixed at their defaults, are
            // gone, whatever their value.
            ("match_refine=1", "unknown tactic"),
            ("lagrange=off", "unknown tactic"),
            ("gf2=on", "unknown tactic"),
            ("force_window=all", "unknown tactic"),
            ("branching=exact", "unknown tactic"),
            ("strong_limit=all", "unknown tactic"),
            ("strong_depth=all", "unknown tactic"),
            ("deep_branching=split", "unknown tactic"),
            ("more_forced=on", "unknown tactic"),
            ("nogood_period=50000", "unknown tactic"),
            ("nogood_keep=none", "unknown tactic"),
            ("nogood_limit=0", "unknown tactic"),
        ] {
            let mut t = Tactics::default();
            let err = t.set(assignment).unwrap_err();
            assert!(err.contains(expected), "{assignment}: {err}");
            assert_eq!(t, Tactics::default(), "{assignment} changed something");
        }
        let err = Tactics::default().set("fix_forcd=on").unwrap_err();
        assert!(err.contains("same_search, fix_forced"), "{err}");
    }

    #[test]
    fn the_conflict_look_is_on_by_default_and_set_by_name() {
        let mut t = Tactics::default();
        assert!(t.search.conflict_branching && t.search.conflict_look);
        t.set("conflict_look=off").unwrap();
        assert!(!t.search.conflict_look);
        assert_eq!(t.description(), "conflict_look=off");
        assert_eq!(t.to_json().get("conflict_look").and_then(Json::as_bool), Some(false));
        assert!(t.set("conflict_look=1").unwrap_err().starts_with("tactic conflict_look: expected on or off"));
        t.set("conflict_look=on").unwrap();
        assert_eq!(t.description(), "default");
    }

    #[test]
    fn the_near_repeat_cut_is_on_by_default_and_set_by_name() {
        let mut t = Tactics::default();
        assert!(t.search.near_repeat);
        t.set("near_repeat=off").unwrap();
        assert!(!t.search.near_repeat);
        assert_eq!(t.description(), "near_repeat=off");
        assert_eq!(t.to_json().get("near_repeat").and_then(Json::as_bool), Some(false));
        assert!(t.set("near_repeat=no").unwrap_err().starts_with("tactic near_repeat: expected on or off"));
        t.set("near_repeat=on").unwrap();
        assert_eq!(t.description(), "default");
    }

    #[test]
    fn the_nogoods_are_forgotten_every_fifty_thousand_steps_unless_forgetting_is_off() {
        let mut t = Tactics::default();
        assert_eq!(t.search.forget_period(), Some(crate::search::NOGOOD_PERIOD));
        t.set("forget_nogoods=off").unwrap();
        assert_eq!(t.search.forget_period(), None);
    }

    #[test]
    fn every_setting_reads_back_as_it_is_shown() {
        // Setting each tactic to the value `settings` shows gives the tactics that show it, and the JSON holds the same
        // values.
        let all = Tactics {
            search: SearchTactics {
                fix_forced: false,
                nogoods: false,
                forget_nogoods: false,
                symmetry: false,
                conflict_branching: false,
                conflict_look: false,
                near_repeat: false,
                price_reuse: true,
                carried_nogoods: true,
                warm_start: true,
            },
            prime_shortcut: false,
            same_search: false,
            prev_cap: false,
            seed_incumbent: false,
        };
        for tactics in [Tactics::default(), all] {
            let mut again = Tactics::default();
            for (name, value) in tactics.settings() {
                again.set(&format!("{name}={}", on_off(value))).unwrap();
            }
            assert_eq!(again, tactics, "{}", tactics.description());
            let json = Json::parse(&tactics.to_json().to_string()).unwrap();
            for (name, value) in tactics.settings() {
                assert_eq!(json.get(name), Some(&Json::Bool(value)), "{name}");
            }
        }
    }
}
