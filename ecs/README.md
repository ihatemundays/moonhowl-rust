# ecs

A tiny, zero-dependency component store for a single `Entity`, with type-safe
multi-component access ("archetypes"). No `World`, no scheduler, no systems —
just a fast place to hang typed data off an object and fetch several pieces
of it at once.

## Concepts

- **Component** — any `'static` type. Implement the marker trait `Component`
  (a plain `impl Component for T {}`, no methods) to make it storable.
- **Entity** — owns a heterogeneous set of components, keyed by `TypeId` in a
  hash map using a small non-cryptographic hasher (`FxHasher`) tuned for
  `TypeId` keys. One entity is one bag of components; a game object (a Godot
  node, say) typically owns one `Entity` directly as a struct field.
- **Archetype** — a single `Component` type, or a tuple of 2–6 of them.
  `Entity::with_archetype`/`with_archetype_mut` fetch a whole archetype at
  once, returning `None` if any member component is missing.

```rust
use ecs::{Component, Entity};

struct Position { x: f32, y: f32 }
impl Component for Position {}

struct Velocity { dx: f32, dy: f32 }
impl Component for Velocity {}

fn main() {
    let mut entity = Entity::new();
    entity
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.5 });

    entity.with_archetype_mut::<(Position, Velocity), _>(|(pos, vel)| {
        pos.x += vel.dx;
        pos.y += vel.dy;
    });

    let moved = entity.with_archetype::<Position, _>(|pos| (pos.x, pos.y));
    assert_eq!(moved, Some((1.0, 0.5)));
}
```

## API

- `Entity::new()` — an entity with no components.
- `has_component::<T>() -> bool`
- `get_component::<T>() -> Option<&T>` / `get_component_mut::<T>() -> Option<&mut T>`
- `set_component::<T>(value) -> &mut Self` — insert or overwrite; chainable.
- `unset_component::<T>() -> &mut Self` — remove if present; chainable.
- `edit_component::<T>(f: FnOnce(&mut T)) -> &mut Self` — mutate in place,
  no-op if the component is absent; chainable.
- `with_archetype::<A, R>(f: FnOnce(A::Ref<'_>) -> R) -> Option<R>` — read
  every component in archetype `A`; `None` if any is missing, otherwise
  `Some(f(...))`.
- `with_archetype_mut::<A, R>(f: FnOnce(A::RefMut<'_>) -> R) -> Option<R>` —
  same, but mutable. Uses disjoint mutable borrows internally so all members
  of a tuple archetype can be written in the same call; passing the *same*
  type twice in a tuple (e.g. `(Position, Position)`) panics on the mutable
  path, since that would require two live `&mut` borrows of one slot.

`A` for the tuple form is any tuple of up to 6 types implementing
`Component`, e.g. `entity.with_archetype::<(Position, Velocity, Health), _>(...)`.

## Performance

`Entity` uses a `HashMap<TypeId, Box<dyn Component>>` with a custom
`FxHasher` instead of Rust's default `SipHash`, since `TypeId` keys don't
need cryptographic hashing. `examples/bench.rs` measures `get_component`,
`with_archetype` (3-component tuple), and `has_component` in a tight loop:

```
cargo run --release --example bench
```

## Testing

```
cargo test -p ecs
```

`tests/archetype.rs` covers single- and multi-component archetypes (1–6
components), reads, mutations, missing-component misses, and the
duplicate-type panic case.
