//! The tactics: every piece of the algorithm that can be switched, with its name, value and default.
//!
//! A *tactic* is one choice the solver makes that has a measurable effect on the answers' proof or on the time they
//! take: a shortcut (`prime_shortcut`, `same_search`), or a way to bound or prune the search (`match_refine`,
//! `nogoods`, `lagrange`, ...).  the table of tactics (solver/docs/design/tactics.md) grades each as proven or sound
//! and says what it does; this module is the one place in the code that knows their names.  The ones that act inside
//! the search are [`SearchTactics`]; the others are only used by the per-game pipeline ([`crate::solver`]).
//!
//! A tactic has a value: an on/off switch, the depth of `match_refine`, a count or the name of a policy of the
//! nogood table, or a setting that tunes the GF(2) search: a window, a choice of branching, a count ([`Setting`]).
//! [`Tactics::set`] reads `name=value` (what `--tactic` takes), [`Tactics::description`] writes the changes from the
//! defaults in the same form, and a result line records both that and every value ([`Tactics::to_json`]).

use crate::gf2::ForceWindow;
use crate::json::Json;
use crate::model::Keep;
use crate::search::{Gf2Branching, SearchTactics};
use std::fmt;

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
pub const TACTIC_COUNT: usize = 24;

/// The value of a tactic: an on/off switch, the depth of `match_refine`, a count or the name of a policy of the nogood
/// table, or one of the settings that tune the GF(2) search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Switch(bool),
    /// `match_refine`: 0 to [`SearchTactics::MAX_MATCH_REFINE`].
    MatchRefine(u8),
    /// `force_window`: a distance above `need` (`80`), a percentage of the slack (`25%`), or `all`.
    Window(ForceWindow),
    /// `branching` and `deep_branching`: `largest`, `split`, `exact` or `forced`.
    Branching(Gf2Branching),
    /// `strong_limit` and `strong_depth`: a whole number, or `all` (`u32::MAX`).
    Count(u32),
    /// `nogood_period` and `nogood_limit`: a whole number.
    Number(u64),
    /// `nogood_keep`: what a forgetting of the nogoods keeps.
    Keep(Keep),
}

/// The value as the command line spells it (`on`, `off`, `2`, `50000`, `top64`).
impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Setting::Switch(on) => f.write_str(if *on { "on" } else { "off" }),
            Setting::MatchRefine(depth) => write!(f, "{depth}"),
            Setting::Window(ForceWindow::Within(window)) => write!(f, "{window}"),
            Setting::Window(ForceWindow::Share(percent)) => write!(f, "{percent}%"),
            Setting::Window(ForceWindow::All) | Setting::Count(u32::MAX) => f.write_str("all"),
            Setting::Branching(branching) => f.write_str(branching.name()),
            Setting::Count(count) => write!(f, "{count}"),
            Setting::Number(number) => write!(f, "{number}"),
            Setting::Keep(keep) => write!(f, "{keep}"),
        }
    }
}

impl Setting {
    fn to_json(self) -> Json {
        match self {
            Setting::Switch(on) => Json::Bool(on),
            Setting::MatchRefine(depth) => Json::from(u32::from(depth)),
            Setting::Window(ForceWindow::Within(window)) => Json::from(window),
            Setting::Count(count) if count != u32::MAX => Json::from(count),
            Setting::Number(number) => Json::from(number),
            Setting::Window(_) | Setting::Branching(_) | Setting::Count(_) | Setting::Keep(_) => {
                Json::from(self.to_string())
            }
        }
    }
}

