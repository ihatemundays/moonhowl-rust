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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandOrder {
    Low,
    Pos(usize),
    High,
}

impl CommandOrder {
    fn rank(self) -> (u8, usize) {
        match self {
            CommandOrder::Low => (0, 0),
            CommandOrder::Pos(pos) => (1, pos),
            CommandOrder::High => (2, 0),
        }
    }
}

enum ComponentCommand {
    Set(TypeId, Box<dyn Component>, CommandOrder),
    Unset(TypeId),
}

impl ComponentCommand {
    // Unsets always rank above every Set, including an explicit CommandOrder::High,
    // so a commit never leaves a component re-set after it was meant to be removed.
    fn rank(&self) -> (u8, usize) {
        match self {
            ComponentCommand::Set(_, _, order) => order.rank(),
            ComponentCommand::Unset(_) => (3, 0),
        }
    }

    fn type_id(&self) -> TypeId {
        match self {
            ComponentCommand::Set(id, ..) => *id,
            ComponentCommand::Unset(id) => *id,
        }
    }
}

pub struct Entity {
    components: ComponentMap,
    systems: SystemMap,
    commands: Vec<ComponentCommand>,
    next_pos: usize,
    active_systems: ActiveSystemSet,
}

impl Entity {
    pub fn new() -> Self {
        Self {
            components: ComponentMap::default(),
            systems: SystemMap::default(),
            commands: Vec::new(),
            next_pos: 0,
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
        let order = self.next_order();
        self.commands
            .push(ComponentCommand::Set(TypeId::of::<T>(), Box::new(component), order));
        self
    }

    pub fn set_component_with_order<T: Component>(&mut self, component: T, order: CommandOrder) -> &mut Self {
        self.commands
            .push(ComponentCommand::Set(TypeId::of::<T>(), Box::new(component), order));
        self
    }

    pub fn unset_component<T: Component>(&mut self) -> &mut Self {
        self.commands.push(ComponentCommand::Unset(TypeId::of::<T>()));
        self
    }

    fn next_order(&mut self) -> CommandOrder {
        let pos = self.next_pos;
        self.next_pos += 1;
        CommandOrder::Pos(pos)
    }

    // Commands are already sorted by rank at this point, so for any TypeId only its
    // last command actually reaches the component map — every earlier one for that
    // same TypeId (redundant Sets, or Sets shadowed by an Unset) is a dead write.
    fn keep_last_command_per_component(&mut self) {
        let mut seen = HashSet::with_hasher(BuildHasherDefault::<FxHasher>::default());
        let mut kept: Vec<ComponentCommand> = Vec::with_capacity(self.commands.len());

        for command in self.commands.drain(..).rev() {
            if seen.insert(command.type_id()) {
                kept.push(command);
            }
        }

        kept.reverse();
        self.commands = kept;
    }

    pub fn commit(&mut self) -> &mut Self {
        let nothing_changed = self.commands.is_empty();
        self.next_pos = 0;

        self.commands.sort_by_key(|command| command.rank());
        self.keep_last_command_per_component();

        for command in self.commands.drain(..) {
            match command {
                ComponentCommand::Set(id, component, _) => {
                    self.components.insert(id, component);
                }
                ComponentCommand::Unset(id) => {
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
