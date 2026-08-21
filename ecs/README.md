# ecs

A small, zero-dependency archetype/table ECS. Entities with the same exact
component set live packed together in one table, each component stored in
its own contiguous, row-aligned column; entities are cheap generational ids,
not containers. Both whole-`World` queries and single-entity lookups are
first-class — there's no scheduler and no query builder, just typed access
to data the caller decides when and how to touch.

## Concepts

- **Component** — any `'static + Send + Sync` type. Implement the marker
  trait `Component` (`impl Component for T {}`, no methods) to make it
  storable.
- **Entity** — a `Copy` id: `{ index, generation }`. It carries no data of
  its own; despawning bumps the slot's generation so a stale `Entity` you're
  still holding fails lookups safely instead of resolving to whatever
  reoccupies that index later. Cheap enough to store anywhere — a Godot node
  can hold one as a plain field.
- **World** — owns entity allocation (`spawn`/`despawn`) and a table per
  unique component set an entity has ever held. All component reads/writes
  go through `World`, keyed by `Entity`; internally it resolves `Entity` to
  a `(table, row)` location, invisible to callers.
- **SparseSet\<T\>** — a general-purpose, O(1) insert/remove/get store keyed
  by `Entity`, densely packed for iteration. `World` doesn't use it
  internally (see Design) — it's exported as an independently useful
  structure for your own entity-keyed data outside of `World`.
- **Archetype** — a single `Component` type, or a tuple of 2–6 of them,
  naming what a query fetches. `World::get`/`get_mut` fetch one for a single
  entity; `with_archetype*` fetch it for every matching entity in the
  `World`. (This is the same word `World` uses internally for "a table's
  exact component set" — a query's `Archetype` just has to be a *subset* of
  whatever full set a matching table holds, not an exact match.)

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
- `World::despawn(Entity) -> bool` — removes the entity's row from its
  table and frees its slot for reuse; `false` if already dead.
- `World::is_alive(Entity) -> bool`
- `World::len()` / `is_empty()` — count of currently-alive entities.

**Components** (all keyed by `Entity`)
- `has_component::<T>(Entity) -> bool`
- `set_component::<T>(Entity, T) -> &mut Self` — insert or overwrite;
  chainable. Setting a component the entity doesn't already have moves its
  row to a different table (see Design) — that's amortized O(1) via a cached
  per-table transition, not a fresh lookup every time.
- `unset_component::<T>(Entity) -> &mut Self` — remove if present;
  chainable, same table-migration cost as `set_component`.

To mutate a component in place, use `get_mut::<T>` below — `if let Some(c) =
world.get_mut::<T>(entity) { ... }` — rather than a separate method; it's
already a no-op when the component's absent, and unlike a `set`/`unset`-style
chainable wrapper it can return a value out of the closure, and never moves
the entity's row (no component set change).

**Single-entity archetype lookup**
- `get::<A>(Entity) -> Option<A::Ref<'_>>`
- `get_mut::<A>(Entity) -> Option<A::RefMut<'_>>`

**World-wide archetype queries**

Every `with_archetype*` closure's last parameter is the `Entity` being
visited — useful for logging/correlation, or for a follow-up lookup against
another, optional component.

- `with_archetype::<A>(f: FnMut(A::Ref<'_>, Entity))` — every matching entity, sequential.
- `with_archetype_mut::<A>(f: FnMut(A::RefMut<'_>, Entity))` — same, mutable. Tuple
  archetypes use disjoint mutable column borrows so every member can be written in
  one pass; passing the same type twice (e.g. `(Position, Position)`) panics
  on the mutable path, since that needs two live `&mut` borrows of one column.
- `with_archetype_async::<A>(f: Fn(A::Ref<'_>, Entity) + Sync)` — same as
  `with_archetype`, split across a scoped thread pool.
- `with_archetype_async_mut::<A>(f: Fn(A::RefMut<'_>, Entity) + Sync)` —
  same as `with_archetype_mut`, split across a scoped thread pool. Unlike
  the other three, `A` here covers tuples too, not just single components
  (see Design) — the closure still only gets `A::RefMut<'_>` and `Entity`,
  not `&World`, since the columns it touches are the ones being split
  mutably across threads; do a follow-up sequential pass with the `Entity`
  values if you need to check something else from inside.

`A` for the tuple form is any tuple of up to 6 `Component` types, e.g.
`world.with_archetype::<(Position, Velocity, Health)>(...)`.

