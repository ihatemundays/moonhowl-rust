use crate::component::IComponent;
use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Entity {
    id: usize,
    components: HashMap<TypeId, Box<dyn IComponent>>,
    read_by: Mutex<HashMap<TypeId, HashSet<usize>>>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            id: Self::get_new_id(),
            components: HashMap::new(),
            read_by: Mutex::new(HashMap::new()),
        }
    }

    fn get_new_id() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn has_component<T: IComponent>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    pub fn is_component_read<T: IComponent>(&self, system_id: usize) -> bool {
        self.read_by
            .lock()
            .unwrap()
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
            .lock()
            .unwrap()
            .entry(TypeId::of::<T>())
            .or_default()
            .insert(system_id);

        self.components
            .get(&TypeId::of::<T>())
            .and_then(|component| (**component).as_any().downcast_ref::<T>())
    }

    pub fn set_component<T: IComponent>(&mut self, component: T) -> &mut Self {
        let boxed_component = Box::new(component);
        self.components.insert(TypeId::of::<T>(), boxed_component);
        self.read_by
            .get_mut()
            .unwrap()
            .insert(TypeId::of::<T>(), HashSet::new());

        self
    }

    pub fn unset_component<T: IComponent>(&mut self) -> &mut Self {
        self.components.remove(&TypeId::of::<T>());
        self.read_by.get_mut().unwrap().remove(&TypeId::of::<T>());

        self
    }

    pub fn has_some_components<T: ComponentSet>(&self) -> bool {
        T::has_some(self)
    }

    pub fn has_every_component<T: ComponentSet>(&self) -> bool {
        T::has_every(self)
    }

    pub fn has_some_read_components<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_some_read(self, system_id)
    }

    pub fn has_every_read_component<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_every_read(self, system_id)
    }

    pub fn has_some_unread_components<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_some_unread(self, system_id)
    }

    pub fn has_every_unread_component<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_every_unread(self, system_id)
    }
}

pub trait ComponentSet {
    fn has_some(entity: &Entity) -> bool;
    fn has_every(entity: &Entity) -> bool;
    fn has_some_read(entity: &Entity, system_id: usize) -> bool;
    fn has_every_read(entity: &Entity, system_id: usize) -> bool;
    fn has_some_unread(entity: &Entity, system_id: usize) -> bool;
    fn has_every_unread(entity: &Entity, system_id: usize) -> bool;
}

macro_rules! impl_component_set {
    ($($t:ident),+) => {
        impl<$($t: IComponent),+> ComponentSet for ($($t,)+) {
            fn has_some(entity: &Entity) -> bool {
                $(entity.has_component::<$t>())||+
            }

            fn has_every(entity: &Entity) -> bool {
                $(entity.has_component::<$t>())&&+
            }

            fn has_some_read(entity: &Entity, system_id: usize) -> bool {
                $(entity.has_read_component::<$t>(system_id))||+
            }

            fn has_every_read(entity: &Entity, system_id: usize) -> bool {
                $(entity.has_read_component::<$t>(system_id))&&+
            }

            fn has_some_unread(entity: &Entity, system_id: usize) -> bool {
                $(entity.has_unread_component::<$t>(system_id))||+
            }

            fn has_every_unread(entity: &Entity, system_id: usize) -> bool {
                $(entity.has_unread_component::<$t>(system_id))&&+
            }
        }
    };
}

impl_component_set!(A);
impl_component_set!(A, B);
impl_component_set!(A, B, C);
impl_component_set!(A, B, C, D);
impl_component_set!(A, B, C, D, E);
impl_component_set!(A, B, C, D, E, F);
impl_component_set!(A, B, C, D, E, F, G);
impl_component_set!(A, B, C, D, E, F, G, H);
