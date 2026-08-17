use moonhowl_ecs::{ActionContext, CheckContext, Entity, ISystem, World};
use moonhowl_macros::Component;

#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct Velocity {
    dx: f32,
    dy: f32,
}

struct MovingThing;

struct MovementLogger;

impl ISystem for MovementLogger {
    fn check(&self, system: &CheckContext, entity: &Entity) -> bool {
        system.has_every_unread_component::<(Position, Velocity)>(entity)
    }

    fn and_then(&self, system: &ActionContext<'_>, entity: &Entity) {
        let Some((position, velocity)) = system.read_components::<(Position, Velocity)>(entity)
        else {
            return;
        };

        println!(
            "entity {}: ({}, {}) moving by ({}, {})",
            entity.get_id(),
            position.x,
            position.y,
            velocity.dx,
            velocity.dy,
        );
    }
}

fn main() {
    let mut world = World::new();

    let moving = world.spawn::<MovingThing>();
    world
        .get_entity_mut::<MovingThing>(moving)
        .unwrap()
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.5 });

    let stationary = world.spawn::<MovingThing>();
    world
        .get_entity_mut::<MovingThing>(stationary)
        .unwrap()
        .set_component(Position { x: 5.0, y: 5.0 });

    world.register_system::<MovingThing, _>(MovementLogger);

    // stationary is skipped: it has no Velocity, so MovementLogger::check
    // never passes for it.
    world.run();
    world.confirm();
}
