use crate::component::IComponent;
use crate::entity::Entity;
use std::sync::atomic::{AtomicUsize, Ordering};

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

    pub fn get_component<T: IComponent>(&self, entity: &Entity) -> Option<&T> {
        entity.get_component::<T>(self.0)
    }

    pub fn check<F>(&self, predicate: F) -> SystemCheck
    where
        F: FnOnce(&System) -> bool,
    {
        match predicate(&self) {
            true => SystemCheck::Pass(&self),
            false => SystemCheck::Fail,
        }
    }
}

pub enum SystemCheck {
    Pass(&System),
    Fail,
}

impl SystemCheck {
    pub fn and_then<F>(&self, callback: F) -> &Self
    where
        F: FnOnce(&System),
    {
        match self {
            SystemCheck::Pass(system) => {
                callback(system);
                self
            },
            SystemCheck::Fail => self,
        }
    }
}
