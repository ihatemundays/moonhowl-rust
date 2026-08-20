use ecs::{Component, World};
use std::hint::black_box;
use std::time::Instant;

struct Position { x: f32 }
impl Component for Position {}

struct Velocity { dx: f32 }
impl Component for Velocity {}

struct Health(f32);
impl Component for Health {}

const ITERS: u32 = 20_000_000;

fn main() {
    let mut world = World::new();
    let entity = world.spawn();
    world
        .set_component(entity, Position { x: 0.0 })
        .set_component(entity, Velocity { dx: 1.0 })
        .set_component(entity, Health(100.0));
    let world = black_box(world);
    let entity = black_box(entity);

    let start = Instant::now();
    let mut acc = 0.0f32;
    for _ in 0..ITERS {
        acc += black_box(&world).get::<Position>(entity).map_or(0.0, |p| p.x);
    }
    let elapsed = start.elapsed();
    black_box(acc);
    println!("get::<Position>: {:.2} ns/op", elapsed.as_nanos() as f64 / ITERS as f64);

    let start = Instant::now();
    let mut acc = 0.0f32;
    for _ in 0..ITERS {
        acc += black_box(&world)
            .get::<(Position, Velocity, Health)>(entity)
            .map_or(0.0, |(p, v, h)| p.x + v.dx + h.0);
    }
    let elapsed = start.elapsed();
    black_box(acc);
    println!("get::<(Position, Velocity, Health)>: {:.2} ns/op", elapsed.as_nanos() as f64 / ITERS as f64);

    let start = Instant::now();
    let mut hits = 0u32;
    for _ in 0..ITERS {
        if black_box(&world).has_component::<Position>(entity) {
            hits += 1;
        }
    }
    let elapsed = start.elapsed();
    black_box(hits);
    println!("has_component::<Position>: {:.2} ns/op", elapsed.as_nanos() as f64 / ITERS as f64);
}
