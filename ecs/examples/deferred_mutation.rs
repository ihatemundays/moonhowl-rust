use ecs::{Component, Entity, World};

struct Health(i32);
impl Component for Health {}

struct Shielded;
impl Component for Shielded {}

fn main() {
    let mut world = World::new();

    let entities: Vec<Entity> = [3, 2, 4, 1].into_iter()
        .map(|hp| {
            let entity = world.spawn();
            world.set_component(entity, Health(hp)).set_component(entity, Shielded);
            entity
        })
        .collect();

    for tick in 1..=4 {
        println!("-- tick {tick} --");

        // Single entity: get_mut, then a separate world call once its borrow ends.
        // Dropping the shield needs has_component/unset_component, which can't
        // happen while `health` (borrowed from world) is still alive -- so the
        // `if let` block only computes a plain bool, and the borrow ends there.
        for &entity in &entities {
            let low_health = if let Some(health) = world.get_mut::<Health>(entity) {
                health.0 -= 1;
                health.0 <= 1
            } else {
                false
            };
            if low_health && world.has_component::<Shielded>(entity) {
                world.unset_component::<Shielded>(entity);
                println!("  {entity:?} loses their shield");
            }
        }

        // Bulk: with_archetype's closure never gets &mut World (true of every
        // with_archetype* variant, not just the _mut ones), so entities that
        // need a structural change -- despawn, here -- are collected during
        // the read pass and applied afterward, once the borrow has ended.
        let mut dead = Vec::new();
        world.with_archetype::<Health>(|health, entity| {
            if health.0 <= 0 {
                dead.push(entity);
            }
        });
        for entity in dead {
            println!("  {entity:?} dies");
            world.despawn(entity);
        }
    }

    println!("{} entities remain", world.len());
}
