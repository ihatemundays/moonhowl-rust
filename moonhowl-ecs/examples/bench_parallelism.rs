use moonhowl_ecs::{ActionContext, CheckContext, Entity, IComponent, ISystem, World};
use std::any::Any;
use std::time::Instant;

#[derive(Clone, Copy)]
struct Position(f32);
impl IComponent for Position {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Clone, Copy)]
struct Velocity(f32);
impl IComponent for Velocity {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct Thing;

struct ApplyVelocity;
impl ISystem for ApplyVelocity {
    fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
        system.has_every_component::<(Position, Velocity)>(entity)
    }

    fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
        let (position, velocity) = system.get_components::<(Position, Velocity)>(entity).unwrap();
        system.set_component(entity, Position(position.0 + velocity.0));
    }
}

fn build_world(entity_count: usize) -> World {
    let mut world = World::new();
    for _ in 0..entity_count {
        let id = world.spawn::<Thing>();
        world
            .get_entity_mut::<Thing>(id)
            .unwrap()
            .set_component(Position(0.0))
            .set_component(Velocity(1.0));
    }
    world.register_system::<Thing, _>(ApplyVelocity);
    world
}

fn time_ticks<F: FnMut(&mut World)>(mut world: World, ticks: u32, mut run: F) -> std::time::Duration {
    let start = Instant::now();
    for _ in 0..ticks {
        run(&mut world);
        world.confirm();
    }
    start.elapsed()
}

fn main() {
    let ticks = 20;
    let parallelism = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    println!("available_parallelism = {parallelism}, {ticks} ticks per row\n");

    println!(
        "{:>10} | {:>12} | {:>12} | {:>12} | {:>12} | {:>12}",
        "entities", "run_sync", "run", "run_chunked", "run_checked_sync", "run_max_parallel"
    );

    for &entity_count in &[10usize, 100, 1_000, 10_000] {
        let sync = time_ticks(build_world(entity_count), ticks, |w| w.run_sync());
        let run = time_ticks(build_world(entity_count), ticks, |w| w.run());
        let chunked = time_ticks(build_world(entity_count), ticks, |w| w.run_chunked());
        let checked = time_ticks(build_world(entity_count), ticks, |w| w.run_checked_sync());

        // Capped: at 10k entities this would spawn 10k * ticks threads, which is
        // exactly the pathological case being demonstrated, not something to hide from,
        // but keeping it bounded avoids multi-minute runs while still making the point.
        let max_parallel = if entity_count <= 2_000 {
            Some(time_ticks(build_world(entity_count), ticks, |w| w.run_max_parallel()))
        } else {
            None
        };

        println!(
            "{:>10} | {:>10.2?} | {:>10.2?} | {:>10.2?} | {:>14.2?} | {:>15}",
            entity_count,
            sync,
            run,
            chunked,
            checked,
            max_parallel.map(|d| format!("{d:.2?}")).unwrap_or_else(|| "skipped".to_string())
        );
    }
}
