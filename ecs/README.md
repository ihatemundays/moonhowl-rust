# ecs

A tiny, zero-dependency component store for a single `Entity`, with type-safe
multi-component access ("archetypes") and per-entity system activation. No
`World`, no scheduler — just a fast place to hang typed data off an object,
fetch several pieces of it at once, and track which systems currently apply
to it.

## Concepts

- **Component** — any `'static` type. Implement the marker trait `Component`
  (a plain `impl Component for T {}`, no methods) to make it storable.
- **Entity** — owns a heterogeneous set of components and a set of bound
  systems, each keyed by `TypeId` in a hash map using a small
  non-cryptographic hasher (`FxHasher`) tuned for `TypeId` keys. One entity
  is one bag of components; a game object (a Godot node, say) typically owns
  one `Entity` directly as a struct field.
- **System** — any `'static` type implementing `System`. Its `test(&self,
  entity: &Entity) -> bool` decides whether the system applies to a given
  entity (usually by checking which components are present). Its
  `is_lazy(&self) -> bool` (defaults to `true`) decides whether `commit()`
  can skip calling `test()` when nothing was queued that commit — see below.
  Multiple different system types can be bound to the same entity at once.
- **Archetype** — a single `Component` type, or a tuple of 2–6 of them.
  `Entity::has_archetype` checks whether every member component is present;
  `Entity::with_archetype` fetches a whole archetype at once (read-only),
  returning `None` if any member component is missing.

### The command queue

`set_component`/`unset_component` don't touch the live component set
directly — they queue a command, applied in issue order by `commit()`. This
is the *only* way the live component set changes: there is no `&mut T`
accessor anywhere in the API, so nothing can write to a component outside of
`commit()`. To change a component's value, build the new value and
`set_component` it — `commit()` replaces the old one. See "Why there's no
`&mut T`" below.

`commit()` also re-evaluates bound systems against the (now up to date)
component set: for each system where `test()` is actually called, the
system's `TypeId` is added to or removed from the entity's active-system
set accordingly.

Whether `test()` is actually called depends on the system's `is_lazy()`
*and* whether anything was queued this commit. If nothing was queued,
lazy systems (`is_lazy() -> true`, the default) are skipped entirely —
their active-system entry is left exactly as it was, since nothing that
could change their answer has happened. Non-lazy systems (`is_lazy() ->
false`) are always re-tested, queued or not. So after a `commit()`,
`is_system_active::<T>()` reflects the current components for every
non-lazy `T`, and for every lazy `T` whose last real answer hasn't been
invalidated by a no-op commit in between.

This is why `AllRefreshedSystem`/`AnyRefreshedSystem` below override
`is_lazy()` to `false`: their whole job is noticing when *nothing* changed
(to decay back to inactive), which requires `test()` to run even on a
commit with nothing queued. A plain existence check like `Movement` below
is a good match for the lazy default — if no components changed, its
answer can't have changed either, so skipping it is a safe, free
optimization.

```rust
use ecs::{Component, Entity, System};

struct Position { x: f32, y: f32 }
impl Component for Position {}

struct Velocity { dx: f32, dy: f32 }
impl Component for Velocity {}

struct Movement;
impl System for Movement {
    fn test(&self, entity: &Entity) -> bool {
        entity.has_component::<Position>() && entity.has_component::<Velocity>()
    }
}

fn main() {
    let mut entity = Entity::new();
    entity
        .bind_system(Movement)
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.5 })
        .commit();

    assert!(entity.is_system_active::<Movement>());

    let (pos, vel) = entity
        .with_system_archetype::<Movement, (Position, Velocity)>()
        .unwrap();
    let moved = Position { x: pos.x + vel.dx, y: pos.y + vel.dy };
    entity.set_component(moved).commit();

    let moved = entity.with_archetype::<Position>().map(|pos| (pos.x, pos.y));
    assert_eq!(moved, Some((1.0, 0.5)));
}
```

## API

### Components

- `has_component::<T>() -> bool`
- `get_component::<T>() -> Option<&T>` — read-only; there is no `_mut`
  counterpart, by design (see below).
- `set_component::<T>(value) -> &mut Self` — queue an insert-or-overwrite
  command; chainable. Not visible to reads until `commit()`.
- `unset_component::<T>() -> &mut Self` — queue a remove command; chainable.
  Not visible to reads until `commit()`.
