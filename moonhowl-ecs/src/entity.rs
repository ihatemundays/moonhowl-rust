use crate::component::IComponent;
use crate::system::System;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A component plus the set of systems that have read it, kept together so
/// the two can never drift out of sync the way two parallel maps could, and
/// so a lookup by `TypeId` only has to happen once.
struct ComponentEntry {
    component: Box<dyn IComponent>,
    read_by: Mutex<HashSet<usize>>,
}

pub struct Entity {
    id: usize,
    components: HashMap<TypeId, ComponentEntry>,
    context: Option<Box<dyn Any + Send>>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            id: Self::get_new_id(),
            components: HashMap::new(),
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
        self.components
            .get(&TypeId::of::<T>())
            .is_some_and(|entry| entry.read_by.lock().unwrap().contains(&system_id))
    }

    fn has_read_component<T: IComponent>(&self, system_id: usize) -> bool {
        // is_component_read already reads false for an absent component
        // (no entry, no read set), so the has_component check that used to
        // gate this is redundant.
        self.is_component_read::<T>(system_id)
    }

    fn has_some_read_components<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_some_read(self, system_id)
    }

    fn has_every_read_component<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_every_read(self, system_id)
    }

    fn has_unread_component<T: IComponent>(&self, system_id: usize) -> bool {
        self.components
            .get(&TypeId::of::<T>())
            .is_some_and(|entry| !entry.read_by.lock().unwrap().contains(&system_id))
    }

    fn has_some_unread_components<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_some_unread(self, system_id)
    }

    fn has_every_unread_component<T: ComponentSet>(&self, system_id: usize) -> bool {
        T::has_every_unread(self, system_id)
    }

    fn read_component<T: IComponent>(&self, system_id: usize) -> Option<&T> {
        let entry = self.components.get(&TypeId::of::<T>())?;
        let component = entry.component.as_any().downcast_ref::<T>();

        if component.is_some() {
            entry.read_by.lock().unwrap().insert(system_id);
        }

        component
    }

    fn read_components<T: ComponentSet>(&self, system_id: usize) -> Option<T::Refs<'_>> {
        T::read_every(self, system_id)
    }

    pub fn get_component<T: IComponent>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|entry| entry.component.as_any().downcast_ref::<T>())
    }

    pub fn get_components<T: ComponentSet>(&self) -> Option<T::Refs<'_>> {
        T::get_every(self)
    }

    pub fn set_component<T: IComponent>(&mut self, component: T) -> &mut Self {
        self.components.insert(
            TypeId::of::<T>(),
            ComponentEntry {
                component: Box::new(component),
                read_by: Mutex::new(HashSet::new()),
            },
        );
        self
    }

    pub fn unset_component<T: IComponent>(&mut self) -> &mut Self {
        self.components.remove(&TypeId::of::<T>());
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
