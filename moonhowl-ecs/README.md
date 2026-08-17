# moonhowl-ecs

A minimal, from-scratch entity-component-system.

## Concepts

- **Entity** — an object tagged with a marker type `M` (any `'static` type, usually a zero-sized struct). Entities of the same marker type are grouped together and share the same set of systems. An entity holds:
  - any number of typed **components** (types implementing `IComponent`)
  - a single opaque **context** value (any `Any + Send` type)
- **World** — owns every entity (grouped by marker type) and every registered system.
- **System** (`ISystem`) — registered against a marker type, runs in two phases per entity:
  1. `check(&CheckContext, &Entity) -> bool` — read-only, decides whether the system applies.
  2. `and_then(&ActionContext, &Entity)` — runs only if `check` passed; acts on the entity.

```rust
use moonhowl_ecs::{ActionContext, CheckContext, Entity, IComponent, ISystem, World};
use std::any::Any;

struct Position { x: f32, y: f32 }
impl IComponent for Position {
    fn as_any(&self) -> &dyn Any { self }
}

struct Velocity { dx: f32, dy: f32 }
impl IComponent for Velocity {
    fn as_any(&self) -> &dyn Any { self }
}

struct MovingThing;

struct ApplyVelocity;
impl ISystem for ApplyVelocity {
    fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
        system.has_every_component::<(Position, Velocity)>(entity)
    }

    fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
        let (position, velocity) = system.get_components::<(Position, Velocity)>(entity).unwrap();
        system.set_component(entity, Position {
            x: position.x + velocity.dx,
            y: position.y + velocity.dy,
        });
    }
}

fn main() {
    let mut world = World::new();

    let id = world.spawn::<MovingThing>();
    world.get_entity_mut::<MovingThing>(id).unwrap()
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.5 });

    world.register_system::<MovingThing, _>(ApplyVelocity);

    world.run_sync();
    world.confirm();
}
```

## Components and context

- `get_component<T>` / `get_components<T>` — read, where `T` for the tuple form is any tuple of up to 8 `IComponent` types (`get_components::<(A, B)>()`).
- `set_component<T>` / `unset_component<T>` on `Entity` — immediate, for use outside a run (setup, teardown, direct edits via `World::get_entity_mut`).
- `set_context<C>` / `get_context<C>` / `get_context_mut<C>` / `clear_context` on `Entity` — same, immediate, for the single opaque context slot.

## Read tracking

Each component tracks, per system, whether that system has called `read_component`/`read_components` on it (via `ActionContext`) before. `CheckContext` exposes this as `is_component_read` / `has_read_component` / `has_unread_component`, plus tuple forms (`has_some_read_components`, `has_every_read_component`, `has_some_unread_components`, `has_every_unread_component`). This lets a system check for entities carrying data it hasn't processed yet, rather than re-running on everything every pass. Setting a component (immediately or via a queued `set_component`) resets its read tracking.

## Running systems

`World` has five ways to run all registered systems over all entities:

- **`run_sync()`** — everything runs on the calling thread.
- **`run()`** — one thread per marker type; both `check` and `and_then` run on that thread.
- **`run_chunked()`** — like `run`, but splits each marker type's entities into `std::thread::available_parallelism()` chunks and runs one thread per chunk, so a single marker type with many entities still uses every core instead of leaving them idle.
- **`run_checked_sync()`** — `check` runs one thread per marker type (like `run`), but every `and_then` call happens afterward on the calling thread. Use this when `and_then` needs to run on a specific thread (e.g. the main thread, for anything not `Sync`) while still parallelizing the read-only `check` phase.
- **`run_max_parallel()`** — one OS thread per *entity*. Included for completeness, not recommended: `thread::scope`'s `spawn` is a real OS thread (tens of microseconds to create and join), not a green thread, so this is dominated by thread-spawn overhead outside of tiny entity counts. `examples/bench_parallelism.rs` measures this concretely — at 1,000 entities it's roughly 180x slower than `run_sync` on the machine it was measured on. `run_chunked` gets the same "use every core" benefit without the oversubscription cost.

All five execute checks and actions in the same order: for a given entity, systems run in **registration order** (the order `register_system` was first called for that marker type — re-registering an existing system keeps its original slot).

Run `cargo run --release --example bench_parallelism` to compare all five on your own machine and entity counts.

## Queued mutations

`ActionContext` (available inside `and_then`) never mutates anything immediately — every mutation is queued and only takes effect once `World::confirm()` is called:

- `set_component<T>` / `unset_component<T>`
- `set_context<C>` / `clear_context`
- `despawn(entity)` — the entity stays visible to other systems for the rest of the current run; only removed on `confirm()`.
- `spawn::<M>(|entity| { ... })` — builds a new `M`-tagged entity and queues it for insertion; returns the id it will be inserted under immediately, even though the entity itself isn't visible (e.g. via `World::get_entity`) until `confirm()`.

This is what makes `run()`/`run_checked_sync()` safe to call from any thread: nothing about the `World`'s entity set changes mid-run. Call `world.confirm()` after a run to apply everything that was queued. Queued operations on the same entity apply in the order the systems touching it ran, i.e. system registration order — so two systems queuing conflicting writes to the same component resolve deterministically based on registration order, not call timing.

## World CRUD

`spawn::<M>`, `despawn::<M>`, `despawn_all`, `contains::<M>`, `get_entity::<M>`, `get_entity_mut::<M>`, `len`, `is_empty`, `iter::<M>` for entities; `register_system::<M, S>`, `deregister_system::<M, S>`, `deregister_all_systems` for systems; `reset` clears both.
