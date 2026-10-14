//! The threads of a window: how the driver shares the run's threads among the games in flight.
//!
//! The run's `--threads T` is a budget.  A game holds threads while it works: a setup one, a search the workers of its
//! engine (the larger of the number it was told to have and the workers still alive, since a lowered number takes
//! effect only as workers come idle and retire; in its start, the dive and the local search, the number it was given).
//! The driver's own work (the repeat lookups and the conclusions) is not counted.  The threads not held are *free*.
//!
//! [`allot()`] is the policy, a pure function of what the games hold and claim, so that it is tested without threads;
//! the driver ([`super`]) samples the searches, calls it, and carries out what it grants.  The driver is the only one
//! that changes a search's number of workers ([`Control::set_workers`]).
//!
//! # The age-based policy
//!
//! Every [`TICK`] while a search runs or a game waits for a thread, the driver samples each running engine
//! ([`Control::status`]) and takes its *idle share* over the tick: the idle time of its workers over their capacity,
//! both as the difference from the last sample.  Then, with the games oldest first:
//!
//! * *Claims*, served oldest game first while threads are free: a setup waiting for a thread asks one; a search
//!   ready to start asks every free thread if it is the only claim and no older game of the window may still start a
//!   search (so a game that enters the window alone, as every game does with a window of one, starts on all the
//!   threads), else one; a running search that is *saturated*, idle less than [`SATURATED`] of the last tick or still
//!   in its start (whose local search uses every thread it has), asks to double: as many more as it has.  In its start
//!   it has no engine to sample, and asks once a tick all the same, so that it grows by doubling at the pace of the
//!   ticks, not all at once, and the threads it has not grown to are left to the other games meanwhile.
//! * *Releases*, only while some other game has a claim: a running search idle at least [`IDLE`] of the last tick,
//!   with idle workers now, and last given threads at least [`SETTLE`] ago gives back the smaller of its idle workers
//!   and the workers its idle time over the tick amounts to, keeping at least one.  Its engine retires them as they
//!   come idle, and only then are they free.
//!
//! Each sample of an engine, and each tick of a search in its start, is acted on once: between ticks (the driver also
//! runs the policy when a setup or a search ends, so that a game waits for no tick to start) a saturated search still
//! counts as a claim, so that younger games do not take all the free threads past it, but asks for nothing more until
//! the next sample.  A search just given its threads has nothing to act on yet.
//!
//! So a search earns threads by keeping them busy.  A dear game ends up with every thread: it stays saturated, so it
//! keeps doubling while threads are free, and the younger games' searches give their idle workers back to it.  A short
//! game finishes before it has grown.  Threads freed by a search that ends or idles go to the oldest game that can use
//! them.  This is the online form of a running estimate of a search's tree: what a search does with its threads is
//! measured instead of what is left of its tree estimated (Knuth's estimator of the size of a tree from random paths
//! was considered and not used).  The constants can only be tuned on a machine of many threads.
//!
//! The one change from claims served strictly by age: a ready search takes every free thread only when no older game
//! may still start a search.  A younger game whose search needs no seed (one resumed from its checkpoint, or with
//! `seed_incumbent` off) can be ready before an older one, and would otherwise take every thread and keep them as
//! long as it stays busy, while the older game, which the chain waits for, starts on whatever is left.
//!
//! [`Control::set_workers`]: crate::engine::Control::set_workers
//! [`Control::status`]: crate::engine::Control::status

use std::time::Duration;

/// How often the driver samples the running engines and shares the threads again.  Short against a game worth moving
/// threads for (most searches of the chain take milliseconds, the dear ones minutes to hours), long against the cost
/// of a sample (a lock of each engine's state).
pub(super) const TICK: Duration = Duration::from_millis(5);

/// How long a search keeps threads it was just given before it can be asked to give any back: newly started workers
/// are idle until the first splits have given them work, which takes a few ticks.
pub(super) const SETTLE: Duration = Duration::from_millis(10);

/// Below this idle share over a tick a search is saturated: its workers have work, so more could help.
pub(super) const SATURATED: f64 = 0.05;

/// From this idle share over a tick a search gives idle workers back when another game has a claim: a quarter of its
/// threads waiting for work is more than the splits of a busy search leave idle.
pub(super) const IDLE: f64 = 0.25;

