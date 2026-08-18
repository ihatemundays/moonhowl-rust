use util::timed_op;

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
