# moonhowl-rust

A small Rust framework: reusable building blocks pulled out of the *What
Lurks Below* Godot game so they can be developed, tested, and versioned on
their own. Most of this workspace is plain Rust with zero dependencies,
consumed as a git dependency by the game — see [Design](#design) for the one
deliberate exception.

## Crates

| Crate | Description |
|---|---|
| [`ecs`](ecs/README.md) | A small sparse-set ECS: components live in per-type stores on a `World`, entities are cheap generational ids, with type-safe multi-component access ("archetypes") both per-entity and world-wide. No scheduler — the caller decides when and how queries run. |
| [`util`](util/README.md) | Small delta-time helpers for game loops — an accumulate-and-fire timer, as a function and a macro. |
| [`util-godot`](util-godot/README.md) | Depends on the real `godot` (gdext) crate — the workspace's one intentional exception. Currently: `DespawnGuard`, which despawns an `ecs::Entity` exactly once on Godot's `PREDELETE` notification, so a freed node can't leak its entity in the `World`. |

See each crate's own README for the full API and examples.

## Using it

All crates are consumed as git dependencies, typically renamed to an
`mh_` prefix at the call site:

```toml
[dependencies]
mh_ecs = { git = "https://github.com/ihatemundays/moonhowl-rust", branch = "master", package = "ecs" }
mh_util = { git = "https://github.com/ihatemundays/moonhowl-rust", branch = "master", package = "util" }
mh_util_godot = { git = "https://github.com/ihatemundays/moonhowl-rust", branch = "master", package = "util-godot" }
```

```rust
use mh_ecs::{Component, Entity, World};
use mh_util::throttle;

struct Health { hp: u8 }
impl Component for Health {}

fn process(world: &mut World, entity: Entity, delta: f64) {
    if !throttle!(delta, 0.25) {
        return;
    }

    if let Some(health) = world.get_mut::<Health>(entity) {
        if health.hp > 0 {
            health.hp -= 1;
        }
    }
}
```

## Design

- **Zero dependencies, with one deliberate exception.** `ecs` and `util`
  build on `std` alone. `util-godot` depends on the real `godot` crate — not
  out of convenience, but because its whole purpose is to react to Godot's
  actual notification constants, and those are only meaningful checked
  against the real thing (see its README for why that still doesn't require
  a running Godot engine to test). Everything else in this workspace stays
  engine-agnostic.
- **Small surface area.** Each crate does one thing — a component store, a
  timer, a despawn hook — rather than growing into a general-purpose engine.
- **No hidden control flow.** `ecs` has no scheduler; the caller decides when
  and how queries and entity updates run (in the game, each Godot node holds
  its own `Entity` and drives it from `process`/`physics_process`).

## Testing

```
cargo test --workspace
```

Each crate can also be tested individually: `cargo test -p ecs`,
`cargo test -p util`, `cargo test -p util-godot`.
