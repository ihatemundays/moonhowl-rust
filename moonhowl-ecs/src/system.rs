use crate::component::IComponent;
use crate::entity::Entity;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::ComponentSet;

pub struct System(usize);

impl System {
    pub fn new() -> Self {
        Self(Self::get_new_id())
    }

    fn get_new_id() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn get_id(&self) -> usize {
        self.0
    }

    pub fn has_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_component::<T>()
    }

    pub fn is_component_read<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.is_component_read::<T>(self.0)
    }

    pub fn has_read_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_read_component::<T>(self.0)
    }

    pub fn has_unread_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_unread_component::<T>(self.0)
    }

    pub fn get_component<'a, T: IComponent>(&self, entity: &'a Entity) -> Option<&'a T> {
        entity.get_component::<T>(self.0)
    }
    
    pub fn has_some_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_components::<T>()
    }

    pub fn has_every_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_component::<T>()
    }

    pub fn has_some_read_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_read_components::<T>(self.0)
    }

    pub fn has_every_read_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_read_component::<T>(self.0)
    }

    pub fn has_some_unread_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_unread_components::<T>(self.0)
    }

    pub fn has_every_unread_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_unread_component::<T>(self.0)
    }
}

pub trait ISystem: Send + Sync {
    fn run(&self, system: &System, entity: &mut Entity);
}
