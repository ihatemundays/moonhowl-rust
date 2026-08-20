use ecs::{Component, World};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

struct Position { x: f32 }
impl Component for Position {}

/// A marker component: no fields, just "this entity is on-screen".
struct OnScreen;
impl Component for OnScreen {}

const TOTAL: usize = 1_000_000;
const TAGGED: usize = 1_000;

fn main() {
    let mut world = World::new();

    for i in 0..TOTAL {
        let entity = world.spawn();
        world.set_component(entity, Position { x: i as f32 });
        if i % (TOTAL / TAGGED) == 0 {
            world.set_component(entity, OnScreen);
        }
    }

    // Bulk queries are driven by the smallest matching store -- here, the
    // ~1,000-entity OnScreen set -- not by scanning all 1,000,000 entities.
    let start = Instant::now();
    let mut visited = 0u32;
    world.with_archetype::<(OnScreen, Position)>(|_| visited += 1);
    println!("sequential (OnScreen, Position): {visited} entities in {:?}", start.elapsed());

    // Note: with only ~1,000 matching entities, this is likely *slower* than the
    // sequential pass above -- scoped-thread spawn overhead dominates at this size.
    // with_archetype_async pays off once the driving set is large, not tiny.
    let start = Instant::now();
    let hits = AtomicU64::new(0);
    world.with_archetype_async::<(OnScreen, Position)>(|_| {
        hits.fetch_add(1, Ordering::Relaxed);
    });
    println!(
        "parallel   (OnScreen, Position): {} entities in {:?}",
        hits.load(Ordering::Relaxed),
        start.elapsed()
    );

    // Parallel mutation is restricted to a single component's dense storage,
    // so it covers all TOTAL entities with Position, tagged or not.
    let start = Instant::now();
    world.with_archetype_async_mut::<Position>(|pos| pos.x *= 2.0);
    println!("parallel mut (Position): {TOTAL} entities in {:?}", start.elapsed());
}