/// Where a tactic's value is kept, so that it can be assigned from text.
enum Slot<'a> {
    Switch(&'a mut bool),
    MatchRefine(&'a mut u8),
    Window(&'a mut ForceWindow),
    Branching(&'a mut Gf2Branching),
    Count(&'a mut u32),
    /// A whole number of at least the second field.
    Number(&'a mut u64, u64),
    Keep(&'a mut Keep),
}

impl Slot<'_> {
    fn get(&self) -> Setting {
        match self {
            Slot::Switch(on) => Setting::Switch(**on),
            Slot::MatchRefine(depth) => Setting::MatchRefine(**depth),
            Slot::Window(window) => Setting::Window(**window),
            Slot::Branching(branching) => Setting::Branching(**branching),
            Slot::Count(count) => Setting::Count(**count),
            Slot::Number(number, _) => Setting::Number(**number),
            Slot::Keep(keep) => Setting::Keep(**keep),
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
            Slot::Window(window) => {
                let share = text.strip_suffix('%').map(|percent| percent.parse().ok().map(ForceWindow::Share));
                **window = match (text, share) {
                    ("all", _) => Some(ForceWindow::All),
                    (_, Some(share)) => share,
                    _ => text.parse().ok().map(ForceWindow::Within),
                }
                .ok_or_else(|| {
                    format!("expected a whole number (80), a percentage from 0% to 255% (25%) or all, got {text:?}")
                })?;
            }
            Slot::Branching(branching) => {
                **branching = Gf2Branching::ALL
                    .into_iter()
                    .find(|b| b.name() == text)
                    .ok_or_else(|| format!("expected largest, split, exact or forced, got {text:?}"))?;
            }
            Slot::Count(count) => {
                **count =
                    match text {
                        "all" => u32::MAX,
                        _ => text.parse().ok().filter(|&count| count < u32::MAX).ok_or_else(|| {
                            format!("expected a whole number below {} or all, got {text:?}", u32::MAX)
                        })?,
                    };
            }
            Slot::Number(number, min) => {
                **number = (text.parse().ok().filter(|&n: &u64| n >= *min))
                    .ok_or_else(|| format!("expected a whole number of at least {min}, got {text:?}"))?;
            }
            Slot::Keep(keep) => {
                **keep = Keep::parse(text).ok_or_else(|| {
                    format!("expected none, hit, sizeN (N at least 1) or topN (N at least 1), got {text:?}")
                })?;
            }
        }
        Ok(())
    }
}

impl Tactics {
    /// The names of the tactics paired with where their values are kept, in the order of the table of tactics.  The
    /// single place in the code that knows the names.
    fn slots_mut(&mut self) -> [(&'static str, Slot<'_>); TACTIC_COUNT] {
        [
            ("prime_shortcut", Slot::Switch(&mut self.prime_shortcut)),
            ("same_search", Slot::Switch(&mut self.same_search)),
            ("match_refine", Slot::MatchRefine(&mut self.search.match_refine)),
            ("fix_forced", Slot::Switch(&mut self.search.fix_forced)),
            ("nogoods", Slot::Switch(&mut self.search.nogoods)),
            ("forget_nogoods", Slot::Switch(&mut self.search.forget_nogoods)),
            ("nogood_period", Slot::Number(&mut self.search.nogood_period, 1)),
            ("nogood_keep", Slot::Keep(&mut self.search.nogood_keep)),
            ("nogood_limit", Slot::Number(&mut self.search.nogood_limit, 0)),
            ("lagrange", Slot::Switch(&mut self.search.lagrange)),
            ("gf2", Slot::Switch(&mut self.search.gf2)),
            ("symmetry", Slot::Switch(&mut self.search.symmetry)),
            ("force_window", Slot::Window(&mut self.search.force_window)),
            ("branching", Slot::Branching(&mut self.search.branching)),
            ("strong_limit", Slot::Count(&mut self.search.strong_limit)),
            ("strong_depth", Slot::Count(&mut self.search.strong_depth)),
            ("deep_branching", Slot::Branching(&mut self.search.deep_branching)),
            ("more_forced", Slot::Switch(&mut self.search.more_forced)),
            ("conflict_branching", Slot::Switch(&mut self.search.conflict_branching)),
            ("conflict_look", Slot::Switch(&mut self.search.conflict_look)),
            ("near_repeat", Slot::Switch(&mut self.search.near_repeat)),
            ("price_reuse", Slot::Switch(&mut self.search.price_reuse)),
            ("prev_cap", Slot::Switch(&mut self.prev_cap)),
            ("seed_incumbent", Slot::Switch(&mut self.seed_incumbent)),
        ]
    }

    /// Every tactic with its name and value.
    pub fn settings(&self) -> [(&'static str, Setting); TACTIC_COUNT] {
        let mut copy = *self;
        copy.slots_mut().map(|(name, slot)| (name, slot.get()))
    }

    /// The names of the tactics, in the order of the table of tactics.
    pub fn names() -> [&'static str; TACTIC_COUNT] {
        Tactics::default().settings().map(|(name, _)| name)
    }

    /// Applies `name=value`: `on` or `off` for a switch, `0` to `7` for `match_refine`, for the settings that tune
    /// the GF(2) search what [`Setting`] says, a whole number for `nogood_period` (at least 1) and `nogood_limit`,
    /// and `none`, `hit`, `sizeN` or `topN` for `nogood_keep`.  Nothing changes if it is an error.
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

    /// All tactics, by name, as a JSON object: booleans for the switches, numbers for `match_refine`, a window
    /// within a distance, a count, `nogood_period` and `nogood_limit`, and strings for the other settings (`25%`,
    /// `all`, `split`, `top64`).
    pub fn to_json(&self) -> Json {
        Json::object(self.settings().map(|(name, value)| (name, value.to_json())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::Forgetting;

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
                "nogood_period",
                "nogood_keep",
                "nogood_limit",
                "lagrange",
                "gf2",
                "symmetry",
                "force_window",
                "branching",
                "strong_limit",
                "strong_depth",
                "deep_branching",
                "more_forced",
                "conflict_branching",
                "conflict_look",
                "near_repeat",
                "price_reuse",
                "prev_cap",
                "seed_incumbent"
            ]
        );
        let values: Vec<String> = t.settings().iter().map(|s| s.1.to_string()).collect();
        let expected = [
            "on", "on", "1", "on", "on", "on", "50000", "none", "0", "off", "on", "on", "all", "exact", "all", "all",
            "split", "on", "on", "on", "on", "off", "on", "on",
        ];
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
            ("force_window=-1", "tactic force_window: expected a whole number (80), a percentage from 0% to 255%"),
            ("force_window=300%", "tactic force_window: expected a whole number (80), a percentage"),
            ("force_window=none", "tactic force_window: expected a whole number (80), a percentage"),
            ("branching=best", "tactic branching: expected largest, split, exact or forced"),
            ("deep_branching=on", "tactic deep_branching: expected largest, split, exact or forced"),
            ("strong_limit=4294967295", "tactic strong_limit: expected a whole number below 4294967295 or all"),
            ("strong_depth=-3", "tactic strong_depth: expected a whole number below 4294967295 or all"),
            ("more_forced=all", "tactic more_forced: expected on or off"),
        ] {
            let err = t.set(assignment).unwrap_err();
            assert!(err.starts_with(expected), "{assignment}: {err}");
            assert_eq!(t, Tactics::default(), "{assignment} changed something");
        }
        let err = t.set("match_refines=1").unwrap_err();
        assert!(err.contains("unknown tactic") && err.contains("same_search, match_refine"), "{err}");

        // The settings that tune the GF(2) search, as `--tactic` spells them and as the JSON has them.
        for (assignment, json) in [
            ("force_window=0", Json::from(0u32)),
            ("force_window=120", Json::from(120u32)),
            ("force_window=25%", Json::from("25%")),
            ("branching=split", Json::from("split")),
            ("strong_limit=2", Json::from(2u32)),
            ("strong_depth=40", Json::from(40u32)),
            ("deep_branching=largest", Json::from("largest")),
        ] {
            let mut t = Tactics::default();
            t.set(assignment).unwrap();
            assert_eq!(t.description(), assignment);
            let (name, _) = assignment.split_once('=').unwrap();
            assert_eq!(t.to_json().get(name), Some(&json), "{assignment}");
        }
        let mut t = Tactics::default();
        t.set("strong_limit=3").unwrap();
        t.set("strong_limit=all").unwrap();
        assert_eq!(t.description(), "default");
        assert_eq!(t.to_json().get("strong_limit"), Some(&Json::from("all")));
        // The window of the forced members is `all` by default (it was 80 until 2026-10-16), and is not mentioned.
        t.set("force_window=80").unwrap();
        assert_eq!(t.description(), "force_window=80");
        t.set("force_window=all").unwrap();
        assert_eq!(t.description(), "default");
        assert_eq!(t.to_json().get("force_window"), Some(&Json::from("all")));
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
    fn the_policy_of_keeping_nogoods_is_set_by_three_tactics() {
        // The defaults forget everything after each 50,000 steps, as before the knobs.
        assert_eq!(Tactics::default().search.forgetting(), Forgetting::DEFAULT);
        let mut t = Tactics::default();
        t.set("nogood_period=200000").unwrap();
        t.set("nogood_keep=top64").unwrap();
        t.set("nogood_limit=256").unwrap();
        assert_eq!(t.description(), "nogood_period=200000,nogood_keep=top64,nogood_limit=256");
        let expected = Forgetting { period: Some(200_000), keep: Keep::Top(64), limit: Some(256) };
        assert_eq!(t.search.forgetting(), expected);
        // `forget_nogoods=off` is never forgetting, whatever the period.
        t.set("forget_nogoods=off").unwrap();
        assert_eq!(t.search.forgetting(), Forgetting { period: None, ..expected });
        let json = t.to_json();
        assert_eq!(json.get("nogood_period").and_then(Json::as_u64), Some(200_000));
        assert_eq!(json.get("nogood_keep").and_then(Json::as_str), Some("top64"));
        assert_eq!(json.get("nogood_limit").and_then(Json::as_u64), Some(256));
        // The policy is not what a checkpoint records, but a run can take another's.
        let recorded = SearchTactics { nogood_keep: Keep::Used, ..SearchTactics::default() };
        assert!(recorded.same_recorded(&SearchTactics::default()));
        assert!(!recorded.same_recorded(&SearchTactics { nogoods: false, ..SearchTactics::default() }));
        let resumed = SearchTactics::default().with_forgetting_of(&t.search);
        assert_eq!(resumed, SearchTactics { forget_nogoods: true, ..t.search });
        for (assignment, expected) in [
            ("nogood_period=0", "tactic nogood_period: expected a whole number of at least 1"),
            ("nogood_period=-1", "tactic nogood_period: expected a whole number of at least 1"),
            ("nogood_period=often", "tactic nogood_period: expected a whole number of at least 1"),
            ("nogood_limit=-3", "tactic nogood_limit: expected a whole number of at least 0"),
            ("nogood_keep=most", "tactic nogood_keep: expected none, hit, sizeN"),
            ("nogood_keep=size0", "tactic nogood_keep: expected none, hit, sizeN"),
            ("nogood_keep=on", "tactic nogood_keep: expected none, hit, sizeN"),
        ] {
            let mut again = Tactics::default();
            let err = again.set(assignment).unwrap_err();
            assert!(err.starts_with(expected), "{assignment}: {err}");
            assert_eq!(again, Tactics::default(), "{assignment} changed something");
        }
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
                nogood_period: 200_000,
                nogood_keep: Keep::Top(64),
                nogood_limit: 256,
                lagrange: false,
                gf2: true,
                symmetry: false,
                force_window: ForceWindow::Share(30),
                branching: Gf2Branching::Forced,
                strong_limit: 3,
                strong_depth: 0,
                deep_branching: Gf2Branching::Largest,
                more_forced: false,
                conflict_branching: false,
                conflict_look: false,
                near_repeat: false,
                price_reuse: true,
            },
            prime_shortcut: false,
            same_search: false,
            prev_cap: false,
            seed_incumbent: false,
        });
        // Every value of the settings that tune the GF(2) search.
        let search = SearchTactics::default();
        for force_window in [ForceWindow::Within(0), ForceWindow::Within(250), ForceWindow::Share(0), ForceWindow::All]
        {
            all.push(Tactics { search: SearchTactics { force_window, ..search }, ..Tactics::default() });
        }
        for branching in Gf2Branching::ALL {
            all.push(Tactics {
                search: SearchTactics { branching, deep_branching: branching, ..search },
                ..Tactics::default()
            });
        }
        for count in [0, 1, 7, u32::MAX - 1] {
            all.push(Tactics {
                search: SearchTactics { strong_limit: count, strong_depth: count, ..search },
                ..Tactics::default()
            });
        }
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
