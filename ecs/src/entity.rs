use crate::archetype::Archetype;
use crate::component::Component;
use crate::system::System;
use std::any::{Any, TypeId};
use std::collections::hash_map::Entry;
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
type CommandMap = HashMap<TypeId, ComponentCommand, BuildHasherDefault<FxHasher>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOrder {
    Lowest,
    Lower(usize),
    Low,
    Medium,
    High,
    Higher(usize),
    Highest,
}

impl CommandOrder {
    // Lower(x) and Higher(x) both read as "x steps away from the fixed tier next to
    // them" — Higher(x) is x steps above High (bigger x applies later, closer to
    // Highest), so Lower(x) is x steps below Low (bigger x applies earlier, closer
    // to Lowest); the two variants therefore rank oppositely against their payload.
    fn rank(self) -> (u8, usize) {
        match self {
            CommandOrder::Lowest => (0, 0),
            CommandOrder::Lower(pos) => (1, usize::MAX - pos),
            CommandOrder::Low => (2, 0),
            CommandOrder::Medium => (3, 0),
            CommandOrder::High => (4, 0),
            CommandOrder::Higher(pos) => (5, pos),
            CommandOrder::Highest => (6, 0),
        }
    }
}

enum ComponentCommand {
    Set(Box<dyn Component>, CommandOrder),
    Unset(CommandOrder),
}

impl ComponentCommand {
    fn rank(&self) -> (u8, usize) {
        match self {
            ComponentCommand::Set(_, order) => order.rank(),
            ComponentCommand::Unset(order) => order.rank(),
        }
    }
}

pub struct Entity {
    components: ComponentMap,
    systems: SystemMap,
    commands: CommandMap,
    active_systems: ActiveSystemSet,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            components: ComponentMap::default(),
            systems: SystemMap::default(),
            commands: CommandMap::default(),
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
        self.set_component_with_order(component, CommandOrder::Medium)
    }

    pub fn set_component_with_order<T: Component>(&mut self, component: T, order: CommandOrder) -> &mut Self {
        self.queue(TypeId::of::<T>(), order, || ComponentCommand::Set(Box::new(component), order))
    }

    pub fn unset_component<T: Component>(&mut self) -> &mut Self {
        self.unset_component_with_order::<T>(CommandOrder::Medium)
    }

    pub fn unset_component_with_order<T: Component>(&mut self, order: CommandOrder) -> &mut Self {
        self.queue(TypeId::of::<T>(), order, || ComponentCommand::Unset(order))
    }

    // At most one command per component type is ever queued: a new command for a
    // type that's already pending is ranked against the pending one before `build`
    // ever runs, so a `Set` that loses the rank check never pays for its `Box`
    // allocation — only a command that will actually be kept gets built at all.
    fn queue(&mut self, id: TypeId, order: CommandOrder, build: impl FnOnce() -> ComponentCommand) -> &mut Self {
        match self.commands.entry(id) {
            Entry::Occupied(mut occupied) => {
                if order.rank() >= occupied.get().rank() {
                    occupied.insert(build());
                }
            }
            Entry::Vacant(vacant) => {
                vacant.insert(build());
            }
        }
        self
    }

    pub fn commit(&mut self) -> &mut Self {
        let nothing_changed = self.commands.is_empty();

        for (id, command) in self.commands.drain() {
            match command {
                ComponentCommand::Set(component, _) => {
                    self.components.insert(id, component);
                }
                ComponentCommand::Unset(_) => {
                    self.components.remove(&id);
                }
            }
        }

        for (id, system) in self.systems.iter() {
            if nothing_changed && system.is_lazy() {
                continue;
            }

            if system.test(self) {
                self.active_systems.insert(*id);
            } else {
                self.active_systems.remove(id);
            }
        }

        self
    }

    pub fn reset(&mut self) -> &mut Self {
        self.commands.clear();
        self.commit()
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

    pub fn has_archetype<A: Archetype>(&self) -> bool {
        A::has(self)
    }

    pub fn with_archetype<A: Archetype>(&self) -> Option<A::Ref<'_>> {
        A::fetch(self)
    }

    pub fn with_system_archetype<S: System, A: Archetype>(&self) -> Option<A::Ref<'_>> {
        if self.is_system_active::<S>() {
            return self.with_archetype::<A>();
        }
        None
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}
