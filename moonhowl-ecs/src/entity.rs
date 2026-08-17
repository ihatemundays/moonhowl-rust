use crate::archetype::Archetype;
use crate::component::Component;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
struct FxHasher(u64);

impl Hasher for FxHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    
    fn write(&mut self, bytes: &[u8]) {
        const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        for chunk in bytes.chunks(8) {
            let mut buf = [0u8; 8];
            buf[..chunk.len()].copy_from_slice(chunk);
            self.0 = (self.0.rotate_left(5) ^ u64::from_ne_bytes(buf)).wrapping_mul(SEED);
        }
    }
}

type ComponentMap = HashMap<TypeId, Box<dyn Component>, BuildHasherDefault<FxHasher>>;

pub struct Entity {
    components: ComponentMap,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            components: ComponentMap::default(),
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

    pub fn edit_component<T: Component>(&mut self, f: impl FnOnce(&mut T)) -> &mut Self {
        if let Some(component) = self.get_component_mut::<T>() {
            f(component);
        }
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
