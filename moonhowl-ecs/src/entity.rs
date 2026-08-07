use crate::component::IComponent;
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

pub struct Entity {
    components: HashMap<TypeId, Box<dyn IComponent>>,
    read_by: RefCell<HashMap<TypeId, HashSet<usize>>>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            read_by: RefCell::new(HashMap::new()),
        }
    }

    pub fn has_component<T: IComponent>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    pub fn is_component_read<T: IComponent>(&self, system_id: usize) -> bool {
        self.read_by
            .borrow()
            .get(&TypeId::of::<T>())
            .is_some_and(|readers| readers.contains(&system_id))
    }

    pub fn has_read_component<T: IComponent>(&self, system_id: usize) -> bool {
        self.has_component::<T>() && self.is_component_read::<T>(system_id)
    }

    pub fn has_unread_component<T: IComponent>(&self, system_id: usize) -> bool {
        self.has_component::<T>() && !self.is_component_read::<T>(system_id)
    }

    pub fn get_component<T: IComponent>(&self, system_id: usize) -> Option<&T> {
        if !self.has_component::<T>() {
            return None;
        }

        self.read_by
            .borrow_mut()
            .entry(TypeId::of::<T>())
            .or_default()
            .insert(system_id);

        self.components
            .get(&TypeId::of::<T>())
            .and_then(|component| (**component).as_any().downcast_ref::<T>())
    }

    pub fn insert_component<T: IComponent>(&mut self, component: T) -> &mut Self {
        let boxed_component = Box::new(component);
        self.components.insert(TypeId::of::<T>(), boxed_component);
        self.read_by
            .get_mut()
            .insert(TypeId::of::<T>(), HashSet::new());

        self
    }

    pub fn remove_component<T: IComponent>(&mut self) -> &mut Self {
        self.components.remove(&TypeId::of::<T>());
        self.read_by.get_mut().remove(&TypeId::of::<T>());

        self
    }
}
