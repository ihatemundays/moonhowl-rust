use crate::component::IComponent;
use crate::system::System;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Entity {
    id: usize,
    components: HashMap<TypeId, Box<dyn IComponent>>,
    read_by: Mutex<HashMap<TypeId, HashSet<usize>>>,
    context: Option<Box<dyn Any + Send>>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            id: Self::get_new_id(),
            components: HashMap::new(),
            read_by: Mutex::new(HashMap::new()),
            context: None,
        }
    }

    fn get_new_id() -> usize {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    fn has_component<T: IComponent>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    fn has_some_components<T: ComponentSet>(&self) -> bool {
        T::has_some(self)
    }

    fn has_every_component<T: ComponentSet>(&self) -> bool {
        T::has_every(self)
    }

    fn is_component_read<T: IComponent>(&self, system_id: usize) -> bool {
        self.read_by
            .lock()
            .unwrap()
            .get(&TypeId::of::<T>())
            .is_some_and(|readers| readers.contains(&system_id))
    }

    fn has_read_component<T: IComponent>(&self, system_id: usize) -> bool {
        self.has_component::<T>() && self.is_component_read::<T>(system_id)
    }

    fn has_some_read_components<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_some_read(self, system_id)
    }

    fn has_every_read_component<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_every_read(self, system_id)
    }

    fn has_unread_component<T: IComponent>(&self, system_id: usize) -> bool {
        self.has_component::<T>() && !self.is_component_read::<T>(system_id)
    }

    fn has_some_unread_components<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_some_unread(self, system_id)
    }

    fn has_every_unread_component<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_every_unread(self, system_id)
    }

    fn read_component<T: IComponent>(&self, system_id: usize) -> Option<&T> {
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

    fn read_components<T: ComponentSet>(&self, system_id: usize) -> Option<T::Refs<'_>> {
        T::read_every(self, system_id)
    }

    pub fn get_component<T: IComponent>(&self) -> Option<&T> {
        if !self.has_component::<T>() {
            return None;
        }

        self.components
            .get(&TypeId::of::<T>())
            .and_then(|component| (**component).as_any().downcast_ref::<T>())
    }

    pub fn get_components<T: ComponentSet>(&self) -> Option<T::Refs<'_>> {
        T::get_every(self)
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

    pub fn set_context<C: Any + Send>(&mut self, context: C) -> &mut Self {
        self.context = Some(Box::new(context));
        self
    }

    pub fn get_context<C: Any + Send>(&self) -> Option<&C> {
        self.context.as_deref()?.downcast_ref::<C>()
    }

    pub fn get_context_mut<C: Any + Send>(&mut self) -> Option<&mut C> {
        self.context.as_deref_mut()?.downcast_mut::<C>()
    }

    pub fn clear_context(&mut self) -> &mut Self {
        self.context = None;
        self
    }
}

pub struct CheckContext<'a>(&'a System);

impl<'a> CheckContext<'a> {
    pub(crate) fn new(system: &'a System) -> Self {
        Self(system)
    }

    pub fn get_id(&self) -> usize {
        self.0.get_id()
    }

    pub fn has_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_component::<T>()
    }

    pub fn is_component_read<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.is_component_read::<T>(self.0.get_id())
    }

    pub fn has_read_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_read_component::<T>(self.0.get_id())
    }

    pub fn has_unread_component<T: IComponent>(&self, entity: &Entity) -> bool {
        entity.has_unread_component::<T>(self.0.get_id())
    }

    pub fn get_component<'e, T: IComponent>(&self, entity: &'e Entity) -> Option<&'e T> {
        entity.get_component()
    }

    pub fn get_components<'e, T: ComponentSet>(&self, entity: &'e Entity) -> Option<T::Refs<'e>> {
        entity.get_components::<T>()
    }

    pub fn has_some_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_components::<T>()
    }

    pub fn has_every_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_component::<T>()
    }

    pub fn has_some_read_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_read_components::<T>(self.0.get_id())
    }

    pub fn has_every_read_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_read_component::<T>(self.0.get_id())
    }

    pub fn has_some_unread_components<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_some_unread_components::<T>(self.0.get_id())
    }

    pub fn has_every_unread_component<T: ComponentSet>(&self, entity: &Entity) -> bool {
        entity.has_every_unread_component::<T>(self.0.get_id())
    }
}

pub struct ActionContext<'a>(&'a System);

impl<'a> ActionContext<'a> {
    pub(crate) fn new(system: &'a System) -> Self {
        Self(system)
    }

    pub fn get_id(&self) -> usize {
        self.0.get_id()
    }

    pub fn get_component<'e, T: IComponent>(&self, entity: &'e Entity) -> Option<&'e T> {
        entity.get_component()
    }

    pub fn get_components<'e, T: ComponentSet>(&self, entity: &'e Entity) -> Option<T::Refs<'e>> {
        entity.get_components::<T>()
    }

    pub fn read_component<'e, T: IComponent>(&self, entity: &'e Entity) -> Option<&'e T> {
        entity.read_component::<T>(self.0.get_id())
    }

    pub fn read_components<'e, T: ComponentSet>(&self, entity: &'e Entity) -> Option<T::Refs<'e>> {
        entity.read_components::<T>(self.0.get_id())
    }
}

pub trait ComponentSet {
    type Refs<'a>;

    fn get_every<'a>(entity: &'a Entity) -> Option<Self::Refs<'a>>;
    fn read_every<'a>(entity: &'a Entity, system_id: usize) -> Option<Self::Refs<'a>>;
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
            type Refs<'a> = ($(&'a $t,)+);

            fn get_every<'a>(entity: &'a Entity) -> Option<Self::Refs<'a>> {
                if !Self::has_every(entity) {
                    return None;
                }

                Some(($(entity.get_component::<$t>().unwrap(),)+))
            }

            fn read_every<'a>(entity: &'a Entity, system_id: usize) -> Option<Self::Refs<'a>> {
                if !Self::has_every(entity) {
                    return None;
                }

                Some(($(entity.read_component::<$t>(system_id).unwrap(),)+))
            }

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
