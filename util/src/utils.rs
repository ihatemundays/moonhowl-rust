pub fn timed_op(timer: f64, delta: f64, limit: f64) -> (f64, bool) {
    let timer = timer + delta;

    if timer >= limit {
        return (0.0, true);
    }

    (timer, false)
}

/// Evaluates to `true` once every `limit` seconds of accumulated `delta`, `false` otherwise.
/// Each call site gets its own timer, so it's safe to invoke from multiple places.
#[macro_export]
macro_rules! throttle {
    ($delta:expr, $limit:expr) => {{
        static mut TIMER: f64 = 0.0;
        let go_on;
        unsafe {
            (TIMER, go_on) = $crate::timed_op(TIMER, $delta, $limit);
        }
        go_on
    }};
}
