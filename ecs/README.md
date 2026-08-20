# ecs

A small, zero-dependency sparse-set ECS. Components live in per-type stores
on a `World`; entities are cheap generational ids, not containers. Both
whole-`World` queries and single-entity lookups are first-class — there's no
scheduler and no query builder, just typed access to data the caller decides
when and how to touch.

## Concepts

- **Component** — any `'static + Send + Sync` type. Implement the marker
  trait `Component` (`impl Component for T {}`, no methods) to make it
  storable.
- **Entity** — a `Copy` id: `{ index, generation }`. It carries no data of
  its own; despawning bumps the slot's generation so a stale `Entity` you're
  still holding fails lookups safely instead of resolving to whatever
  reoccupies that index later. Cheap enough to store anywhere — a Godot node
  can hold one as a plain field.
- **World** — owns entity allocation (`spawn`/`despawn`) and one
  `SparseSet<T>` per component type. All component reads/writes go through
  `World`, keyed by `Entity`.
- **SparseSet\<T\>** — the storage underneath each component type: O(1)
  insert/remove/get, densely packed for iteration. Exported directly in case
  you want the same structure for your own entity-keyed data.
- **Archetype** — a single `Component` type, or a tuple of 2–6 of them.
  `World::get`/`get_mut` fetch an archetype for one entity; `with_archetype*`
  fetch it for every matching entity in the `World`.

```rust
use ecs::{Component, World};

struct Position { x: f32, y: f32 }
impl Component for Position {}

struct Velocity { dx: f32, dy: f32 }
impl Component for Velocity {}

fn main() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 0.0, y: 0.0 })
        .set_component(entity, Velocity { dx: 1.0, dy: 0.5 });

    // Every entity with both components. The second closure parameter is the
    // Entity being visited, in case you need it (e.g. to look up another,
    // optional component, or just for logging/correlation).
    world.with_archetype_mut::<(Position, Velocity)>(|(pos, vel), _entity| {
        pos.x += vel.dx;
        pos.y += vel.dy;
    });

    // One entity, looked up directly.
    let moved = world.get::<Position>(entity).map(|pos| (pos.x, pos.y));
    assert_eq!(moved, Some((1.0, 0.5)));
}
```

See `examples/simulation.rs` for a small multi-tick sim (movement, per-entity
damage, despawn-on-death), `examples/parallel.rs` for the parallel query
variants, and `examples/deferred_mutation.rs` for the collect-then-apply
pattern needed to add/remove components or despawn based on what a query
sees (its closures never hand back `&mut World`).

## API

**Entities**
- `World::spawn() -> Entity`
- `World::despawn(Entity) -> bool` — removes the entity from every
  component store and frees its slot for reuse; `false` if already dead.
- `World::is_alive(Entity) -> bool`
- `World::len()` / `is_empty()` — count of currently-alive entities.

**Components** (all keyed by `Entity`)
- `has_component::<T>(Entity) -> bool`
- `set_component::<T>(Entity, T) -> &mut Self` — insert or overwrite; chainable.
- `unset_component::<T>(Entity) -> &mut Self` — remove if present; chainable.

To mutate a component in place, use `get_mut::<T>` below — `if let Some(c) =
world.get_mut::<T>(entity) { ... }` — rather than a separate method; it's
already a no-op when the component's absent, and unlike a `set`/`unset`-style
chainable wrapper it can return a value out of the closure.

**Single-entity archetype lookup**
- `get::<A>(Entity) -> Option<A::Ref<'_>>`
- `get_mut::<A>(Entity) -> Option<A::RefMut<'_>>`

**World-wide archetype queries**

