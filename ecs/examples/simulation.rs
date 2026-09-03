use ecs::{Component, Entity, World};

struct Name(&'static str);
impl Component for Name {}

struct Position { x: f32, y: f32 }
impl Component for Position {}

struct Velocity { dx: f32, dy: f32 }
impl Component for Velocity {}

struct Health(i32);
impl Component for Health {}

fn main() {
    let mut world = World::new();

    // The roster holds its own Entity handles -- spawning doesn't require
    // tracking ids separately, since Entity is just a small Copy value.
    let roster: Vec<Entity> = [("scout", 1.0, 3), ("bruiser", 0.5, 5), ("skirmisher", 2.0, 2)]
        .into_iter()
        .map(|(name, speed, hp)| {
            let entity = world.spawn();
            world
                .set_component(entity, Name(name))
                .set_component(entity, Position { x: 0.0, y: 0.0 })
                .set_component(entity, Velocity { dx: speed, dy: 0.0 })
                .set_component(entity, Health(hp));
            entity
        })
        .collect();

    // A prop with no Velocity: with_archetype_mut::<(Position, Velocity)> below
    // never touches it, even though it sits in the same World.
    let rock = world.spawn();
    world.set_component(rock, Name("rock")).set_component(rock, Position { x: 5.0, y: 0.0 });

    for tick in 1..=4 {
        // World-driven query: only visits entities that have *both* components.
        world.with_archetype_mut::<(Position, Velocity)>(|(pos, vel), _| {
            pos.x += vel.dx;
            pos.y += vel.dy;
        });

        // Individual-entity lookups: the roster already knows which entities to
        // check, so combat damage doesn't need a world-wide scan.
        for &entity in &roster {
            let Some(health) = world.get_mut::<Health>(entity) else { continue };
            health.0 -= 1;
            if health.0 <= 0 {
                let name = world.get::<Name>(entity).map_or("?", |n| n.0);
                println!("tick {tick}: {name} died");
                world.despawn(entity);
            }
        }

        println!("-- tick {tick} --");
        world.with_archetype::<(Name, Position)>(|(name, pos), _| {
            println!("  {}: ({:.1}, {:.1})", name.0, pos.x, pos.y);
        });
    }

    println!("{} entities remain", world.len());
}