/// What a game of the window holds or asks for, as the driver hands it to [`allot()`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Need {
    /// Its setup waits for a thread.
    SetUp,
    /// Its setup runs: it holds one thread.
    SettingUp,
    /// Set up for a search that cannot start yet (its repeat lookup is not made, or the game before it has no game
    /// yet to seed it with): it may claim threads later.
    Pending,
    /// Its search can start.
    Ready,
    /// Its search runs.
    Running(Running),
    /// It holds and asks nothing: a prime, trivial or repeat game, a search that has ended, or a failure.
    Nothing,
}

/// A running search, as [`allot()`] sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Running {
    /// The workers it was last told to have.
    pub target: usize,
    /// Its engine's live workers at the last sample (0 before the engine starts).
    pub live: usize,
    pub load: Load,
    /// The time since it was last given threads.
    pub since_grant: Duration,
}

/// What a running search does with its threads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Load {
    /// In its start (the dive and the local search): there is no engine to sample yet.  `fresh` from a tick until the
    /// policy has run with it; not when the search has just been given its threads.
    Starting { fresh: bool },
    /// Its engine over the last tick: the idle share, the workers idle at the sample, and the idle time summed over the
    /// workers.  `fresh` until the policy has run with it.
    Sampled { idle_share: f64, idle_now: usize, idle_time: Duration, fresh: bool },
    /// Not known: its engine has just started (one sample, nothing to compare it with yet) or has ended.
    Unknown,
}

/// What [`allot()`] grants a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Grant {
    /// Start its setup, on one thread.
    SetUp,
    /// Start its search with this many workers.
    Start(usize),
    /// Its search's new number of workers: more for a claim, fewer for a release.
    Workers(usize),
}

impl Running {
    /// The threads it holds.
    fn holds(&self) -> usize {
        self.target.max(self.live)
    }

    /// Whether its threads are all busy, so that it claims more.
    fn saturated(&self) -> bool {
        match self.load {
            Load::Starting { .. } => true,
            Load::Sampled { idle_share, .. } => idle_share < SATURATED,
            Load::Unknown => false,
        }
    }

    /// How many more threads it asks for: as many as it has (its target in its start, where it has no workers yet),
    /// once per sample.
    fn asks(&self) -> usize {
        match self.load {
            Load::Starting { fresh: true } => self.target,
            Load::Sampled { fresh: true, .. } => self.live,
            Load::Starting { .. } | Load::Sampled { .. } | Load::Unknown => 0,
        }
    }

    /// How many workers it gives back: the smaller of its idle workers and the workers its idle time over the last
    /// tick amounts to, keeping at least one; none unless it idles enough and has settled.
    fn gives(&self) -> usize {
        let Load::Sampled { idle_share, idle_now, idle_time, fresh: true } = self.load else { return 0 };
        if idle_share < IDLE || self.since_grant < SETTLE {
            return 0;
        }
        let idle_workers = usize::try_from(idle_time.as_nanos() / TICK.as_nanos()).unwrap_or(usize::MAX);
        idle_now.min(idle_workers).min(self.target.saturating_sub(1))
    }
}

impl Need {
    /// Whether it claims threads now.
    fn claims(&self) -> bool {
        match self {
            Need::SetUp | Need::Ready => true,
            Need::Running(running) => running.saturated(),
            Need::SettingUp | Need::Pending | Need::Nothing => false,
        }
    }

    /// Whether it may still start a search: a setup not done, or a search set up and not started.
    fn may_start(&self) -> bool {
        matches!(self, Need::SetUp | Need::SettingUp | Need::Pending | Need::Ready)
    }
}

