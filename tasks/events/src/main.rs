//! Rust uses condition variables (Condvars) for asynchronous event processing.  Each Mutex has a
//! list of zero or more Condvars, which are essentially events that the task may wait on or signal
//! when it holds the mutex.  When a task begins to wait on a condvar, it enters a waiting state
//! until the condvar is signaled by another task, at which point it resumes processing.  Thus,
//! each condvar represents an event on which a task may wait.  The one subtlety is that condvar
//! signals are only received if there is actually a task waiting on the signal--see the below
//! program for an example of how this may be achieved in practice.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, spawn};
use std::time::{Duration, Instant};

/// Given a duration to wait before sending an event from one process to another, returns the
/// elapsed time before the event was actually sent.
#[cfg_attr(feature = "cargo-clippy", allow(mutex_atomic))]
fn handle_event(duration: Duration) -> Duration {
    // Create a Mutex.  By default a Mutex is created with a single condition variable (condvar_id
    // 0) but it can be created with an arbitrary number using Mutex::new_with_condvars();
    let pair = Arc::new((Mutex::new(false), Condvar::new()));
    let pair_ = Arc::clone(&pair);
    let start = Instant::now();
    // Lock the mutex
    let &(ref mutex, ref cond) = &*pair;
    let mut guard = mutex.lock().unwrap();
    // Start our secondary task (which will signal our waiting main task)
    spawn(move || {
        let &(ref mutex_, ref cond_) = &*pair_;
        // Lock the mutex
        let mut guard = mutex_.lock().unwrap();
        *guard = true;

        // Sleep for `duration`.
        thread::sleep(duration);
        // Signal the waiting mutex (equivalent to guard.cond.signal_on(0)).
        // One can also signal to all tasks on the waiting mutex with broadcast (broadcast_on(0)).
        //
        // Note that if no tasks are actually waiting yet (which is possible, if we got the lock
        // before the other task), then this might report that it failed to wake up any tasks.
        // That is why the mutex was locked before the task was spawned--we know we cannot possibly
        // get past the mutex at the top of the task until the wait() statement below is reached.
        cond_.notify_one();
        // Although we signaled the waiting mutex, it will not awaken until this guard is dropped.
    });
    // Wait for the event state to be set to signaled (equivalent to guard.cond.wait_on(0)).
    while !*guard {
        guard = cond.wait(guard).unwrap();
    }
    // Should be done signaling (i.e. we've waited for `duration`).
    let elapsed = start.elapsed();
    // When the guard exits scope, the condvar is reset.
    drop(guard);
    // Return the elapsed time
    elapsed
}

pub fn main() {
    let duration = Duration::from_secs(1); // Process event after one second.
    println!(
        "{} seconds elapsed before event triggered",
        handle_event(duration).as_secs()
    );
}

#[test]
pub fn test_events() {
    // Process event after one tenth of a second.
    let duration = Duration::from_secs(1) / 10;

    // Make sure it really did take at least that long for the event to be processed.
    let out = handle_event(duration);
    assert!(duration <= out);
}
