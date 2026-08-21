use crate::component::Component;
use crate::entity::Entity;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default)]
pub struct FxHasher(u64);

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

pub type FxBuildHasher = BuildHasherDefault<FxHasher>;

/// A single component's storage within one `Table`: a type-erased `Vec<T>`.
/// Every operation either moves a value (never clones it) or drops it, so
/// migrating a row between tables never requires `T: Clone`.
pub trait Column: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn swap_remove_drop(&mut self, row: usize);
    fn swap_remove_to(&mut self, row: usize, dst: &mut dyn Column);
    fn new_empty(&self) -> Box<dyn Column>;
}

impl<T: Component> Column for Vec<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn swap_remove_drop(&mut self, row: usize) {
        self.swap_remove(row);
    }

    fn swap_remove_to(&mut self, row: usize, dst: &mut dyn Column) {
        let value = self.swap_remove(row);
        dst.as_any_mut().downcast_mut::<Vec<T>>().expect("column type mismatch").push(value);
    }

    fn new_empty(&self) -> Box<dyn Column> {
        Box::new(Vec::<T>::new())
    }
}

pub type ColumnMap = HashMap<TypeId, Box<dyn Column>, FxBuildHasher>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct TableId(u32);

#[derive(Default)]
struct TableEdges {
    add: HashMap<TypeId, TableId, FxBuildHasher>,
    remove: HashMap<TypeId, TableId, FxBuildHasher>,
}

pub struct Table {
    entities: Vec<Entity>,
    columns: ColumnMap,
    edges: TableEdges,
}

impl Table {
    fn empty() -> Self {
        Self { entities: Vec::new(), columns: ColumnMap::default(), edges: TableEdges::default() }
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn entities(&self) -> &[Entity] {
        &self.entities
    }

    pub fn push_entity(&mut self, entity: Entity) -> u32 {
        self.entities.push(entity);
        (self.entities.len() - 1) as u32
    }

    pub fn swap_remove_entity(&mut self, row: usize) -> Option<Entity> {
        self.entities.swap_remove(row);
        self.entities.get(row).copied()
    }

    pub fn has_column(&self, type_id: TypeId) -> bool {
        self.columns.contains_key(&type_id)
    }

    pub fn columns(&self) -> &ColumnMap {
        &self.columns
    }

    pub fn columns_mut(&mut self) -> &mut ColumnMap {
        &mut self.columns
    }

    /// Entities and columns as two disjoint borrows -- lets a caller read
    /// row-to-entity mappings while independently resolving/mutating
    /// columns, without the two competing for the same `&mut self`.
    pub fn split_mut(&mut self) -> (&[Entity], &mut ColumnMap) {
        (&self.entities, &mut self.columns)
    }

    pub fn column<T: Component>(&self) -> Option<&Vec<T>> {
        self.columns.get(&TypeId::of::<T>())?.as_any().downcast_ref()
    }

    pub fn column_mut<T: Component>(&mut self) -> Option<&mut Vec<T>> {
        self.columns.get_mut(&TypeId::of::<T>())?.as_any_mut().downcast_mut()
    }

    pub fn push_column<T: Component>(&mut self, value: T) {
        self.columns
            .get_mut(&TypeId::of::<T>())
            .expect("destination table missing column for T")
            .as_any_mut()
            .downcast_mut::<Vec<T>>()
            .expect("column type mismatch")
            .push(value);
    }

    pub fn set_value<T: Component>(&mut self, row: usize, value: T) {
        self.column_mut::<T>().expect("missing column")[row] = value;
    }
}

/// Moves row `row` of `src` into a fresh row of `dst`, for every component
/// type the two tables have in common; drops any value whose column exists
/// only in `src` (the component being removed, on an `unset_component`
/// transition). Does not touch `entities` on either table -- the caller
/// (`World::move_entity`) is responsible for that and for keeping
/// `World::locations` in sync.
pub fn migrate_shared_columns(src: &mut Table, dst: &mut Table, row: usize) {
    for (type_id, src_col) in src.columns.iter_mut() {
        match dst.columns.get_mut(type_id) {
            Some(dst_col) => src_col.swap_remove_to(row, dst_col.as_mut()),
            None => src_col.swap_remove_drop(row),
        }
    }
}

/// One table per unique, exact component set an entity has ever had (an
/// "archetype"). Tables are created lazily and never removed, so a `TableId`
/// is valid for the lifetime of the `World` that produced it.
pub struct ArchetypeRegistry {
    tables: Vec<Table>,
    by_key: HashMap<Box<[TypeId]>, TableId, FxBuildHasher>,
}

impl ArchetypeRegistry {
    pub fn new() -> Self {
        let mut by_key = HashMap::default();
        by_key.insert(Box::from([]) as Box<[TypeId]>, TableId(0));
        Self { tables: vec![Table::empty()], by_key }
    }

