use ecs::{Component, World};
use std::sync::atomic::{AtomicI32, Ordering};

struct Position {
    x: f32,
}

impl Component for Position {}

struct Velocity {
    dx: f32,
}

impl Component for Velocity {}

#[test]
fn with_archetype_runs_over_every_matching_entity() {
    let mut world = World::new();
    for i in 0..5 {
        let entity = world.spawn();
        world.set_component(entity, Position { x: i as f32 });
    }
    world.spawn();

    let mut sum = 0.0;
    world.with_archetype::<Position>(|pos, _| sum += pos.x);

    assert_eq!(sum, 0.0 + 1.0 + 2.0 + 3.0 + 4.0);
}

#[test]
fn with_archetype_mut_updates_every_matching_entity_in_place() {
    let mut world = World::new();
    for i in 0..5 {
        let entity = world.spawn();
        world.set_component(entity, Position { x: i as f32 });
    }

    world.with_archetype_mut::<Position>(|pos, _| pos.x *= 2.0);

    let mut sum = 0.0;
    world.with_archetype::<Position>(|pos, _| sum += pos.x);
    assert_eq!(sum, 0.0 + 2.0 + 4.0 + 6.0 + 8.0);
}

#[test]
fn with_archetype_parallel_runs_over_every_matching_entity() {
    let mut world = World::new();
    for _ in 0..10_000 {
        let entity = world.spawn();
        world.set_component(entity, Position { x: 1.0 });
    }
    for _ in 0..10_000 {
        world.spawn();
    }

    let hits = AtomicI32::new(0);
    world.with_archetype_async::<Position>(|_pos, _| {
        hits.fetch_add(1, Ordering::Relaxed);
    });

    assert_eq!(hits.load(Ordering::Relaxed), 10_000);
}

#[test]
fn with_archetype_async_mut_updates_every_component_in_place() {
    let mut world = World::new();
    for _ in 0..10_000 {
        let entity = world.spawn();
        world.set_component(entity, Position { x: 1.0 });
    }

    world.with_archetype_async_mut::<Position>(|pos, _| pos.x += 2.0);

    let mut sum = 0.0;
    world.with_archetype::<Position>(|pos, _| sum += pos.x);
    assert_eq!(sum, 3.0 * 10_000.0);
}

#[test]
fn with_archetype_async_mut_hands_back_the_matching_entity() {
    let mut world = World::new();
    let entities: Vec<_> = (0..1_000)
        .map(|i| {
            let entity = world.spawn();
            world.set_component(entity, Position { x: i as f32 });
            entity
        })
        .collect();

    let seen = std::sync::Mutex::new(Vec::new());
    world.with_archetype_async_mut::<Position>(|_pos, entity| {
        seen.lock().unwrap().push(entity);
    });

    let mut seen = seen.into_inner().unwrap();
    seen.sort_by_key(|e| e.index());
    let mut expected = entities;
    expected.sort_by_key(|e| e.index());
    assert_eq!(seen, expected);
}

#[test]
fn with_archetype_mut_updates_every_matching_tuple_entity_in_place() {
    let mut world = World::new();
    for _ in 0..10_000 {
        let entity = world.spawn();
        world.set_component(entity, Position { x: 1.0 });
        world.set_component(entity, Velocity { dx: 2.0 });
    }

    world.with_archetype_mut::<(Position, Velocity)>(|(pos, vel), _| pos.x += vel.dx);

    let mut sum = 0.0;
    world.with_archetype::<Position>(|pos, _| sum += pos.x);
    assert_eq!(sum, 3.0 * 10_000.0);
}

#[test]
fn despawn_removes_the_entity_from_every_store() {
    let mut world = World::new();
    let a = world.spawn();
    world.set_component(a, Position { x: 1.0 });
    let b = world.spawn();
    world.set_component(b, Position { x: 2.0 });

    assert!(world.despawn(a));
    assert!(!world.is_alive(a));
    assert!(!world.has_component::<Position>(a));

    let mut sum = 0.0;
    world.with_archetype::<Position>(|pos, _| sum += pos.x);
    assert_eq!(sum, 2.0);
}

#[test]
fn despawn_frees_the_slot_for_reuse_with_a_bumped_generation() {
    let mut world = World::new();
    let a = world.spawn();
    world.despawn(a);
    let b = world.spawn();

    assert_eq!(a.index(), b.index());
    assert_ne!(a.generation(), b.generation());
    assert!(!world.is_alive(a));
    assert!(world.is_alive(b));
}

#[test]
fn get_looks_up_a_single_entity() {
    let mut world = World::new();
    let entity = world.spawn();
    world.set_component(entity, Position { x: 1.0 });
    world.set_component(entity, Velocity { dx: 2.0 });

    assert_eq!(world.get::<Position>(entity).map(|p| p.x), Some(1.0));
    assert_eq!(
        world.get::<(Position, Velocity)>(entity).map(|(p, v)| (p.x, v.dx)),
        Some((1.0, 2.0))
    );
}
