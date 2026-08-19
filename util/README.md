# util

Small, zero-dependency helpers for frame/delta-time based game loops.
Currently: a single accumulate-and-fire timer, exposed as both a plain
function and a macro.

## `timed_op`

```rust
fn timed_op(timer: f64, delta: f64, limit: f64) -> (f64, bool)
```

Accumulates `delta` onto `timer`. If the total reaches `limit`, returns
`(0.0, true)` (fired, reset); otherwise returns `(timer + delta, false)`.
Pure and stateless — the caller owns the timer value and threads it through
each call.

## `throttle!`

```rust
throttle!(delta, limit) -> bool
```

Wraps `timed_op` with a `static mut` timer scoped to the macro's call site,
so you don't have to store the timer yourself. Evaluates to `true` once
every `limit` seconds of accumulated `delta`, `false` otherwise:

```rust
use mh_util::throttle;

fn process(&mut self, delta: f64) {
    if !throttle!(delta, 0.25) {
        return;
    }
    // runs at most 4 times per second, however often `process` is called
}
```

**The timer belongs to the call site, not the caller.** Each place
`throttle!(...)` appears in the source gets its own independent, persistent
timer — calling it from two different functions gives two independent
timers, but calling it once from a function invoked for many different
entities means all of those entities share the *same* timer. Don't use it
to rate-limit per-instance from inside a generic/shared function; use
`timed_op` with a timer stored on the instance instead.

## Testing

```
cargo test -p util
```

`tests/timed_op.rs` covers the accumulator directly; `tests/throttle.rs`
exercises the macro's per-call-site behavior (deliberately kept to a single
`#[test]` function, since cargo runs tests in parallel and the macro's
`static mut` timers are shared across the whole test binary).
