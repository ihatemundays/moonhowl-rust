# util-godot

The one Godot-coupled crate in this workspace (see the root README's
[Design](../README.md#design) section) — it depends on the real
[`godot`](https://crates.io/crates/godot) (gdext) crate so its behavior can
be checked against Godot's actual notification constants, not guessed ones.
`ecs` and `util` stay dependency-free; this crate is the deliberate
exception.

## The problem

`ecs::Entity` is a cheap, `Copy` id with no owner and no `Drop` — that's what
makes it safe to store on a Godot node. But it also means nothing despawns
an entity automatically when the node holding its id goes away. If a node is
freed and nothing calls `world.despawn(entity)`, that entity and its
components stay alive in the `World` forever: not a Rust memory-safety leak
(everything's still owned and would free with the `World`), but an unbounded
logical one for the life of the process, since despawned-but-never-freed
slots never return to `World`'s free list for reuse.

## `DespawnGuard`

```rust
use ecs::World;
use util_godot::DespawnGuard;

let mut world = World::new();
let entity = world.spawn();
let mut guard = DespawnGuard::new(entity);

// From an `on_notification` override (or equivalent), on every notification:
guard.handle(&mut world, /* whatever notification value you received */ 1);
```

`handle` despawns `entity` from `world` the first time it sees Godot's
`PREDELETE` notification — the one Godot guarantees fires exactly once,
right before an object is actually freed (unlike `EXIT_TREE`, which can fire
more than once if a node is removed and re-added to the tree). Every other
notification is ignored, and any call after the first `PREDELETE` is a
no-op — safe to call unconditionally from every `on_notification`
invocation without writing that guard yourself.

`handle` is generic over `Into<i32>`, so it accepts whichever
class-specific notification enum your `IObject`-derived override actually
receives (`NodeNotification`, `ObjectNotification`, etc. — gdext generates
one per class, all convertible to the underlying `i32`), rather than forcing
every caller to convert first.

Wiring it into a real class:

```rust
use ecs::World;
use godot::classes::{INode, Node, notify::NodeNotification};
use godot::prelude::*;
use util_godot::DespawnGuard;

#[derive(GodotClass)]
#[class(base = Node, no_init)]
struct Actor {
    base: Base<Node>,
    despawn_guard: DespawnGuard,
}

#[godot_api]
impl INode for Actor {
    fn on_notification(&mut self, what: NodeNotification) {
        self.despawn_guard.handle(&mut my_world(), what);
    }
}
```

(`my_world()` stands in for however your game accesses its `World` — a
singleton, a resource, whatever fits; that plumbing is the game's concern,
not this crate's.)

## Why `PREDELETE`, and why this is testable without a running Godot engine

`PREDELETE`, `READY`, `EXIT_TREE`, etc. are plain values gdext generates at
*build time* from a bundled Godot API description — reading them, or
constructing `NodeNotification`/`ObjectNotification` values, needs no live
engine. `examples/actor.rs` and `tests/despawn_guard.rs` both compile and
run under plain `cargo test`/`cargo build --examples`, and the tests assert
`DespawnGuard` reacts correctly to the *real* `ObjectNotification::PREDELETE`
/ `NodeNotification::PREDELETE` constants gdext generated for the pinned
`godot` version — not a magic number standing in for them — so a future
gdext upgrade that renumbers or renames these would fail loudly here rather
than silently misfiring in the game. What can't be verified without an
actual running engine is that Godot really calls `on_notification` when it
says it will; that's gdext's own contract, not something this crate
re-proves.

`tests/despawn_guard.rs` also covers the reason `DespawnGuard` tracks its
own `despawned` flag rather than trusting Godot to only ever call it once:
if a stale/duplicate `PREDELETE` arrives after the entity's slot has already
been despawned and reused by a new entity (see the root `ecs` README's notes
on generational ids), the guard must not touch the new occupant.

## Testing

```
cargo test -p util-godot
```