/// The grants of one round of the policy (see the module documentation) for the games of `needs`, oldest first, with
/// `threads` threads in all: each as the index of its game in `needs`, and what it is granted.
pub(super) fn allot(threads: usize, needs: &[Need]) -> Vec<(usize, Grant)> {
    let held: usize = needs
        .iter()
        .map(|need| match need {
            Need::SettingUp => 1,
            Need::Running(running) => running.holds(),
            _ => 0,
        })
        .sum();
    let mut free = threads.saturating_sub(held);
    let claims: Vec<usize> = (0..needs.len()).filter(|&i| needs[i].claims()).collect();
    let mut grants = Vec::new();
    if !claims.is_empty() {
        // A search that releases idles, so it does not claim: the claims are other games'.
        for (i, need) in needs.iter().enumerate() {
            if let Need::Running(running) = need {
                let gives = running.gives();
                if gives > 0 {
                    grants.push((i, Grant::Workers(running.target - gives)));
                }
            }
        }
    }
    for &i in &claims {
        if free == 0 {
            break;
        }
        match needs[i] {
            Need::SetUp => {
                grants.push((i, Grant::SetUp));
                free -= 1;
            }
            Need::Ready => {
                let alone = claims.len() == 1 && !needs[..i].iter().any(Need::may_start);
                let workers = if alone { free } else { 1 };
                grants.push((i, Grant::Start(workers)));
                free -= workers;
            }
            Need::Running(running) => {
                let more = running.asks().min(free);
                if more > 0 {
                    grants.push((i, Grant::Workers(running.target + more)));
                    free -= more;
                }
            }
            Need::SettingUp | Need::Pending | Need::Nothing => unreachable!("only claims are served"),
        }
    }
    grants
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A search in its start, given `target` workers `since` ago, with a tick not acted on yet if `fresh`.
    fn starting(target: usize, fresh: bool, since: Duration) -> Need {
        Need::Running(Running { target, live: 0, load: Load::Starting { fresh }, since_grant: since })
    }

    /// A search whose engine runs `live` of `target` workers, idle `idle_share` of the last tick with `idle_now` idle,
    /// and was given threads `since` ago.
    fn sampled(target: usize, live: usize, idle_share: f64, idle_now: usize, since: Duration) -> Need {
        // The idle time over the tick that the share gives with `live` workers.
        let idle_time = TICK.mul_f64(idle_share * live as f64);
        let load = Load::Sampled { idle_share, idle_now, idle_time, fresh: true };
        Need::Running(Running { target, live, load, since_grant: since })
    }

    const LONG_AGO: Duration = Duration::from_secs(1);

    #[test]
    fn a_lone_ready_search_gets_every_free_thread() {
        assert_eq!(allot(8, &[Need::Ready]), [(0, Grant::Start(8))]);
        // A setup holds its thread, and a game that needs nothing changes nothing.
        assert_eq!(allot(8, &[Need::Nothing, Need::Ready, Need::SettingUp]), [(1, Grant::Start(7))]);
        // With nothing free, nothing starts.
        assert_eq!(allot(1, &[Need::Ready, Need::SettingUp]), []);
    }

    #[test]
    fn setups_get_one_thread_each_oldest_first() {
        let needs = [Need::SetUp, Need::SettingUp, Need::SetUp, Need::SetUp];
        assert_eq!(allot(3, &needs), [(0, Grant::SetUp), (2, Grant::SetUp)]);
        assert_eq!(allot(8, &needs), [(0, Grant::SetUp), (2, Grant::SetUp), (3, Grant::SetUp)]);
    }

    #[test]
    fn several_claims_are_served_oldest_first() {
        // Two ready searches and a setup: one thread each, in the order of the games.
        let needs = [Need::Ready, Need::SetUp, Need::Ready];
        assert_eq!(allot(8, &needs), [(0, Grant::Start(1)), (1, Grant::SetUp), (2, Grant::Start(1))]);
        assert_eq!(allot(2, &needs), [(0, Grant::Start(1)), (1, Grant::SetUp)]);
        // An older saturated search is served before a younger ready one, which then gets one thread, or none.
        let needs = [sampled(2, 2, 0.0, 0, LONG_AGO), Need::Ready];
        assert_eq!(allot(8, &needs), [(0, Grant::Workers(4)), (1, Grant::Start(1))]);
        assert_eq!(allot(4, &needs), [(0, Grant::Workers(4))]);
    }

    #[test]
    fn a_ready_search_does_not_take_every_thread_past_an_older_game_that_may_still_start() {
        for older in [Need::SetUp, Need::SettingUp, Need::Pending] {
            let grants = allot(8, &[older, Need::Ready]);
            assert_eq!(grants.last(), Some(&(1, Grant::Start(1))), "{older:?}");
        }
        // A younger game that may still start does not hold it back.
        assert_eq!(allot(8, &[Need::Ready, Need::Pending]), [(0, Grant::Start(8))]);
    }

    #[test]
    fn a_saturated_search_doubles() {
        // Busy over the last tick: as many more as it has live, as far as threads are free.
        assert_eq!(allot(16, &[sampled(4, 4, 0.01, 0, LONG_AGO)]), [(0, Grant::Workers(8))]);
        assert_eq!(allot(6, &[sampled(4, 4, 0.01, 0, LONG_AGO)]), [(0, Grant::Workers(6))]);
        // Not saturated, or not known, or a sample already acted on: no more.
        assert_eq!(allot(16, &[sampled(4, 4, 0.10, 1, LONG_AGO)]), []);
        let unknown = Need::Running(Running { target: 4, live: 4, load: Load::Unknown, since_grant: LONG_AGO });
        assert_eq!(allot(16, &[unknown]), []);
        let load = Load::Sampled { idle_share: 0.0, idle_now: 0, idle_time: Duration::ZERO, fresh: false };
        let used = Need::Running(Running { target: 4, live: 4, load, since_grant: LONG_AGO });
        assert_eq!(allot(16, &[used]), []);
        // A sample acted on is still a claim: a younger ready search gets one thread, not all.
        assert_eq!(allot(16, &[used, Need::Ready]), [(1, Grant::Start(1))]);
    }

    #[test]
    fn a_search_in_its_start_doubles_once_a_tick() {
        // A tick not acted on yet: it doubles the target it was given, as far as threads are free.
        assert_eq!(allot(16, &[starting(3, true, Duration::ZERO)]), [(0, Grant::Workers(6))]);
        assert_eq!(allot(5, &[starting(3, true, Duration::ZERO)]), [(0, Grant::Workers(5))]);
        // A tick acted on, or threads just given: it asks nothing, though it is still a claim, so that a younger ready
        // search gets one thread, not all.
        let used = starting(3, false, Duration::ZERO);
        assert_eq!(allot(16, &[used]), []);
        assert_eq!(allot(16, &[used, Need::Ready]), [(1, Grant::Start(1))]);
    }

    #[test]
    fn an_idle_search_releases_only_for_another_claim_after_settling_and_keeps_one() {
        // Half idle over the tick, 4 of 8 workers idle now: the idle time is 4 workers' worth of the tick.
        let idle = sampled(8, 8, 0.5, 4, LONG_AGO);
        // No other claim: it keeps them.
        assert_eq!(allot(8, &[idle]), []);
        // Another game's claim: it gives back 4 (which are free only once its engine has retired them).
        assert_eq!(allot(8, &[idle, Need::SetUp]), [(0, Grant::Workers(4))]);
        // Given threads less than SETTLE ago: it keeps them.
        let recent = sampled(8, 8, 0.5, 4, SETTLE / 2);
        assert_eq!(allot(8, &[recent, Need::SetUp]), []);
        // Idle less than IDLE of the tick, or with no worker idle now: it keeps them.
        assert_eq!(allot(8, &[sampled(8, 8, 0.2, 2, LONG_AGO), Need::SetUp]), []);
        assert_eq!(allot(8, &[sampled(8, 8, 0.5, 0, LONG_AGO), Need::SetUp]), []);
        // It gives back no more than its idle time amounts to: 2 workers' worth of the tick, though 6 are idle now.
        let load = Load::Sampled { idle_share: 0.25, idle_now: 6, idle_time: TICK * 2, fresh: true };
        let few = Need::Running(Running { target: 8, live: 8, load, since_grant: LONG_AGO });
        assert_eq!(allot(8, &[few, Need::SetUp]), [(0, Grant::Workers(6))]);
        // At least one worker remains.
        let all_idle = sampled(3, 3, 1.0, 3, LONG_AGO);
        assert_eq!(allot(8, &[all_idle, Need::Ready]), [(0, Grant::Workers(1)), (1, Grant::Start(5))]);
        let one = sampled(1, 1, 1.0, 1, LONG_AGO);
        assert_eq!(allot(1, &[one, Need::Ready]), []);
    }

    #[test]
    fn free_threads_count_the_larger_of_the_target_and_the_live_workers() {
        // Told to keep 2 but still 6 live (retiring as they come idle): 6 are held.
        let retiring = Need::Running(Running { target: 2, live: 6, load: Load::Unknown, since_grant: LONG_AGO });
        assert_eq!(
            allot(8, &[retiring, Need::SetUp, Need::SetUp, Need::SetUp]),
            [(1, Grant::SetUp), (2, Grant::SetUp)]
        );
        // Told to have 6 with 2 started so far: 6 are held.
        let growing = Need::Running(Running { target: 6, live: 2, load: Load::Unknown, since_grant: LONG_AGO });
        assert_eq!(allot(8, &[growing, Need::SetUp, Need::SetUp, Need::SetUp]), [(1, Grant::SetUp), (2, Grant::SetUp)]);
    }
}
