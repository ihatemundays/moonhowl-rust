use crate::archetype::Archetype;
use crate::component::Component;
use crate::entity::Entity;
use crate::table::{ArchetypeRegistry, FxBuildHasher, TableId, migrate_shared_columns};
use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::thread;

#[derive(Clone, Copy)]
pub(crate) struct EntityLocation {
    pub(crate) table: TableId,
    pub(crate) row: u32,
}

/// Which tables currently match an `Archetype` query, cached per archetype
/// type. Tables are only ever created, never removed, and a table's column
/// set never changes once created, so this only ever needs to scan the
/// tables added since it was last consulted.
#[derive(Default)]
struct QueryCache {
    tables_scanned: usize,
    matching: Vec<TableId>,
}

pub struct World {
    generations: Vec<u32>,
    free_indices: Vec<u32>,
    locations: Vec<EntityLocation>,
    registry: ArchetypeRegistry,
    query_cache: RefCell<HashMap<TypeId, QueryCache, FxBuildHasher>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            generations: Vec::new(),
            free_indices: Vec::new(),
            locations: Vec::new(),
            registry: ArchetypeRegistry::new(),
            query_cache: RefCell::new(HashMap::default()),
        }
    }

    pub fn spawn(&mut self) -> Entity {
        let entity = if let Some(index) = self.free_indices.pop() {
            Entity::new(index, self.generations[index as usize])
        } else {
            let index = self.generations.len() as u32;
            self.generations.push(0);
            Entity::new(index, 0)
        };

        let root = self.registry.root_table();
        let row = self.registry.table_mut(root).push_entity(entity);
        let location = EntityLocation { table: root, row };

        let index = entity.index() as usize;
        if index < self.locations.len() {
            self.locations[index] = location;
        } else {
            self.locations.push(location);
        }

        entity
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        let Some(loc) = self.location(entity) else { return false };
        let table = self.registry.table_mut(loc.table);
        let row = loc.row as usize;
        for col in table.columns_mut().values_mut() {
            col.swap_remove_drop(row);
        }
        if let Some(moved) = table.swap_remove_entity(row) {
            self.locations[moved.index() as usize].row = row as u32;
        }

        self.generations[entity.index() as usize] += 1;
        self.free_indices.push(entity.index());
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations.get(entity.index() as usize) == Some(&entity.generation())
    }

    pub fn len(&self) -> usize {
        self.generations.len() - self.free_indices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_component<T: Component>(&mut self, entity: Entity, component: T) -> &mut Self {
        let Some(loc) = self.location(entity) else { return self };
        let type_id = TypeId::of::<T>();
        if self.registry.table(loc.table).has_column(type_id) {
            // Already present: overwrite in place, no migration.
            self.registry.table_mut(loc.table).set_value::<T>(loc.row as usize, component);
            return self;
        }

        let dst_id = self.registry.add_edge::<T>(loc.table);
        self.move_entity(entity, loc.table, dst_id, loc.row as usize);
        // T's column exists in dst but wasn't in src, so migrate_shared_columns (inside
        // move_entity) left it untouched -- every other dst column and dst's entity list
        // are now one row longer, but T's column is still one short. Pushing here brings
        // it back into row alignment with the rest of the table.
        self.registry.table_mut(dst_id).push_column::<T>(component);
        self
    }

    pub fn unset_component<T: Component>(&mut self, entity: Entity) -> &mut Self {
        let Some(loc) = self.location(entity) else { return self };
        let type_id = TypeId::of::<T>();
        if !self.registry.table(loc.table).has_column(type_id) {
            return self;
        }

        let dst_id = self.registry.remove_edge::<T>(loc.table);
        self.move_entity(entity, loc.table, dst_id, loc.row as usize);
        // dst has strictly fewer columns than src; migrate_shared_columns already dropped
        // T's value on the way. Every dst column is already aligned -- nothing else to do.
        self
    }

    pub fn has_component<T: Component>(&self, entity: Entity) -> bool {
        self.location(entity).is_some_and(|loc| self.registry.table(loc.table).has_column(TypeId::of::<T>()))
    }

    pub fn get<A: Archetype>(&self, entity: Entity) -> Option<A::Ref<'_>> {
        A::fetch(self, entity)
    }

    pub fn get_mut<A: Archetype>(&mut self, entity: Entity) -> Option<A::RefMut<'_>> {
        A::fetch_mut(self, entity)
    }

    pub fn with_archetype<A: Archetype>(&self, mut f: impl FnMut(A::Ref<'_>, Entity)) {
        for table_id in self.matching_tables::<A>() {
            let table = self.registry.table(table_id);
            let Some(columns) = A::resolve_columns(table.columns()) else { continue };
            for (row, &entity) in table.entities().iter().enumerate() {
                f(A::row(&columns, row), entity);
            }
        }
    }

    pub fn with_archetype_mut<A: Archetype>(&mut self, mut f: impl FnMut(A::RefMut<'_>, Entity)) {
        for table_id in self.matching_tables::<A>() {
            let table = self.registry.table_mut(table_id);
            let (entities, columns) = table.split_mut();
            let Some(mut resolved) = A::resolve_columns_mut(columns) else { continue };
            for (row, &entity) in entities.iter().enumerate() {
                f(A::row_mut(&mut resolved, row), entity);
            }
        }
    }

    pub fn with_archetype_async<A>(&self, f: impl Fn(A::Ref<'_>, Entity) + Sync)
    where
        A: Archetype,
        for<'a> A::Columns<'a>: Sync,
    {
        let f = &f;
        for table_id in self.matching_tables::<A>() {
            let table = self.registry.table(table_id);
            let Some(columns) = A::resolve_columns(table.columns()) else { continue };
            let columns = &columns;
            thread::scope(|scope| {
                for range in row_chunks(table.len()) {
                    scope.spawn(move || {
                        for row in range {
                            f(A::row(columns, row), table.entities()[row]);
                        }
                    });
                }
            });
        }
    }

    pub fn with_archetype_async_mut<A>(&mut self, f: impl Fn(A::RefMut<'_>, Entity) + Sync)
    where
        A: Archetype,
        for<'a> A::ColumnsMut<'a>: Send,
    {
        let f = &f;
        for table_id in self.matching_tables::<A>() {
            let table = self.registry.table_mut(table_id);
            let (entities, columns) = table.split_mut();
            let Some(resolved) = A::resolve_columns_mut(columns) else { continue };
            thread::scope(|scope| {
                let mut remaining = resolved;
                for range in row_chunks(entities.len()) {
                    let chunk_entities = &entities[range.clone()];
                    let (chunk, rest) = A::split_columns_mut(remaining, range.len());
                    remaining = rest;
                    scope.spawn(move || {
                        let mut chunk = chunk;
                        for (i, &entity) in chunk_entities.iter().enumerate() {
                            f(A::row_mut(&mut chunk, i), entity);
                        }
                    });
                }
            });
        }
    }

    pub(crate) fn location(&self, entity: Entity) -> Option<EntityLocation> {
        self.is_alive(entity).then(|| self.locations[entity.index() as usize])
    }

    pub(crate) fn component<T: Component>(&self, entity: Entity) -> Option<&T> {
        let loc = self.location(entity)?;
        self.registry.table(loc.table).column::<T>()?.get(loc.row as usize)
    }

    pub(crate) fn component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let loc = self.location(entity)?;
        self.registry.table_mut(loc.table).column_mut::<T>()?.get_mut(loc.row as usize)
    }

    pub(crate) fn registry_mut(&mut self) -> &mut ArchetypeRegistry {
        &mut self.registry
    }

    fn move_entity(&mut self, entity: Entity, src_id: TableId, dst_id: TableId, row: usize) {
        let [src, dst] = self.registry.two_tables_mut(src_id, dst_id);
        migrate_shared_columns(src, dst, row);

        if let Some(moved) = src.swap_remove_entity(row) {
            self.locations[moved.index() as usize].row = row as u32;
        }

        let new_row = dst.push_entity(entity);
        self.locations[entity.index() as usize] = EntityLocation { table: dst_id, row: new_row };
    }

    /// The tables currently matching `A`: every table whose column set is a
    /// superset of `A::type_ids()`. Cached per archetype type and only
    /// rescanned over tables created since the cache was last built.
    fn matching_tables<A: Archetype>(&self) -> Vec<TableId> {
        let type_ids = A::type_ids();
        let mut cache = self.query_cache.borrow_mut();
        let entry = cache.entry(TypeId::of::<A>()).or_default();
        let total = self.registry.table_count();
        if entry.tables_scanned < total {
            for (id, table) in self.registry.tables_from(entry.tables_scanned) {
                if type_ids.iter().all(|ty| table.has_column(*ty)) {
                    entry.matching.push(id);
                }
            }
            entry.tables_scanned = total;
        }
        entry.matching.clone()
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

fn chunk_size(len: usize) -> usize {
    let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    len.div_ceil(threads).max(1)
}

fn row_chunks(len: usize) -> impl Iterator<Item = Range<usize>> {
    let size = chunk_size(len);
    (0..len).step_by(size).map(move |start| start..(start + size).min(len))
}
