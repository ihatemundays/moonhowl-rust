use util::throttle;

fn tick_a(delta: f64) -> bool {
    throttle!(delta, 1.0)
}

fn tick_b(delta: f64) -> bool {
    throttle!(delta, 1.0)
}

// A single test function on purpose: `throttle!`'s timer is a `static mut` tied to its
// source location, so it persists for the whole test-binary process. Splitting these
// assertions across multiple #[test] fns would let cargo's parallel test threads race
// on the same static and contaminate each other's timers.
#[test]
fn throttle_accumulates_fires_resets_and_is_independent_per_call_site() {
    assert!(!tick_a(0.4));
    assert!(!tick_a(0.4));
    assert!(tick_a(0.4));

    assert!(!tick_a(0.5));
    assert!(tick_a(0.5));

    assert!(!tick_b(0.9));
    assert!(!tick_a(0.9));
    assert!(tick_a(0.2));
    assert!(!tick_b(0.09));
    assert!(tick_b(0.02));
}
