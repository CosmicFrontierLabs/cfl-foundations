//! Injectable timekeeping: wallclock and deterministic mock clocks.
//!
//! A simulation pipeline reads time through a [`Clock`] instead of calling
//! [`std::time::Instant::now`] / [`std::thread::sleep`] directly. In
//! production a [`SystemClock`] forwards to the wallclock; in a test or a
//! deterministic simulation a [`MockClock`] supplies *virtual* time that only
//! advances when the simulation says so.
//!
//! The two primitives a caller needs are:
//!
//! - [`Clock::now`] — the current time as a clock-relative [`Instant`].
//! - [`Clock::sleep`] — block the calling thread until that much clock time
//!   has elapsed.
//!
//! A [`MockClock`] starts at [`Instant::ORIGIN`] (time zero); use
//! [`MockClock::starting_at`] / [`MockClock::auto_advancing_starting_at`] to
//! start it at some other instant.
//!
//! Under a [`MockClock`], `sleep` does not burn wallclock time: it parks the
//! calling thread until the *simulation* catches up to the wake time. Time can
//! be driven forward two ways:
//!
//! - **Manual** ([`MockClock::advance`]) — a driver thread pushes time forward
//!   and parked sleepers wake when it passes their target.
//! - **Auto-advance** ([`MockClock::auto_advancing`]) — the clock jumps to the
//!   soonest pending wake on its own once **every live clone of the clock is
//!   parked in a `sleep`**. This makes a multi-threaded simulation fully
//!   deterministic: the outcome depends on the sleep durations, not on OS
//!   scheduling.
//!
//! Each clone of a `MockClock` is a *participant*: it counts toward the "is
//! everyone parked?" check until it is dropped. The idiom is to hand exactly
//! one clone to each thread that takes part in the simulated timeline and let
//! that clone drop when the thread finishes. The one thing to avoid is keeping
//! an idle clone on a thread that will *not* sleep (for example, holding a
//! clone in the orchestrator while it blocks in `join`): the clock would wait
//! forever for that clone to park. Move the clones into the worker threads and
//! read results back through their return values or shared state.

use std::collections::BTreeMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;
use std::time::Instant as StdInstant;

/// A point in time as measured by a [`Clock`].
///
/// Monotonic and clock-relative: the value is the [`Duration`] elapsed since
/// the clock's origin, not a wallclock or [`std::time::Instant`]. Using one
/// type for both real and mock clocks lets simulation code compare and
/// subtract instants without caring which clock produced them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Instant {
    since_origin: Duration,
}

impl Instant {
    /// The clock's origin — the instant a clock reports before any time has
    /// elapsed.
    pub const ORIGIN: Self = Self {
        since_origin: Duration::ZERO,
    };

    /// Time elapsed from the clock origin to this instant.
    pub const fn elapsed_since_origin(&self) -> Duration {
        self.since_origin
    }

    /// Duration from `earlier` to `self`, saturating at zero if `earlier` is
    /// actually later (never panics, unlike the `-` operator).
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.since_origin.saturating_sub(earlier.since_origin)
    }
}

impl std::ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, rhs: Duration) -> Instant {
        Instant {
            since_origin: self.since_origin + rhs,
        }
    }
}

impl std::ops::AddAssign<Duration> for Instant {
    fn add_assign(&mut self, rhs: Duration) {
        self.since_origin += rhs;
    }
}

impl std::ops::Sub<Instant> for Instant {
    type Output = Duration;

    /// Panics if `rhs` is later than `self`; use
    /// [`Instant::saturating_duration_since`] when that is possible.
    fn sub(self, rhs: Instant) -> Duration {
        self.since_origin - rhs.since_origin
    }
}

/// An injectable source of time.
///
/// Object-safe, so callers can hold an `Arc<dyn Clock>` and swap a
/// [`SystemClock`] for a [`MockClock`] without changing their code.
pub trait Clock: Send + Sync {
    /// The current time as a clock-relative [`Instant`].
    fn now(&self) -> Instant;

    /// Block the calling thread until `dur` of clock time has elapsed.
    ///
    /// A zero duration returns immediately.
    fn sleep(&self, dur: Duration);
}

impl<C: Clock + ?Sized> Clock for Arc<C> {
    fn now(&self) -> Instant {
        (**self).now()
    }

    fn sleep(&self, dur: Duration) {
        (**self).sleep(dur)
    }
}

