use crate::component::IComponent;
use crate::system::System;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};

pub struct Entity {
    components: HashMap<TypeId, Box<dyn IComponent>>,
    registered_components: HashMap<TypeId, HashSet<usize>>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            registered_components: HashMap::new(),
        }
    }

    pub fn has_component<T: IComponent>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    pub fn has_registered_component<T: IComponent>(&self, id: usize) -> bool {
        if !self.has_component::<T>() {
            return false;
        }

        if let Some(registered_component) = self.registered_components.get(&TypeId::of::<T>()) {
            return !registered_component.contains(id);
        }

        true
    }

    pub fn get_component<T: IComponent>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|component| (**component).as_any().downcast_ref::<T>())
    }

    pub fn get_registered_component<T: IComponent>(&mut self, id: usize) -> Option<&T> {
        if !self.has_component::<T>() {
            return None;
        }

        if let Some(registered_component) = self.registered_components.get_mut(&TypeId::of::<T>()) {
            registered_component.insert(id);
        };

        self.get_component::<T>()
    }

    pub fn push_component<T: IComponent>(&mut self, component: T) -> &mut Self {
        let boxed_component = Box::new(component);
        self.components.insert(TypeId::of::<T>(), boxed_component);
        self.registered_components
            .insert(TypeId::of::<T>(), HashSet::new());

        self
    }

    pub fn pull_component<T: IComponent>(&mut self) -> &mut Self {
        self.components.remove(&TypeId::of::<T>());
        self.registered_components.remove(&TypeId::of::<T>());

        self
    }

    pub fn check<F>(&self, predicate: F) -> EntityCheck
    where
        F: FnOnce(&Entity) -> bool,
    {
        EntityCheck(predicate(&self))
    }
}

pub enum EntitySystem<'a> {
    Reader(&'a Entity, &'a System),
    Writer(&'a mut Entity, &'a System),
}

impl<'a> EntitySystem<'a> {
    pub fn new_reader(entity: &'a Entity, system: &'a System) -> Self {
        Self::Reader(entity, system)
    }

    pub fn new_writer(entity: &'a mut Entity, system: &'a System) -> Self {
        Self::Writer(entity, system)
    }

    pub fn has_component<T: IComponent>(&self) -> bool {
        match self {
            EntitySystem::Reader(entity, system) => system.has_component::<T>(entity),
            EntitySystem::Writer(entity, system) => system.has_component::<T>(entity),
        }
    }

    pub fn get_component<T: IComponent>(&mut self) -> Option<&T> {
        match self {
            EntitySystem::Reader(entity, system) => entity.get_component::<T>(),
            EntitySystem::Writer(entity, system) => system.get_component::<T>(entity),
        }
    }

    pub fn check<F>(&self, predicate: F) -> EntityCheck
    where
        F: FnOnce(&EntitySystem) -> bool,
    {
        EntityCheck(predicate(&self))
    }
}

pub struct EntityCheck(bool);

impl EntityCheck {
    pub fn and_then<F>(&self, callback: F) -> &Self
    where
        F: FnOnce(),
    {
        if self.0 {
            callback();
        }

        self
    }
}
