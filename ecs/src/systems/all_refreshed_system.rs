use crate::systems::address_archetype::AddressArchetype;
use std::cell::Cell;
use crate::{Entity, System};

pub struct AllRefreshedSystem<A: AddressArchetype> {
    seen: Cell<Option<A::Addresses>>,
}

impl<A: AddressArchetype> AllRefreshedSystem<A> {
    pub fn new() -> Self {
        Self {
            seen: Cell::new(None),
        }
    }
}

impl<A: AddressArchetype> Default for AllRefreshedSystem<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: AddressArchetype + 'static> System for AllRefreshedSystem<A> {
    fn is_lazy(&self) -> bool {
        false
    }

    fn test(&self, entity: &Entity) -> bool {
        let Some(current) = A::addresses(entity) else {
            return false;
        };

        let previous = self.seen.get();
        self.seen.set(Some(current));

        match previous {
            None => true,
            Some(previous) => !A::any_repeats(current, previous),
        }
    }
}
