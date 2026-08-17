use crate::archetype::Archetype;
use crate::component::Component;
use std::any::{Any, TypeId};
use std::collections::HashMap;

pub struct Entity {
    components: HashMap<TypeId, Box<dyn Component>>,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn has_component<T: Component>(&self) -> bool {
        self.components.contains_key(&TypeId::of::<T>())
    }

    pub fn get_component<T: Component>(&self) -> Option<&T> {
        self.components
            .get(&TypeId::of::<T>())
            .and_then(|component| (component.as_ref() as &dyn Any).downcast_ref::<T>())
    }

    pub fn get_component_mut<T: Component>(&mut self) -> Option<&mut T> {
        self.components
            .get_mut(&TypeId::of::<T>())
            .and_then(|component| (component.as_mut() as &mut dyn Any).downcast_mut::<T>())
    }

    pub(crate) fn get_components_mut<const N: usize>(
        &mut self,
        ids: [TypeId; N],
    ) -> [Option<&mut Box<dyn Component>>; N] {
        self.components.get_disjoint_mut(ids.each_ref())
    }

    pub fn set_component<T: Component>(&mut self, component: T) -> &mut Self {
        self.components.insert(TypeId::of::<T>(), Box::new(component));
        self
    }

    pub fn unset_component<T: Component>(&mut self) -> &mut Self {
        self.components.remove(&TypeId::of::<T>());
        self
    }

    pub fn with_archetype<A: Archetype, R>(&self, f: impl FnOnce(A::Ref<'_>) -> R) -> Option<R> {
        A::fetch(self).map(f)
    }

    pub fn with_archetype_mut<A: Archetype, R>(
        &mut self,
        f: impl FnOnce(A::RefMut<'_>) -> R,
    ) -> Option<R> {
        A::fetch_mut(self).map(f)
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}
