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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accumulates_below_limit_without_firing() {
        let (timer, fired) = timed_op(0.0, 0.4, 1.0);

        assert_eq!(timer, 0.4);
        assert!(!fired);
    }

    #[test]
    fn fires_and_resets_when_delta_reaches_limit_exactly() {
        let (timer, fired) = timed_op(0.6, 0.4, 1.0);

        assert_eq!(timer, 0.0);
        assert!(fired);
    }

    #[test]
    fn fires_and_resets_when_delta_exceeds_limit() {
        let (timer, fired) = timed_op(0.6, 0.5, 1.0);

        assert_eq!(timer, 0.0);
        assert!(fired);
    }

    #[test]
    fn fires_immediately_when_a_single_delta_exceeds_the_limit() {
        let (timer, fired) = timed_op(0.0, 2.0, 1.0);

        assert_eq!(timer, 0.0);
        assert!(fired);
    }

    #[test]
    fn zero_delta_never_fires() {
        let (timer, fired) = timed_op(0.5, 0.0, 1.0);

        assert_eq!(timer, 0.5);
        assert!(!fired);
    }
}