All four `with_archetype*` queries walk every table whose component set is a
superset of the query's, not the whole `World` — querying `(OnScreen,
Position)` where only one table actually has both columns costs proportional
to that table's size, not the total entity count. Which tables match is
cached per archetype and only rescanned over tables created since the cache
was last built (tables are created lazily and never removed) — see Design.

## Design

**Why entities are generational ids, not containers.** An early version of
this crate stored components directly on `Entity` (a `HashMap<TypeId, Box<dyn
Component>>` per entity). That makes single-entity access simple but makes
"every entity with `X`" an O(all entities) scan with no way to do better —
there's no index anywhere that says which entities actually have a given
component. Storing components off to the side and resolving `Entity` to a
location means a query can start from an index of who actually has what,
instead of guessing-and-checking against everyone.

Generational `Entity` ids are what make an id spawned once, despawned, and
never looked at again *safe* to still be holding: `{index: 5, generation: 0}`
and a later `{index: 5, generation: 1}` (after slot 5 was despawned and
reused) are unequal values, and every lookup — `get`, `get_mut`,
`has_component`, `set_component`, `unset_component` — resolves through a
`generations` table keyed by index first, so a stale handle returns
`None`/no-ops instead of silently reading or corrupting whatever now
occupies that slot.

**Why archetype/table storage, not sparse-set.** An earlier version of this
crate gave every component type its own global `SparseSet<T>` (see
`SparseSet` above), with multi-component queries driven by whichever
involved store was currently smallest, probing the others at scattered
indices. That turned out to have a real, data-dependent cost: when two
queried components' stores happened to have been populated in the same
order (the common case — most components are set together at spawn),
probing was accidentally near-sequential and cheap; when their insertion
histories diverged — a component added long after spawn, or reordered by
despawn churn — each probe was a genuine cache miss, measured at roughly an
order of magnitude slower on a multi-million-entity benchmark. Grouping
entities by their exact component set into contiguous, row-aligned tables
removes the variance entirely: every column in a matching table walks in
lockstep with every other, with zero per-entity indirection, regardless of
how or when each entity acquired its components.

The cost moves to structural changes instead: `set_component`/
`unset_component` now migrate a row between tables when they add or remove
a component type the entity didn't already have (not when overwriting one
it already has). Each table caches its `add`/`remove` transition for every
component type it's seen used, so a repeated identical transition — the
common case, since most entities of a given kind acquire/lose components in
the same few ways — is an O(1) cached edge lookup, not a fresh scan.

**Why bulk queries resolve columns once per table, not once per entity.**
`Archetype::resolve_columns`/`resolve_columns_mut` look up and downcast a
query's columns once per *matching table* per `with_archetype*` call, handing
back direct `&[T]`/`&mut [T]` slices; the per-entity step is then just an
array index, no further lookup or downcast. Combined with the cached
matching-table list (previous section), a repeated query over a `World`
with no new archetypes appearing costs one cheap cache check plus a
straight-line walk — no hashmap traffic in the per-entity loop at all.

**Why `with_archetype_async_mut` covers tuples, not just single components.**
An earlier, sparse-set-backed version of this crate restricted parallel
mutation to one component type at a time: splitting several *independent*
sparse sets safely across threads would need scattered-index access proven
sound by the chunk partition, which isn't expressible without `unsafe`.
Table storage removes that obstacle — every column in one table is a
same-length `Vec<T>`, row-aligned with every other column in that table, so
splitting several of them by the *same* disjoint row ranges is exactly the
soundness argument `<[T]>::split_at_mut`/`chunks_mut` already rely on for a
single slice, just applied to a tuple of slices at once. No `unsafe` needed.

**A trade-off worth knowing about: fragmentation.** Because a table groups
entities by their *entire* component set, a single-component query like
`with_archetype_async_mut::<Position>` no longer has one global store to
walk — it walks every table that includes a `Position` column, however many
distinct archetypes that turns out to be. If your entities are spread across
many different component combinations, a query for one broadly-shared
component pays a little overhead per table (each gets its own scoped-thread
batch) instead of one large contiguous pass. This is the standard trade-off
archetype ECS engines make (Bevy and flecs both document it): multi-component
queries — the case this redesign targets — get dramatically faster and lose
their insertion-order variance; very broad single-component queries over a
world with many distinct archetypes can do a bit more (still linear, still
correct) work than a sparse-set world would have.

## How this compares to other ECS designs

This crate is a small **archetype/table ECS** — the same storage family as
[Bevy](https://bevyengine.org/) and [flecs](https://www.flecs.dev/) — having
moved there from an interim **sparse-set** design (the same family as
[EnTT](https://github.com/skypjack/entt) (C++) and `specs`' default
storage), which itself replaced an original **naive per-entity bag**.

| | per-entity bag (original) | sparse-set (interim) | archetype/table (this crate) |
|---|---|---|---|
| Component add/remove | O(1), local to the entity | O(1), local to that component's set | migrates the entity's row to a different table, amortized O(1) via a cached transition |
| "every entity with X" | O(all entities) scan | O(entities with X) via that component's dense array | O(entities with X), walking whichever tables have that column |
| Multi-component iteration | dict lookup per component per entity | smallest set drives, others probed at scattered indices (cost varies with insertion-order correlation) | fully contiguous, index-aligned columns — no variance |
| Parallel mutation across N components | N/A | needs unsafe scattered-index access (never implemented here) | safe by construction — columns in one table are already row-aligned |
| Structural cost | none (nothing to move) | none (sets are independent) | one table-to-table row move, all-or-nothing per transition |

This crate deliberately stays smaller than any of the named engines — no
scheduler, no system dependency graph, no query builder, no
relationships/hierarchies, no serialization. It borrows the storage strategy
of a production archetype ECS without the framework built on top of it,
leaving scheduling and ordering to the caller (see the top-level README's
"no hidden control flow" principle).

## Performance

Table and archetype-registry maps (`TypeId` keys, both for a table's own
columns and for its cached add/remove transition edges) use a custom
`FxHasher` instead of Rust's default `SipHash`, since `TypeId` keys don't
need cryptographic hashing.

Tables are created lazily and never removed, and a table's column set never
changes once created — so the per-archetype "which tables match" cache (see
Design) only ever needs to scan tables added since it was last consulted,
never re-checking ones it already classified.

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