Every `with_archetype*` closure's last parameter is the `Entity` being
visited — useful for logging/correlation, or for a follow-up lookup against
another, optional component (see the `with_archetype_async_mut` note below
for the one case where that follow-up can't happen *inside* the closure).

- `with_archetype::<A>(f: FnMut(A::Ref<'_>, Entity))` — every matching entity, sequential.
- `with_archetype_mut::<A>(f: FnMut(A::RefMut<'_>, Entity))` — same, mutable. Tuple
  archetypes use disjoint mutable borrows so every member can be written in
  one pass; passing the same type twice (e.g. `(Position, Position)`) panics
  on the mutable path, since that needs two live `&mut` borrows of one slot.
- `with_archetype_async::<A>(f: Fn(A::Ref<'_>, Entity) + Sync)` — same as
  `with_archetype`, split across a scoped thread pool.
- `with_archetype_async_mut::<T: Component>(f: Fn(&mut T, Entity) + Sync)` —
  parallel mutation, but for a *single* component type only (see Design
  below). The closure only gets `&mut T` and `Entity`, not `&World` — that
  component's own store is the one being split mutably across threads, so
  there's no safe way to also hand back a `World` reference to check other
  components *from inside* this callback; do that in a follow-up pass with
  the `Entity` values instead if you need it.

`A` for the tuple form is any tuple of up to 6 `Component` types, e.g.
`world.with_archetype::<(Position, Velocity, Health)>(...)`.

All four `with_archetype*` queries are driven by the smallest of the
matching component stores, not by scanning every entity in the `World` —
querying `(OnScreen, Position)` where 1,000 of 1,000,000 entities are
`OnScreen` costs O(1,000), not O(1,000,000).

## Design

**Why entities are generational ids, not containers.** The previous version
of this crate stored components directly on `Entity` (a `HashMap<TypeId,
Box<dyn Component>>` per entity). That makes single-entity access simple but
makes "every entity with `X`" an O(all entities) scan with no way to do
better — there's no index anywhere that says which entities actually have a
given component. Moving component storage onto `World`, one `SparseSet<T>`
per type, means a query can start from the component's own dense list of
entities instead of guessing-and-checking against everyone.

Generational `Entity` ids are what make an id spawned once, despawned, and
never looked at again *safe* to still be holding: `{index: 5, generation: 0}`
and a later `{index: 5, generation: 1}` (after slot 5 was despawned and
reused) are unequal values, and every `SparseSet` stores the full `Entity`
next to its data, so a lookup with a stale handle returns `None` instead of
silently reading whatever now occupies that slot.

**Why `with_archetype_async_mut` is single-component only.** Parallel
mutation across a tuple archetype would need disjoint mutable access into
*several* independent `SparseSet`s at once, at indices that don't line up
between them (entity 5 might be dense index 2 in one store and dense index
900 in another). That's solvable, but only with unsafe scattered-index code
proven safe by the entity-chunk partition — the standard approach real ECS
engines take. This crate stays 100% safe Rust and keeps the scope narrower
instead: parallel mutation is safe here because it's just `chunks_mut` over
one component's own contiguous storage. Tuple mutation is still available,
just sequential, via `with_archetype_mut`.

## How this compares to other ECS designs

This crate is a small **sparse-set ECS** — the same storage family as
[EnTT](https://github.com/skypjack/entt) (C++) and `specs`' default storage,
as opposed to an **archetype/table ECS** like Bevy or `hecs`, or a **naive
per-entity bag** like this crate's own previous design.

| | per-entity bag (old design) | sparse-set (this crate) | archetype/table (Bevy, hecs) |
|---|---|---|---|
| Component add/remove | O(1), local to the entity | O(1), local to that component's set | can move the entity to a different table |
| "every entity with X" | O(all entities) scan | O(entities with X) via that component's dense array | O(entities with X), often better cache locality across a whole query |
| Multi-component iteration | dict lookup per component per entity | smallest set drives, others probed at scattered indices | fully contiguous, index-aligned columns |
| Parallel mutation across N components | N/A | needs unsafe scattered-index access (not implemented here) | safe by construction — columns are already aligned |
| Structural cost | none (nothing to move) | none (sets are independent) | archetype migration on add/remove |

Sparse-set storage is the middle ground: cheaper structural changes than an
archetype ECS, much better query cost than a per-entity bag, at the cost of
some cache locality on multi-component queries and (here) not attempting
lock-free parallel mutation across several component types at once. This
crate deliberately stays smaller than any of the named engines — no
scheduler, no system dependency graph, no query builder, no
relationships/hierarchies, no serialization. It borrows the storage strategy
of a production sparse-set ECS without the framework built on top of it,
leaving scheduling and ordering to the caller (see the top-level README's
"no hidden control flow" principle).

## Performance

Component stores use a `HashMap<TypeId, Box<dyn ComponentStore>>` with a
custom `FxHasher` instead of Rust's default `SipHash`, since `TypeId` keys
don't need cryptographic hashing.

```
cargo run --release --example bench      # get / tuple get / has_component, tight loop
cargo run --release --example parallel   # sequential vs. parallel bulk queries at scale
```

## Testing

```
cargo test -p ecs
```

`tests/archetype.rs` covers single- and multi-component archetypes (1–6
components) via single-entity `get`/`get_mut`, including the missing-component
and duplicate-type-panic cases. `tests/world.rs` covers `World`-wide queries,
the async variants, and entity lifecycle (despawn, slot reuse, generation
bumps).