- `commit() -> &mut Self` — drain queued `set_component`/`unset_component`
  commands and apply them to the live component set, in the order they were
  issued, then re-test each bound system whose `is_lazy()` is `false`, or
  whose `is_lazy()` is `true` but something was actually queued this call,
  to refresh the active-system set; chainable.

### Archetypes

- `has_archetype::<A>() -> bool` — whether every component in archetype `A`
  is present, without fetching any of them.
- `with_archetype::<A>() -> Option<A::Ref<'_>>` — fetch every component in
  archetype `A` directly (a `&T` for a single type, a tuple of `&T`s for a
  tuple archetype); `None` if any is missing.

`A` for the tuple form is any tuple of up to 6 types implementing
`Component`, e.g. `entity.with_archetype::<(Position, Velocity, Health)>()`.

`with_archetype` returns the fetched references directly rather than taking
a closure — earlier it took `f: impl FnOnce(A::Ref<'_>) -> R` and returned
`Option<R>`, but a closure boundary makes it awkward to interleave archetype
reads with other borrows or early returns in the caller. Since it now just
hands back `Option<A::Ref<'_>>`, normal borrow-checker rules apply directly
to the caller's code instead of being mediated through a lambda.

### Systems

The `System` trait itself has two methods:

- `test(&self, entity: &Entity) -> bool` — required. Decides whether the
  system applies to `entity`.
- `is_lazy(&self) -> bool` — defaults to `true`. When `true` and a `commit()`
  has nothing queued, `test()` is skipped for that system and its
  active-system entry is left untouched. Override to `false` for a system
  that needs to run on every `commit()` regardless of whether anything was
  queued — see `AllRefreshedSystem`/`AnyRefreshedSystem` below.

`Entity` methods for working with bound systems:

- `bind_system::<T>(system: T) -> &mut Self` — bind a system instance to the
  entity; chainable. Takes effect immediately (unlike components, binding
  isn't queued through `commit()`).
- `unbind_system::<T>() -> &mut Self` — remove a bound system and drop its
  active-system entry if set; chainable.
- `is_system_active::<T>() -> bool` — whether `T`'s last `test()` (run
  during the most recent `commit()`) returned `true`.
- `with_system_archetype::<S, A>() -> Option<A::Ref<'_>>` — like
  `with_archetype`, but only returns `Some` if system `S` is currently
  active on the entity.

### `systems::AllRefreshedSystem<A>` — change detection

`AllRefreshedSystem<A>` (in the `systems` module, alongside other ready-made
`System`s) is active only once every member of archetype `A` has a fresh
address since the last `commit()` — i.e. all of them were actually
re-`set_component`'d, not left over from before. `set_component` always
allocates a new `Box`, so an unchanged component keeps the address
`AllRefreshedSystem` last recorded for it; it's `false` as soon as *any*
single member repeats.

It overrides `is_lazy()` to `false`. If it stayed lazy (the default),
`commit()` would skip calling its `test()` on any commit with nothing
queued — so it would never get the chance to notice "nothing changed" and
decay back to inactive, and would stay stuck `true` from whenever it last
saw every member refresh.

```rust
use ecs::Entity;
use ecs::systems::AllRefreshedSystem;

type Refresh = AllRefreshedSystem<(Position, Velocity, Health)>;

entity.bind_system(Refresh::new());
entity.commit();
entity.is_system_active::<Refresh>(); // true only once all three are fresh
```

It's built on a second, opt-in trait, `AddressArchetype: Archetype`
(implemented for the same single-type-or-tuple-of-up-to-6 shapes as
`Archetype`), which fetches each member's address instead of its data.
This is kept separate from `Archetype` itself — most archetypes are only
ever read via `with_archetype` and shouldn't have to carry
address-comparison machinery they don't use.

`test()` needs `Cell` for interior mutability internally, since `System::test`
takes `&self`, not `&mut self` — there's no other way for it to remember
what it saw last time.

To use it as a building block inside a larger system, hold it as a plain
field and call its `test` directly, rather than binding it as its own
system and checking `is_system_active` from a sibling — within a single
`commit()`, systems run in unspecified order, so there's no guarantee
`AllRefreshedSystem` would already have run when another system asked
about it:

```rust
struct RevivedAndMoving {
    refresh: AllRefreshedSystem<(Position, Velocity, Health)>,
}

impl System for RevivedAndMoving {
    // Same reason `AllRefreshedSystem` itself overrides this: it embeds
    // change detection, so it needs to run every commit to notice when
    // nothing refreshed and decay back to inactive.
    fn is_lazy(&self) -> bool {
        false
    }

    fn test(&self, entity: &Entity) -> bool {
        self.refresh.test(entity) && entity.get_component::<Health>().is_some_and(|h| h.0 > 0)
    }
}
```

