use crate::archetype::Archetype;
use crate::component::Component;
use crate::entity::Entity;

pub trait AddressArchetype: Archetype {
    type Addresses: Copy;

    fn addresses(entity: &Entity) -> Option<Self::Addresses>;
    fn any_repeats(current: Self::Addresses, previous: Self::Addresses) -> bool;
}

impl<T: Component> AddressArchetype for T {
    type Addresses = *const T;

    fn addresses(entity: &Entity) -> Option<Self::Addresses> {
        entity.get_component::<T>().map(|component| component as *const T)
    }

    fn any_repeats(current: Self::Addresses, previous: Self::Addresses) -> bool {
        current == previous
    }
}

macro_rules! impl_address_archetype_for_tuple {
    ($(($t:ident, $cur:ident, $prev:ident)),+) => {
        impl<$($t: Component),+> AddressArchetype for ($($t,)+) {
            type Addresses = ($(*const $t,)+);

            fn addresses(entity: &Entity) -> Option<Self::Addresses> {
                Some(($(entity.get_component::<$t>()? as *const $t,)+))
            }

            fn any_repeats(current: Self::Addresses, previous: Self::Addresses) -> bool {
                let ($($cur,)+) = current;
                let ($($prev,)+) = previous;
                $($cur == $prev)||+
            }
        }
    };
}

impl_address_archetype_for_tuple!((A, a1, a2), (B, b1, b2));
impl_address_archetype_for_tuple!((A, a1, a2), (B, b1, b2), (C, c1, c2));
impl_address_archetype_for_tuple!((A, a1, a2), (B, b1, b2), (C, c1, c2), (D, d1, d2));
impl_address_archetype_for_tuple!((A, a1, a2), (B, b1, b2), (C, c1, c2), (D, d1, d2), (E, e1, e2));
impl_address_archetype_for_tuple!((A, a1, a2), (B, b1, b2), (C, c1, c2), (D, d1, d2), (E, e1, e2), (F, f1, f2));
