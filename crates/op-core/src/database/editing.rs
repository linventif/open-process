//! Relational selections remap table, column, index and foreign-key identities.
use super::*;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub tables: Vec<Table>,
    pub foreign_keys: Vec<ForeignKey>,
}
impl Schema {
    pub fn fragment(&self, selected: &BTreeSet<Id>) -> Fragment {
        Fragment {
            tables: self.tables.iter().filter(|t| selected.contains(&t.id)).cloned().collect(),
            foreign_keys: self
                .foreign_keys
                .iter()
                .filter(|f| selected.contains(&f.table) && selected.contains(&f.referenced_table))
                .cloned()
                .collect(),
        }
    }
    pub fn paste(&mut self, fragment: &Fragment, delta: Pos) -> Result<BTreeSet<Id>, String> {
        Self {
            tables: fragment.tables.clone(),
            foreign_keys: fragment.foreign_keys.clone(),
            ..Self::new("")
        }
        .check_storage()?;
        for table in &fragment.tables {
            for index in &table.indexes {
                if index.columns.iter().any(|id| table.column(*id).is_none()) {
                    return Err(tr("validation.storage_connection").into());
                }
            }
        }
        for key in &fragment.foreign_keys {
            let table = fragment.tables.iter().find(|t| t.id == key.table);
            let referenced = fragment.tables.iter().find(|t| t.id == key.referenced_table);
            if table.is_none()
                || referenced.is_none()
                || key.columns.iter().any(|id| table.unwrap().column(*id).is_none())
                || key.referenced_columns.iter().any(|id| referenced.unwrap().column(*id).is_none())
            {
                return Err(tr("validation.storage_connection").into());
            }
        }
        let count = fragment.tables.iter().map(|t| 1 + t.columns.len() + t.indexes.len()).sum::<usize>() + fragment.foreign_keys.len();
        if count > 20_000 || self.next_id().checked_add(count as u64).is_none_or(|id| id == u64::MAX) {
            return Err(tr("validation.storage_limit").into());
        }
        let mut names: BTreeSet<String> = self
            .tables
            .iter()
            .flat_map(|t| t.indexes.iter().map(|i| i.name.clone()))
            .chain(self.foreign_keys.iter().map(|k| k.name.clone()))
            .collect();
        let mut unique_name = |name: &mut String| {
            if name.is_empty() {
                return;
            }
            let base = name.clone();
            let mut suffix = 2;
            while !names.insert(name.clone()) {
                *name = format!("{base}_{suffix}");
                suffix += 1;
            }
        };
        let mut result = self.clone();
        let mut next = result.next_id();
        let mut map = HashMap::new();
        let mut ids = BTreeSet::new();
        let mut fresh = |old| {
            let id = next;
            next += 1;
            map.insert(old, id);
            id
        };
        for old in &fragment.tables {
            let mut table = old.clone();
            table.id = fresh(old.id);
            ids.insert(table.id);
            table.bounds.x += delta.x;
            table.bounds.y += delta.y;
            // SQL names remain unique when duplicating within the same schema.
            let base = table.name.clone();
            let mut suffix = 2;
            while result.tables.iter().any(|t| t.name == table.name) {
                table.name = format!("{base}_{suffix}");
                suffix += 1;
            }
            for column in &mut table.columns {
                column.id = fresh(column.id);
            }
            for index in &mut table.indexes {
                index.id = fresh(index.id);
                unique_name(&mut index.name);
            }
            result.tables.push(table);
        }
        for table in result.tables.iter_mut().filter(|t| ids.contains(&t.id)) {
            for index in &mut table.indexes {
                index.columns = index.columns.iter().filter_map(|id| map.get(id).copied()).collect();
            }
        }
        for old in &fragment.foreign_keys {
            let mut key = old.clone();
            key.id = next;
            next += 1;
            unique_name(&mut key.name);
            key.table = map[&key.table];
            key.referenced_table = map[&key.referenced_table];
            key.columns = key.columns.iter().map(|id| map[id]).collect();
            key.referenced_columns = key.referenced_columns.iter().map(|id| map[id]).collect();
            result.foreign_keys.push(key);
        }
        result.check_storage()?;
        *self = result;
        Ok(ids)
    }
}
