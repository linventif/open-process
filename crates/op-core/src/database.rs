//! Relational database schemas, independent of the GUI and of a live database.
//! This models tables directly; Merise MCD transformations are a separate concern.
use crate::geom::{Pos, Rect};
use crate::i18n::{key, tr, trf};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

mod ddl;
pub use ddl::{generate, quote_identifier};

pub type Id = u64;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dialect {
    #[default]
    PostgreSql,
    MySql,
    Sqlite,
}
impl Dialect {
    pub const ALL: [Self; 3] = [Self::PostgreSql, Self::MySql, Self::Sqlite];
    pub fn label(self) -> &'static str {
        match self {
            Self::PostgreSql => "PostgreSQL",
            Self::MySql => "MySQL / MariaDB",
            Self::Sqlite => "SQLite",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SqlType {
    Integer,
    BigInt,
    VarChar { length: u32 },
    Text,
    Decimal { precision: u8, scale: u8 },
    Boolean,
    Date,
    Timestamp,
    Binary,
}
impl SqlType {
    pub const ALL: [Self; 9] = [
        Self::Integer,
        Self::BigInt,
        Self::VarChar { length: 255 },
        Self::Text,
        Self::Decimal { precision: 12, scale: 2 },
        Self::Boolean,
        Self::Date,
        Self::Timestamp,
        Self::Binary,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Integer => "INTEGER",
            Self::BigInt => "BIGINT",
            Self::VarChar { .. } => "VARCHAR",
            Self::Text => "TEXT",
            Self::Decimal { .. } => "DECIMAL",
            Self::Boolean => "BOOLEAN",
            Self::Date => "DATE",
            Self::Timestamp => "TIMESTAMP",
            Self::Binary => "BINARY",
        }
    }
    pub fn ddl(self, dialect: Dialect) -> String {
        match (self, dialect) {
            (Self::Integer | Self::BigInt, Dialect::Sqlite) => "INTEGER".into(),
            (Self::Binary, Dialect::PostgreSql) => "BYTEA".into(),
            (Self::Binary, _) => "BLOB".into(),
            (Self::Timestamp, Dialect::MySql) => "DATETIME".into(),
            (Self::VarChar { length }, _) => format!("VARCHAR({length})"),
            (Self::Decimal { precision, scale }, _) => format!("DECIMAL({precision},{scale})"),
            _ => self.label().into(),
        }
    }
    fn valid(self) -> bool {
        match self {
            Self::VarChar { length } => length > 0 && length <= 65_535,
            Self::Decimal { precision, scale } => precision > 0 && precision <= 38 && scale <= precision,
            _ => true,
        }
    }
    pub fn is_integer(self) -> bool {
        matches!(self, Self::Integer | Self::BigInt)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum DefaultValue {
    #[default]
    None,
    Text(String),
    Number(String),
    Boolean(bool),
    Null,
    CurrentDate,
    CurrentTimestamp,
}
impl DefaultValue {
    pub fn label(&self) -> &'static str {
        match self {
            Self::None => key("database.no_default"),
            Self::Text(_) => key("database.text_literal"),
            Self::Number(_) => key("database.number_literal"),
            Self::Boolean(_) => key("database.boolean_literal"),
            Self::Null => "NULL",
            Self::CurrentDate => "CURRENT_DATE",
            Self::CurrentTimestamp => "CURRENT_TIMESTAMP",
        }
    }
    pub fn choices() -> [Self; 7] {
        [
            Self::None,
            Self::Text(String::new()),
            Self::Number("0".into()),
            Self::Boolean(false),
            Self::Null,
            Self::CurrentDate,
            Self::CurrentTimestamp,
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub id: Id,
    pub name: String,
    pub data_type: SqlType,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
    pub auto_increment: bool,
    pub default: DefaultValue,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub id: Id,
    pub name: String,
    pub columns: Vec<Id>,
    pub unique: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub id: Id,
    pub name: String,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub bounds: Rect,
}
impl Table {
    pub fn column(&self, id: Id) -> Option<&Column> {
        self.columns.iter().find(|c| c.id == id)
    }
    pub fn primary_key(&self) -> Vec<Id> {
        self.columns.iter().filter(|c| c.primary_key).map(|c| c.id).collect()
    }
    pub fn is_candidate_key(&self, columns: &[Id]) -> bool {
        !columns.is_empty()
            && (self.primary_key() == columns
                || (columns.len() == 1 && self.column(columns[0]).is_some_and(|c| c.unique))
                || self.indexes.iter().any(|i| i.unique && i.columns == columns))
    }
    pub fn rect(&self) -> Rect {
        Rect {
            w: self.bounds.w.max(190.0),
            h: self.bounds.h.max(42.0 + self.columns.len() as f32 * 25.0),
            ..self.bounds
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferentialAction {
    #[default]
    NoAction,
    Restrict,
    Cascade,
    SetNull,
}
impl ReferentialAction {
    pub const ALL: [Self; 4] = [Self::NoAction, Self::Restrict, Self::Cascade, Self::SetNull];
    pub fn sql(self) -> &'static str {
        match self {
            Self::NoAction => "NO ACTION",
            Self::Restrict => "RESTRICT",
            Self::Cascade => "CASCADE",
            Self::SetNull => "SET NULL",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ForeignKey {
    pub id: Id,
    pub name: String,
    pub table: Id,
    pub columns: Vec<Id>,
    pub referenced_table: Id,
    pub referenced_columns: Vec<Id>,
    pub on_delete: ReferentialAction,
    pub on_update: ReferentialAction,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Schema {
    pub name: String,
    pub dialect: Dialect,
    pub tables: Vec<Table>,
    pub foreign_keys: Vec<ForeignKey>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub element: Id,
    pub message: String,
}

impl Schema {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            dialect: Default::default(),
            tables: vec![],
            foreign_keys: vec![],
        }
    }
    pub fn table(&self, id: Id) -> Option<&Table> {
        self.tables.iter().find(|t| t.id == id)
    }
    pub fn next_id(&self) -> Id {
        self.tables
            .iter()
            .flat_map(|t| {
                std::iter::once(t.id)
                    .chain(t.columns.iter().map(|c| c.id))
                    .chain(t.indexes.iter().map(|i| i.id))
            })
            .chain(self.foreign_keys.iter().map(|f| f.id))
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
    pub fn add_table(&mut self, pos: Pos) -> Id {
        let id = self.next_id();
        let name = unique_name("table", self.tables.iter().map(|t| t.name.as_str()));
        self.tables.push(Table {
            id,
            name,
            columns: vec![],
            indexes: vec![],
            bounds: Rect::new(pos.x, pos.y, 260.0, 120.0),
        });
        let column = self.add_column(id).unwrap();
        let c = self.tables.last_mut().unwrap().columns.iter_mut().find(|c| c.id == column).unwrap();
        c.name = "id".into();
        c.data_type = SqlType::Integer;
        c.nullable = false;
        c.primary_key = true;
        c.auto_increment = true;
        id
    }
    pub fn add_column(&mut self, table: Id) -> Option<Id> {
        let id = self.next_id();
        let t = self.tables.iter_mut().find(|t| t.id == table)?;
        let name = unique_name("column", t.columns.iter().map(|c| c.name.as_str()));
        t.columns.push(Column {
            id,
            name,
            data_type: SqlType::VarChar { length: 255 },
            nullable: true,
            primary_key: false,
            unique: false,
            auto_increment: false,
            default: DefaultValue::None,
        });
        Some(id)
    }
    pub fn add_index(&mut self, table: Id) -> Option<Id> {
        let id = self.next_id();
        let t = self.tables.iter_mut().find(|t| t.id == table)?;
        let first = t.columns.first()?.id;
        t.indexes.push(Index {
            id,
            name: format!("idx_{id}"),
            columns: vec![first],
            unique: false,
        });
        Some(id)
    }
    /// A relationship starts from the referenced PK and creates missing local columns.
    pub fn add_foreign_key(&mut self, table: Id, target: Id) -> Result<Id, String> {
        if self.table(table).is_none() {
            return Err(tr("database.select_table_first").into());
        }
        let parent = self
            .table(target)
            .ok_or_else(|| tr("database.referenced_table_missing").to_owned())?
            .clone();
        let references = parent.primary_key();
        if references.is_empty() {
            return Err(tr("database.referenced_table_needs_primary_key").into());
        }
        let mut columns = vec![];
        for reference in &references {
            let source = parent.column(*reference).unwrap();
            let name = format!("{}_{}", parent.name, source.name);
            let existing = self
                .table(table)
                .unwrap()
                .columns
                .iter()
                .find(|c| c.name == name && c.data_type == source.data_type)
                .map(|c| c.id);
            let id = if let Some(id) = existing {
                id
            } else {
                let id = self.add_column(table).unwrap();
                let c = self
                    .tables
                    .iter_mut()
                    .find(|t| t.id == table)
                    .unwrap()
                    .columns
                    .iter_mut()
                    .find(|c| c.id == id)
                    .unwrap();
                c.name = name;
                c.data_type = source.data_type;
                id
            };
            columns.push(id);
        }
        let id = self.next_id();
        self.foreign_keys.push(ForeignKey {
            id,
            name: format!("fk_{id}"),
            table,
            columns,
            referenced_table: target,
            referenced_columns: references,
            on_delete: ReferentialAction::NoAction,
            on_update: ReferentialAction::NoAction,
        });
        Ok(id)
    }
    pub fn delete_table(&mut self, id: Id) {
        self.tables.retain(|t| t.id != id);
        self.foreign_keys.retain(|f| f.table != id && f.referenced_table != id);
    }
    pub fn delete_column(&mut self, table: Id, id: Id) {
        if let Some(t) = self.tables.iter_mut().find(|t| t.id == table) {
            if t.column(id).is_none() {
                return;
            }
            t.columns.retain(|c| c.id != id);
            t.indexes.retain(|i| !i.columns.contains(&id));
        } else {
            return;
        }
        self.foreign_keys
            .retain(|f| !f.columns.contains(&id) && !f.referenced_columns.contains(&id));
    }
    /// Names and incomplete constraints remain editable; identity and geometry must be sound.
    pub fn check_storage(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        for id in self
            .tables
            .iter()
            .flat_map(|t| {
                std::iter::once(t.id)
                    .chain(t.columns.iter().map(|c| c.id))
                    .chain(t.indexes.iter().map(|i| i.id))
            })
            .chain(self.foreign_keys.iter().map(|f| f.id))
        {
            if id == 0 || id == u64::MAX || !ids.insert(id) {
                return Err(tr("database.database_element_identifiers_must_unique_non_zero").into());
            }
        }
        if self.tables.iter().any(|t| {
            let r = t.bounds;
            ![r.x, r.y, r.w, r.h].iter().all(|v| v.is_finite()) || r.w <= 0.0 || r.h <= 0.0
        }) {
            return Err(tr("validation.invalid_dimensions").into());
        }
        Ok(())
    }
    pub fn validate(&self, dialect: Dialect) -> Vec<Diagnostic> {
        let mut issues = vec![];
        let mut add = |element, message: String| issues.push(Diagnostic { element, message });
        if let Err(e) = self.check_storage() {
            add(0, e);
            return issues;
        }
        if self.tables.is_empty() {
            add(0, tr("database.add_table_before_generating_ddl").into());
        }
        let canonical = |s: &str| {
            if dialect == Dialect::PostgreSql {
                s.to_owned()
            } else {
                s.to_lowercase()
            }
        };
        let mut relations = BTreeSet::new();
        for t in &self.tables {
            if !valid_identifier(&t.name, dialect) || !relations.insert(canonical(&t.name)) {
                add(t.id, trf("database.invalid_duplicate_table_name", &[&t.name]));
            }
        }
        if dialect == Dialect::PostgreSql {
            for t in &self.tables {
                let implicit = (!t.primary_key().is_empty())
                    .then(|| format!("pk_{}", t.id))
                    .into_iter()
                    .chain(t.columns.iter().filter(|c| c.unique).map(|c| format!("uq_{}", c.id)))
                    .chain(t.columns.iter().filter(|c| c.auto_increment).map(|c| format!("seq_{}", c.id)));
                for name in implicit {
                    if !relations.insert(canonical(&name)) {
                        add(t.id, trf("database.invalid_duplicate_index_name", &[&name]));
                    }
                }
            }
        }
        let mut foreign_names: BTreeSet<_> = self
            .tables
            .iter()
            .flat_map(|t| {
                (!t.primary_key().is_empty())
                    .then(|| format!("pk_{}", t.id))
                    .into_iter()
                    .chain(t.columns.iter().filter(|c| c.unique).map(|c| format!("uq_{}", c.id)))
            })
            .collect();
        for t in &self.tables {
            if t.columns.is_empty() {
                add(t.id, trf("database.table_has_no_columns", &[&t.name]));
            }
            let mut names = BTreeSet::new();
            let pk = t.primary_key();
            let identities = t.columns.iter().filter(|c| c.auto_increment).count();
            for c in &t.columns {
                let context = format!("{}.{}", t.name, c.name);
                if !valid_identifier(&c.name, dialect) || !names.insert(canonical(&c.name)) {
                    add(c.id, trf("database.invalid_duplicate_column_name", &[&context]));
                }
                if !c.data_type.valid()
                    || (dialect == Dialect::MySql
                        && (matches!(c.data_type,SqlType::VarChar{length} if length>16_383)
                            || matches!(c.data_type,SqlType::Decimal{scale,..} if scale>30)))
                {
                    add(c.id, trf("database.invalid_sql_type_parameters", &[&context]));
                }
                if c.primary_key && c.nullable {
                    add(c.id, trf("database.primary_key_columns_cannot_nullable", &[&context]));
                }
                if c.auto_increment && (identities > 1 || pk != [c.id] || !c.data_type.is_integer() || c.default != DefaultValue::None) {
                    add(
                        c.id,
                        trf("database.identity_requires_single_integer_primary_key_without", &[&context]),
                    );
                }
                if !default_valid(c) {
                    add(c.id, trf("database.default_value_incompatible_column", &[&context]));
                }
            }
            let mut index_names = BTreeSet::new();
            if dialect == Dialect::MySql {
                index_names.extend(t.columns.iter().filter(|c| c.unique).map(|c| format!("uq_{}", c.id)));
            }
            for index in &t.indexes {
                if !valid_identifier(&index.name, dialect)
                    || index.name.eq_ignore_ascii_case("PRIMARY")
                    || !index_names.insert(canonical(&index.name))
                    || (dialect != Dialect::MySql && !relations.insert(canonical(&index.name)))
                {
                    add(index.id, trf("database.invalid_duplicate_index_name", &[&index.name]));
                }
                if !valid_columns(t, &index.columns) {
                    add(index.id, trf("database.index_has_missing_repeated_columns", &[&index.name]));
                }
            }
            if dialect == Dialect::MySql {
                let keys = std::iter::once(pk)
                    .chain(t.columns.iter().filter(|c| c.unique).map(|c| vec![c.id]))
                    .chain(t.indexes.iter().map(|i| i.columns.clone()));
                for cols in keys.filter(|cols| !cols.is_empty()) {
                    if !mysql_key_valid(t, &cols) {
                        add(t.id, trf("database.mysql_indexed_columns_require_bounded_types_within", &[&t.name]));
                    }
                }
            }
        }
        for f in &self.foreign_keys {
            if !valid_identifier(&f.name, dialect) || !foreign_names.insert(canonical(&f.name)) {
                add(f.id, trf("database.invalid_duplicate_foreign_key_name", &[&f.name]));
            }
            let (Some(child), Some(parent)) = (self.table(f.table), self.table(f.referenced_table)) else {
                add(f.id, tr("database.referenced_table_missing").into());
                continue;
            };
            if dialect == Dialect::MySql && child.indexes.iter().any(|i| i.name.eq_ignore_ascii_case(&f.name)) {
                add(f.id, trf("database.invalid_duplicate_index_name", &[&f.name]));
            }
            if !valid_columns(child, &f.columns)
                || !valid_columns(parent, &f.referenced_columns)
                || f.columns.len() != f.referenced_columns.len()
            {
                add(f.id, trf("database.foreign_key_columns_missing_repeated_unbalanced", &[&f.name]));
                continue;
            }
            if !parent.is_candidate_key(&f.referenced_columns) {
                add(f.id, trf("database.foreign_key_must_reference_primary_unique_key", &[&f.name]));
            }
            if f.columns
                .iter()
                .zip(&f.referenced_columns)
                .any(|(a, b)| child.column(*a).unwrap().data_type != parent.column(*b).unwrap().data_type)
            {
                add(f.id, trf("database.foreign_key_column_types_do_not_match", &[&f.name]));
            }
            if (f.on_delete == ReferentialAction::SetNull || f.on_update == ReferentialAction::SetNull)
                && f.columns.iter().any(|id| !child.column(*id).unwrap().nullable)
            {
                add(f.id, trf("database.set_null_requires_nullable_foreign_key_columns", &[&f.name]));
            }
            if dialect == Dialect::MySql && !mysql_key_valid(child, &f.columns) {
                add(
                    f.id,
                    trf("database.mysql_indexed_columns_require_bounded_types_within", &[&child.name]),
                );
            }
        }
        issues
    }
}
fn unique_name<'a>(prefix: &str, names: impl Iterator<Item = &'a str>) -> String {
    let names: BTreeSet<_> = names.map(str::to_lowercase).collect();
    (1..).map(|n| format!("{prefix}_{n}")).find(|s| !names.contains(s)).unwrap()
}
fn valid_identifier(name: &str, dialect: Dialect) -> bool {
    !name.trim().is_empty()
        && !name.chars().any(char::is_control)
        && match dialect {
            Dialect::PostgreSql => name.len() <= 63,
            Dialect::MySql => name.chars().count() <= 64 && name.chars().all(|c| u32::from(c) <= 0xffff) && !name.ends_with(' '),
            Dialect::Sqlite => !name.to_ascii_lowercase().starts_with("sqlite_"),
        }
}
fn valid_columns(t: &Table, ids: &[Id]) -> bool {
    !ids.is_empty() && ids.iter().all(|id| t.column(*id).is_some()) && ids.iter().copied().collect::<BTreeSet<_>>().len() == ids.len()
}
fn mysql_key_valid(t: &Table, ids: &[Id]) -> bool {
    let sizes: Option<Vec<usize>> = ids
        .iter()
        .map(|id| match t.column(*id)?.data_type {
            SqlType::Text | SqlType::Binary => None,
            SqlType::VarChar { length } => Some(length as usize * 4),
            SqlType::Integer => Some(4),
            SqlType::Boolean => Some(1),
            _ => Some(32),
        })
        .collect();
    sizes.is_some_and(|sizes| sizes.iter().sum::<usize>() <= 3072)
}
fn default_valid(c: &Column) -> bool {
    match &c.default {
        DefaultValue::None => true,
        DefaultValue::Null => c.nullable && !c.primary_key,
        DefaultValue::Text(s) => {
            matches!(c.data_type, SqlType::Text | SqlType::VarChar { .. })
                && !s.contains('\0')
                && match c.data_type {
                    SqlType::VarChar { length } => s.chars().count() <= length as usize,
                    _ => true,
                }
        }
        DefaultValue::Number(s) => {
            let s = s.trim();
            let lexical = !s.is_empty() && s.bytes().all(|b| b"0123456789+-.eE".contains(&b)) && s.parse::<f64>().is_ok_and(f64::is_finite);
            lexical
                && match c.data_type {
                    SqlType::Integer => s.parse::<i32>().is_ok(),
                    SqlType::BigInt => s.parse::<i64>().is_ok(),
                    SqlType::Decimal { precision, scale } => decimal_fits(s, precision, scale),
                    _ => false,
                }
        }
        DefaultValue::Boolean(_) => c.data_type == SqlType::Boolean,
        DefaultValue::CurrentDate => c.data_type == SqlType::Date,
        DefaultValue::CurrentTimestamp => c.data_type == SqlType::Timestamp,
    }
}

/// Check exact decimal representability without rounding through a binary float.
fn decimal_fits(value: &str, precision: u8, scale: u8) -> bool {
    if scale > precision {
        return false;
    }
    let unsigned = value.trim_start_matches(['+', '-']);
    let (mantissa, exponent) = if let Some((m, e)) = unsigned.split_once(['e', 'E']) {
        let Ok(exponent) = e.parse::<i32>() else {
            return false;
        };
        if !(-1000..=1000).contains(&exponent) {
            return false;
        }
        (m, exponent)
    } else {
        (unsigned, 0)
    };
    let integer_length = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
    let digits = mantissa.bytes().filter(|b| *b != b'.').collect::<Vec<_>>();
    let Some(first) = digits.iter().position(|b| *b != b'0') else {
        return true;
    };
    let last = digits.iter().rposition(|b| *b != b'0').unwrap() as i32;
    let position = integer_length + exponent;
    (position - first as i32).max(0) <= i32::from(precision - scale) && (last + 1 - position).max(0) <= i32::from(scale)
}

pub fn example() -> Schema {
    let mut s = Schema::new(tr(key("database.shop_database")));
    let customer = s.add_table(Pos::new(90.0, 90.0));
    s.tables[0].name = "customers".into();
    let email = s.add_column(customer).unwrap();
    let c = s.tables[0].columns.iter_mut().find(|c| c.id == email).unwrap();
    c.name = "email".into();
    c.data_type = SqlType::VarChar { length: 200 };
    c.nullable = false;
    c.unique = true;
    let order = s.add_table(Pos::new(480.0, 90.0));
    s.tables[1].name = "orders".into();
    let total = s.add_column(order).unwrap();
    let c = s.tables[1].columns.iter_mut().find(|c| c.id == total).unwrap();
    c.name = "total".into();
    c.data_type = SqlType::Decimal { precision: 12, scale: 2 };
    c.nullable = false;
    c.default = DefaultValue::Number("0".into());
    s.add_foreign_key(order, customer).unwrap();
    let item = s.add_table(Pos::new(480.0, 360.0));
    s.tables[2].name = "order_items".into();
    let quantity = s.add_column(item).unwrap();
    let c = s.tables[2].columns.iter_mut().find(|c| c.id == quantity).unwrap();
    c.name = "quantity".into();
    c.data_type = SqlType::Integer;
    c.nullable = false;
    c.default = DefaultValue::Number("1".into());
    let fk = s.add_foreign_key(item, order).unwrap();
    s.foreign_keys.iter_mut().find(|f| f.id == fk).unwrap().on_delete = ReferentialAction::Cascade;
    s
}
