use crate::archetype::Archetype;
use crate::component::Component;
use crate::system::System;
use std::any::{Any, TypeId};
use std::collections::{HashMap, HashSet};
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
type SystemMap = HashMap<TypeId, Box<dyn System>, BuildHasherDefault<FxHasher>>;
type ActiveSystemSet = HashSet<TypeId, BuildHasherDefault<FxHasher>>;

enum ComponentCommand {
    Set(TypeId, Box<dyn Component>),
    Unset(TypeId),
}

pub struct Entity {
    components: ComponentMap,
    systems: SystemMap,
    commands: Vec<ComponentCommand>,
    active_systems: ActiveSystemSet,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            components: ComponentMap::default(),
            systems: SystemMap::default(),
            commands: Vec::new(),
            active_systems: ActiveSystemSet::default(),
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

    pub fn set_component<T: Component>(&mut self, component: T) -> &mut Self {
        self.commands
            .push(ComponentCommand::Set(TypeId::of::<T>(), Box::new(component)));
        self
    }

    pub fn unset_component<T: Component>(&mut self) -> &mut Self {
        self.commands.push(ComponentCommand::Unset(TypeId::of::<T>()));
        self
    }

    pub fn commit(&mut self) -> &mut Self {
        for command in self.commands.drain(..) {
            match command {
                ComponentCommand::Set(id, component) => {
                    self.components.insert(id, component);
                }
                ComponentCommand::Unset(id) => {
                    self.components.remove(&id);
                }
            }
        }

        for (id, system) in self.systems.iter() {
            if system.test(self) {
                self.active_systems.insert(*id);
            } else {
                self.active_systems.remove(id);
            }
        }

        self
    }

    pub fn is_system_active<T: System>(&self) -> bool {
        self.active_systems.contains(&TypeId::of::<T>())
    }

    pub fn bind_system<T: System>(&mut self, system: T) -> &mut Self {
        self.systems.insert(TypeId::of::<T>(), Box::new(system));
        self
    }

    pub fn unbind_system<T: System>(&mut self) -> &mut Self {
        self.systems.remove(&TypeId::of::<T>());
        self.active_systems.remove(&TypeId::of::<T>());
        self
    }

    pub fn with_archetype<A: Archetype, R>(&self, f: impl FnOnce(A::Ref<'_>) -> R) -> Option<R> {
        A::fetch(self).map(f)
    }

    pub fn with_system_archetype<S: System, A: Archetype, R>(&self, f: impl FnOnce(A::Ref<'_>) -> R) -> Option<R> {
        if self.is_system_active::<S>() {
            return self.with_archetype::<A, R>(f)
        }
        None
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}
