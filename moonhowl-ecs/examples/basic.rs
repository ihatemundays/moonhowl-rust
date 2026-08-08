use moonhowl_ecs::{ActionContext, CheckContext, EntityCore, IComponent, IEntity, ISystem, World};
use moonhowl_macros::{ecs_component, ecs_entity};
use std::any::Any;

struct Position {
    x: f32,
    y: f32,
}

impl IComponent for Position {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[ecs_component]
struct Velocity {
    dx: f32,
    dy: f32,
}

#[ecs_entity]
struct MovingThing {
    entity_core: EntityCore,
}

struct MovementLogger;

impl ISystem<MovingThing> for MovementLogger {
    fn check(&self, system: &CheckContext, entity: &MovingThing) -> bool {
        system.has_every_unread_component::<(Position, Velocity), _>(entity)
    }

    fn and_then(&self, system: &ActionContext, entity: &MovingThing) {
        let Some((position, velocity)) = system.read_components::<(Position, Velocity), _>(entity)
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

    let mut moving = MovingThing {
        entity_core: EntityCore::new(),
    };
    moving
        .entity_core
        .set_component(Position { x: 0.0, y: 0.0 })
        .set_component(Velocity { dx: 1.0, dy: 0.5 });
    world.spawn(moving);

    let mut stationary = MovingThing {
        entity_core: EntityCore::new(),
    };
    stationary
        .entity_core
        .set_component(Position { x: 5.0, y: 5.0 });
    world.spawn(stationary);

    world.register_system::<MovingThing, _>(MovementLogger);

    // stationary is skipped: it has no Velocity, so MovementLogger::check
    // never passes for it.
    world.run();
}