    /// The zero-component table every newly spawned entity starts in.
    pub fn root_table(&self) -> TableId {
        TableId(0)
    }

    pub fn table(&self, id: TableId) -> &Table {
        &self.tables[id.0 as usize]
    }

    pub fn table_mut(&mut self, id: TableId) -> &mut Table {
        &mut self.tables[id.0 as usize]
    }

    pub fn table_count(&self) -> usize {
        self.tables.len()
    }

    /// Tables from index `start` onward, paired with their `TableId` --
    /// lets a query cache resume scanning where it left off without ever
    /// needing to construct a `TableId` itself.
    pub fn tables_from(&self, start: usize) -> impl Iterator<Item = (TableId, &Table)> {
        self.tables.iter().enumerate().skip(start).map(|(i, table)| (TableId(i as u32), table))
    }

    pub fn two_tables_mut(&mut self, a: TableId, b: TableId) -> [&mut Table; 2] {
        self.tables.get_disjoint_mut([a.0 as usize, b.0 as usize]).expect("src and dst tables must differ")
    }

    /// The table reached by adding component `T` to `src`'s set, creating
    /// it (and interning its canonical key) on first use. Cached as an edge
    /// on `src` afterward, so a repeated identical transition is O(1).
    pub fn add_edge<T: Component>(&mut self, src: TableId) -> TableId {
        let type_id = TypeId::of::<T>();
        if let Some(&dst) = self.tables[src.0 as usize].edges.add.get(&type_id) {
            return dst;
        }

        let mut key: Vec<TypeId> = self.tables[src.0 as usize].columns.keys().copied().collect();
        key.push(type_id);
        key.sort_unstable();

        let dst = if let Some(&existing) = self.by_key.get(key.as_slice()) {
            existing
        } else {
            let mut columns = ColumnMap::default();
            for (&id, col) in &self.tables[src.0 as usize].columns {
                columns.insert(id, col.new_empty());
            }
            columns.insert(type_id, Box::new(Vec::<T>::new()) as Box<dyn Column>);
            let new_id = TableId(self.tables.len() as u32);
            self.tables.push(Table { entities: Vec::new(), columns, edges: TableEdges::default() });
            self.by_key.insert(key.into_boxed_slice(), new_id);
            new_id
        };

        self.tables[src.0 as usize].edges.add.insert(type_id, dst);
        self.tables[dst.0 as usize].edges.remove.insert(type_id, src);
        dst
    }

    /// Symmetric counterpart to `add_edge`: the table reached by removing
    /// component `T` from `src`'s set.
    pub fn remove_edge<T: Component>(&mut self, src: TableId) -> TableId {
        let type_id = TypeId::of::<T>();
        if let Some(&dst) = self.tables[src.0 as usize].edges.remove.get(&type_id) {
            return dst;
        }

        let mut key: Vec<TypeId> =
            self.tables[src.0 as usize].columns.keys().copied().filter(|&id| id != type_id).collect();
        key.sort_unstable();

        let dst = if let Some(&existing) = self.by_key.get(key.as_slice()) {
            existing
        } else {
            let mut columns = ColumnMap::default();
            for (&id, col) in &self.tables[src.0 as usize].columns {
                if id != type_id {
                    columns.insert(id, col.new_empty());
                }
            }
            let new_id = TableId(self.tables.len() as u32);
            self.tables.push(Table { entities: Vec::new(), columns, edges: TableEdges::default() });
            self.by_key.insert(key.into_boxed_slice(), new_id);
            new_id
        };

        self.tables[src.0 as usize].edges.remove.insert(type_id, dst);
        self.tables[dst.0 as usize].edges.add.insert(type_id, src);
        dst
    }
}