See `examples/all_refreshed_system.rs` and `tests/all_refreshed_system.rs` for a plain
walkthrough, and `examples/composed_refreshed_system.rs` for the composed-system
version above:

```
cargo run --example all_refreshed_system
cargo run --example composed_refreshed_system
```

### `systems::AnyRefreshedSystem<A>` — change detection (any member)

`AnyRefreshedSystem<A>` is `AllRefreshedSystem`'s OR counterpart: active
once *at least one* member of archetype `A` has a fresh address since the
last `commit()`, rather than requiring every member to refresh together.
Naming mirrors `Iterator::all`/`Iterator::any` on purpose — same AND/OR
relationship, same names.

```rust
use ecs::Entity;
use ecs::systems::AnyRefreshedSystem;

type Refresh = AnyRefreshedSystem<(Position, Velocity, Health)>;

entity.bind_system(Refresh::new());
entity.commit();
entity.is_system_active::<Refresh>(); // true as soon as any one of the three is fresh
```

Like `AllRefreshedSystem`, it overrides `is_lazy()` to `false` for the same
reason: it needs `test()` to run on every commit, queued or not, to notice
when nothing refreshed and decay back to inactive.

Both systems share `AddressArchetype`'s two combinators: `any_repeats`
(true if at least one member's address is unchanged, OR of equality) and
`all_repeat` (true only if every member's address is unchanged, AND of
equality). `AllRefreshedSystem::test` negates the first, `AnyRefreshedSystem::test`
negates the second — De Morgan's laws turn "OR of equal" into "AND of
different" and vice versa, which is exactly the AND/ANY split above.

For a single-component archetype, `AllRefreshedSystem<T>` and
`AnyRefreshedSystem<T>` are identical — there's only one member, so "all"
and "any" collapse to the same check. The distinction only matters once
`A` has 2 or more members.

See `examples/any_refreshed_system.rs` and `tests/any_refreshed_system.rs`:

```
cargo run --example any_refreshed_system
```

### Why there's no `&mut T`

`set_component`/`unset_component` + `commit()` are the *only* way a
component's value changes. There's deliberately no `get_component_mut` or
`with_archetype_mut` — either would hand out a live `&mut T` into the
committed component set, which is a write that bypasses the command queue
entirely: it happens instantly, out of order relative to whatever else is
queued, and isn't visible as a command to anything inspecting `commit()`'s
effects. Removing those accessors makes "components only change via
`commit()`" a property the type system enforces, not a convention callers
have to honor. To update a component, build the new value (reading the old
one via `get_component`/`with_archetype` if needed) and `set_component` it;
`commit()` replaces the old value with the new one.

If you need an owned copy of a component's data (e.g. to release the borrow
from `with_archetype` before queuing a `set_component`), that's on the
caller — clone the value yourself inside the closure. The crate doesn't add
a `Copy`/`Clone` bound or a parallel owned-fetch path for this; components
aren't required to be `Copy` or `Clone` at all.

## Performance

`Entity` uses `HashMap<TypeId, ...>` (for components, systems, and the
active-system set) with a custom `FxHasher` instead of Rust's default
`SipHash`, since `TypeId` keys don't need cryptographic hashing.
`examples/bench.rs` measures `get_component`, `with_archetype` (3-component
tuple), and `has_component` in a tight loop:

```
cargo run --release --example bench
```

## Testing

```
cargo test -p ecs
```

`tests/archetype.rs` covers single- and multi-component archetypes (1–6
components), reads, deferred/ordered command application, overwrites via
`set_component`, and missing-component misses.

`tests/system_laziness.rs` covers `System::is_lazy()`: it defaults to
`true`; a lazy system's `test()` is skipped by `commit()` on a call with
nothing queued (its active-system entry is left untouched), while a
system overriding `is_lazy()` to `false` is re-tested every `commit()`
regardless; and `commit()` still applies queued components either way.

`tests/all_refreshed_system.rs` and `tests/any_refreshed_system.rs` cover
`AllRefreshedSystem` and `AnyRefreshedSystem` respectively: single- and
multi-member archetypes, going inactive when a required member is
missing, and the AND-vs-OR contrast — `AllRefreshedSystem` needs every
member fresh in the same commit, `AnyRefreshedSystem` only needs one.