/// A [`Clock`] backed by the operating system's monotonic clock.
///
/// [`now`](Clock::now) forwards to [`std::time::Instant::now`] (offset from the
/// instant the clock was created) and [`sleep`](Clock::sleep) forwards to
/// [`std::thread::sleep`].
#[derive(Clone, Copy, Debug)]
pub struct SystemClock {
    origin: StdInstant,
}

impl SystemClock {
    /// Create a clock whose origin is now.
    pub fn new() -> Self {
        Self {
            origin: StdInstant::now(),
        }
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant {
            since_origin: StdInstant::now().saturating_duration_since(self.origin),
        }
    }

    fn sleep(&self, dur: Duration) {
        std::thread::sleep(dur);
    }
}

/// Mutable, shared interior of a [`MockClock`].
struct State {
    /// Current simulated time, as a duration since the clock origin.
    now: Duration,
    /// When true, the clock advances itself to the soonest pending wake once
    /// every live clock handle is parked in a `sleep`.
    auto_advance: bool,
    /// Number of live [`MockClock`] clones sharing this timeline.
    handles: usize,
    /// Number of threads currently parked inside [`MockClock::sleep`].
    blocked: usize,
    /// Multiset of absolute wake targets of currently-parked sleepers, keyed
    /// by target with a count of waiters at that target. The soonest pending
    /// wake is the first key.
    pending: BTreeMap<Duration, usize>,
}

struct Shared {
    state: Mutex<State>,
    /// Notified whenever `now`, `handles`, or `blocked` changes in a way that
    /// could let a parked sleeper make progress.
    cvar: Condvar,
}

/// A [`Clock`] that supplies deterministic virtual time.
///
/// Cloning a `MockClock` yields another handle to the *same* clock, so all
/// clones share one timeline. In auto-advance mode each live clone also counts
/// as a participant — see the [module docs](self) for the manual vs.
/// auto-advance models and the one rule about idle clones.
pub struct MockClock {
    shared: Arc<Shared>,
}

impl MockClock {
    /// A manual mock clock starting at [`Instant::ORIGIN`]. Time only moves
    /// when [`advance`](MockClock::advance) / [`advance_to`](MockClock::advance_to)
    /// is called.
    pub fn new() -> Self {
        Self::build(false, Duration::ZERO)
    }

    /// A manual mock clock starting at `start` instead of [`Instant::ORIGIN`].
    pub fn starting_at(start: Instant) -> Self {
        Self::build(false, start.since_origin)
    }

    /// An auto-advancing mock clock starting at [`Instant::ORIGIN`].
    ///
    /// Once every live clone of the clock is parked in a [`sleep`](Clock::sleep),
    /// the clock jumps forward to the soonest pending wake without any
    /// wallclock delay. Hand one clone to each participating thread; do not
    /// retain an idle clone on a thread that will not sleep, or the clock will
    /// wait forever for it to park.
    pub fn auto_advancing() -> Self {
        Self::build(true, Duration::ZERO)
    }

    /// An auto-advancing mock clock (see [`auto_advancing`]) starting at
    /// `start` instead of [`Instant::ORIGIN`].
    ///
    /// [`auto_advancing`]: MockClock::auto_advancing
    pub fn auto_advancing_starting_at(start: Instant) -> Self {
        Self::build(true, start.since_origin)
    }

    fn build(auto_advance: bool, start: Duration) -> Self {
        Self {
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    now: start,
                    auto_advance,
                    handles: 1,
                    blocked: 0,
                    pending: BTreeMap::new(),
                }),
                cvar: Condvar::new(),
            }),
        }
    }

    /// Push simulated time forward by `dur`, waking any sleepers whose target
    /// is now in the past.
    pub fn advance(&self, dur: Duration) {
        let mut state = self.shared.state.lock().unwrap();
        state.now += dur;
        self.shared.cvar.notify_all();
    }

    /// Move simulated time to `target`. Time is monotonic, so a `target` in the
    /// past is ignored.
    pub fn advance_to(&self, target: Instant) {
        let mut state = self.shared.state.lock().unwrap();
        if target.since_origin > state.now {
            state.now = target.since_origin;
            self.shared.cvar.notify_all();
        }
    }
}

