# moonhowl-rust

A small Rust framework: reusable building blocks pulled out of the *What
Lurks Below* Godot game so they can be developed, tested, and versioned on
their own. The whole workspace is plain Rust with zero dependencies,
consumed as a git dependency by the game.

## Crates

| Crate | Description |
|---|---|
| [`ecs`](ecs/README.md) | A minimal per-entity ECS: each `Entity` owns its own components directly behind a deferred command queue (`set_component`/`unset_component` + `commit()`), with type-safe multi-component reads ("archetypes") and a `System` trait for querying which entities currently match a condition. No scheduler, no `World` — the caller decides when and how commits and queries run. |
| [`util`](util/README.md) | Small delta-time helpers for game loops — an accumulate-and-fire timer, as a function and a macro. |

See each crate's own README for the full API and examples.

## Using it

Crates are consumed as git dependencies, typically renamed to an `mh_`
prefix at the call site:

```toml
[dependencies]
mh_ecs = { git = "https://github.com/ihatemundays/moonhowl-rust", branch = "master", package = "ecs" }
mh_util = { git = "https://github.com/ihatemundays/moonhowl-rust", branch = "master", package = "util" }
```

```rust
use mh_ecs::{Component, Entity};
use mh_util::throttle;

struct Health(u8);
impl Component for Health {}

fn process(entity: &mut Entity, delta: f64) {
    if !throttle!(delta, 0.25) {
        return;
    }

    if let Some(health) = entity.with_archetype::<Health>() {
        let hp = health.0.saturating_sub(1);
        entity.set_component(Health(hp)).commit();
    }
}
```

## Design

- **Zero dependencies.** Every crate here builds on `std` alone — nothing
  engine-specific or third-party.
- **Small surface area.** Each crate does one thing — a component store, a
  timer — rather than growing into a general-purpose engine.
- **No hidden control flow.** `ecs` has no scheduler and no `World`; the
  caller decides when and how commits and queries run (in the game, each
  Godot node holds its own `Entity` and drives it from
  `process`/`physics_process`).

## Testing

```
cargo test --workspace
```

Each crate can also be tested individually: `cargo test -p ecs`,
`cargo test -p util`.
