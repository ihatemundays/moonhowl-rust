# moonhowl-rust

A small, zero-dependency Rust framework: reusable building blocks pulled out
of the *What Lurks Below* Godot game so they can be developed, tested, and
versioned on their own. Nothing in this workspace depends on Godot or
`godot-rust` — it's plain Rust, consumed as a git dependency by the game.

## Crates

| Crate | Description |
|---|---|
| [`ecs`](ecs/README.md) | A small sparse-set ECS: components live in per-type stores on a `World`, entities are cheap generational ids, with type-safe multi-component access ("archetypes") both per-entity and world-wide. No scheduler — the caller decides when and how queries run. |
| [`util`](util/README.md) | Small delta-time helpers for game loops — an accumulate-and-fire timer, as a function and a macro. |

See each crate's own README for the full API and examples.

## Using it

Both crates are consumed as git dependencies, typically renamed to an
`mh_` prefix at the call site:

```toml
[dependencies]
mh_ecs = { git = "https://github.com/ihatemundays/moonhowl-rust", branch = "master", package = "ecs" }
mh_util = { git = "https://github.com/ihatemundays/moonhowl-rust", branch = "master", package = "util" }
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

    world.edit_component::<Health>(entity, |health| {
        if health.hp > 0 {
            health.hp -= 1;
        }
    });
}
```

## Design

- **Zero dependencies.** Both crates build on `std` alone.
- **Small surface area.** Each crate does one thing — a component store, a
  timer — rather than growing into a general-purpose engine.
- **No hidden control flow.** `ecs` has no scheduler; the caller decides when
  and how queries and entity updates run (in the game, each Godot node holds
  its own `Entity` and drives it from `process`/`physics_process`).

## Testing

```
cargo test --workspace
```

Each crate can also be tested individually: `cargo test -p ecs`,
`cargo test -p util`.