impl Default for MockClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for MockClock {
    fn clone(&self) -> Self {
        self.shared.state.lock().unwrap().handles += 1;
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl Drop for MockClock {
    fn drop(&mut self) {
        // Tolerate a poisoned lock so a panicking participant doesn't turn into
        // a double-panic/abort that hides the original failure.
        let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        state.handles -= 1;
        // One fewer participant may leave the remaining clones all parked, so
        // let them re-check the auto-advance condition.
        self.shared.cvar.notify_all();
    }
}

impl Clock for MockClock {
    fn now(&self) -> Instant {
        let state = self.shared.state.lock().unwrap();
        Instant {
            since_origin: state.now,
        }
    }

    fn sleep(&self, dur: Duration) {
        if dur.is_zero() {
            return;
        }

        let mut state = self.shared.state.lock().unwrap();
        let target = state.now + dur;
        *state.pending.entry(target).or_insert(0) += 1;
        state.blocked += 1;

        loop {
            if state.now >= target {
                // Wake: consume our slot and leave.
                if let Some(count) = state.pending.get_mut(&target) {
                    *count -= 1;
                    if *count == 0 {
                        state.pending.remove(&target);
                    }
                }
                state.blocked -= 1;
                return;
            }

            // Auto-advance only when the whole simulation has gone quiet: every
            // live clone is parked, and the soonest wake is strictly in the
            // future (so no already-runnable sleeper is starved by jumping past
            // it).
            if state.auto_advance && state.blocked == state.handles {
                let next = *state
                    .pending
                    .keys()
                    .next()
                    .expect("a parked sleeper guarantees a pending wake");
                if next > state.now {
                    state.now = next;
                    self.shared.cvar.notify_all();
                    continue;
                }
            }

            state = self.shared.cvar.wait(state).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Clock, Instant, MockClock, SystemClock};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn instant_arithmetic() {
        let t0 = Instant::ORIGIN;
        let t1 = t0 + ms(250);
        assert_eq!(t1 - t0, ms(250));
        assert_eq!(t1.elapsed_since_origin(), ms(250));
        assert!(t1 > t0);
        // saturating_duration_since never panics on a backwards delta.
        assert_eq!(t0.saturating_duration_since(t1), Duration::ZERO);
    }

    #[test]
    fn system_clock_is_monotonic() {
        let clock = SystemClock::new();
        let a = clock.now();
        clock.sleep(ms(1));
        let b = clock.now();
        assert!(b >= a);
    }

    #[test]
    fn mock_now_starts_at_origin_and_advances_manually() {
        let clock = MockClock::new();
        assert_eq!(clock.now(), Instant::ORIGIN);
        clock.advance(ms(40));
        assert_eq!(clock.now(), Instant::ORIGIN + ms(40));
        // advance_to is monotonic: a past target is ignored.
        clock.advance_to(Instant::ORIGIN + ms(10));
        assert_eq!(clock.now(), Instant::ORIGIN + ms(40));
        clock.advance_to(Instant::ORIGIN + ms(100));
        assert_eq!(clock.now(), Instant::ORIGIN + ms(100));
    }

    #[test]
    fn mock_can_start_at_a_nonzero_time() {
        let start = Instant::ORIGIN + ms(500);
        let clock = MockClock::starting_at(start);
        assert_eq!(clock.now(), start);
        clock.advance(ms(20));
        assert_eq!(clock.now(), start + ms(20));

        // Auto-advancing variant honors the start too. The single handle held
        // by this thread is the only participant.
        let auto = MockClock::auto_advancing_starting_at(start);
        assert_eq!(auto.now(), start);
        auto.sleep(ms(10));
        assert_eq!(auto.now(), start + ms(10));
    }

    #[test]
    fn mock_zero_sleep_returns_immediately() {
        let clock = MockClock::new();
        // No driver thread, manual clock: a zero sleep must not block.
        clock.sleep(Duration::ZERO);
        assert_eq!(clock.now(), Instant::ORIGIN);
    }

    #[test]
    fn mock_sleep_blocks_until_driver_catches_up() {
        let clock = MockClock::new();
        let worker = clock.clone();
        let handle = thread::spawn(move || {
            worker.sleep(ms(50));
            worker.now()
        });

        // We can't know when the worker parks, so keep nudging time forward
        // (yielding between) until it finishes. This terminates because each
        // advance outpaces the relative sleep target once the worker is parked.
        let observed = loop {
            clock.advance(ms(5));
            thread::yield_now();
            if handle.is_finished() {
                break handle.join().unwrap();
            }
        };

        // It only woke after at least its 50ms target elapsed.
        assert!(observed >= Instant::ORIGIN + ms(50));
    }

    #[test]
    fn mock_auto_advance_single_thread() {
        // This thread holds the only clone, so it is the only participant: it
        // parks, the clock advances to its target, it wakes — no extra setup.
        let clock = MockClock::auto_advancing();

        let mut observed = Vec::new();
        for _ in 0..3 {
            clock.sleep(ms(5));
            observed.push(clock.now());
        }

        assert_eq!(
            observed,
            vec![
                Instant::ORIGIN + ms(5),
                Instant::ORIGIN + ms(10),
                Instant::ORIGIN + ms(15),
            ]
        );
    }

    #[test]
    fn mock_auto_advance_multirate_exposure_and_guidance() {
        // Mirrors the real use case: one thread holds a long stabilized
        // exposure open while faster threads (a gyro ticking every 2ms, a
        // monocle guidance loop at 8ms) run many cycles inside that window.
        // Auto-advance steps through every wake in time order, so the cycle
        // counts are a function of the rates alone, not OS scheduling.
        //
        // Each clone is moved into its worker thread; this thread keeps no
        // clone, so it is not counted as a never-sleeping participant.
        let clock = MockClock::auto_advancing();
        let clock_gyro = clock.clone();
        let clock_monocle = clock.clone();

        let deadline = Instant::ORIGIN + ms(100);
        let gyro_ticks = Arc::new(Mutex::new(0usize));
        // Stands in for the FSM position the guidance loop keeps updating; the
        // exposure integrates against whatever has accumulated by its end.
        let fsm_updates = Arc::new(Mutex::new(0usize));

        let exposure_handle = thread::spawn(move || {
            clock.sleep(ms(100));
            clock.now()
        });
        let gyro_handle = {
            let ticks = Arc::clone(&gyro_ticks);
            thread::spawn(move || {
                while clock_gyro.now() < deadline {
                    clock_gyro.sleep(ms(2));
                    *ticks.lock().unwrap() += 1;
                }
            })
        };
        let monocle_handle = {
            let updates = Arc::clone(&fsm_updates);
            thread::spawn(move || {
                while clock_monocle.now() < deadline {
                    clock_monocle.sleep(ms(8));
                    *updates.lock().unwrap() += 1;
                }
            })
        };

        let exposure_end = exposure_handle.join().unwrap();
        gyro_handle.join().unwrap();
        monocle_handle.join().unwrap();

        // The exposure ran exactly its commanded length.
        assert_eq!(exposure_end, deadline);
        // Gyro: loop tops at 0,2,..,98 (<100) → 50 ticks, ending right at 100.
        assert_eq!(*gyro_ticks.lock().unwrap(), 50);
        // Monocle: loop tops at 0,8,..,96 (<100) → 13 updates (the last sleeps
        // just past the deadline).
        assert_eq!(*fsm_updates.lock().unwrap(), 13);
    }

    #[test]
    fn mock_auto_advance_interleaves_two_threads_deterministically() {
        let clock = MockClock::auto_advancing();
        let clock_b = clock.clone();

        let log_a = Arc::new(Mutex::new(Vec::new()));
        let log_b = Arc::new(Mutex::new(Vec::new()));

        let a = {
            let log = Arc::clone(&log_a);
            // Move `clock` in; this thread owns that participant.
            thread::spawn(move || {
                for _ in 0..3 {
                    clock.sleep(ms(10));
                    log.lock().unwrap().push(clock.now());
                }
            })
        };
        let b = {
            let log = Arc::clone(&log_b);
            thread::spawn(move || {
                for _ in 0..2 {
                    clock_b.sleep(ms(25));
                    log.lock().unwrap().push(clock_b.now());
                }
            })
        };

        a.join().unwrap();
        b.join().unwrap();

        // Auto-advance jumps to the soonest pending wake when both threads are
        // parked, so the per-thread wake times are independent of scheduling.
        let o = |n| Instant::ORIGIN + ms(n);
        assert_eq!(*log_a.lock().unwrap(), vec![o(10), o(20), o(30)]);
        assert_eq!(*log_b.lock().unwrap(), vec![o(25), o(50)]);
    }
}
