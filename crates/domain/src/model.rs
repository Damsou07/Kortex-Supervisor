use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum Error {
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    Invalid(String),
    #[error(
        "Conflit : la ligne a été modifiée ou supprimée depuis sa lecture. Rechargez les données avant de réappliquer."
    )]
    Conflict,
    #[error("Requête annulée.")]
    Cancelled,
    #[error("{0}")]
    Storage(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ObjectId {
    pub schema: String,
    pub name: String,
}
impl ObjectId {
    pub fn new(schema: &str, name: &str) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
        }
    }
    pub fn key(&self) -> String {
        format!("{}:{}{}", self.schema.len(), self.schema, self.name)
    }
    pub fn label(&self) -> String {
        format!("{}.{}", self.schema, self.name)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Engine {
    Postgres {
        host: String,
        port: u16,
        database: String,
        user: String,
        tls: bool,
        ca_file: String,
    },
    Sqlite {
        path: String,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub engine: Engine,
    pub remember_password: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default: Option<String>,
    pub generated: bool,
    pub comment: String,
}
impl Column {
    pub fn new(name: &str, data_type: &str, nullable: bool) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            nullable,
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Index {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub partial: bool,
    pub definition: String,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub name: String,
    pub kind: String,
    pub definition: String,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ForeignKey {
    pub id: String,
    pub name: String,
    pub source: ObjectId,
    pub columns: Vec<String>,
    pub target: ObjectId,
    pub target_columns: Vec<String>,
    pub match_full: bool,
    pub on_update: String,
    pub on_delete: String,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub id: ObjectId,
    pub stable_id: String,
    pub is_view: bool,
    pub columns: Vec<Column>,
    pub primary_key: Vec<String>,
    pub unique_keys: Vec<Vec<String>>,
    pub indexes: Vec<Index>,
    pub constraints: Vec<Constraint>,
    pub foreign_keys: Vec<ForeignKey>,
    pub comment: String,
    pub definition: String,
    pub estimated_rows: Option<i64>,
}
impl Table {
    pub fn identity_columns(&self) -> Vec<String> {
        if self.is_view {
            return vec![];
        }
        std::iter::once(&self.primary_key)
            .chain(self.unique_keys.iter())
            .find(|key| {
                !key.is_empty()
                    && key
                        .iter()
                        .all(|name| self.columns.iter().any(|c| c.name == *name && !c.nullable))
            })
            .cloned()
            .unwrap_or_default()
    }
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.name == name)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Catalog {
    pub tables: Vec<Table>,
    pub databases: Vec<String>,
    pub server_version: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Binary(Vec<u8>),
}
impl Value {
    pub fn display(&self) -> String {
        match self {
            Self::Null => "NULL".into(),
            Self::Integer(v) => v.to_string(),
            Self::Real(v) => v.to_string(),
            Self::Text(v) => v.clone(),
            Self::Binary(v) => v.iter().map(|b| format!("{b:02x}")).collect(),
        }
    }
    pub fn text(&self) -> Option<String> {
        if matches!(self, Self::Null) {
            None
        } else {
            Some(self.display())
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DataRow {
    pub values: Vec<Value>,
    pub version: Option<String>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataPage {
    pub rows: Vec<DataRow>,
    pub has_more: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum InputValue {
    Null,
    Text(String),
    Default,
}
impl From<&Value> for InputValue {
    fn from(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            _ => Self::Text(value.display()),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum RowChange {
    Insert {
        values: BTreeMap<String, InputValue>,
    },
    Update {
        original: DataRow,
        values: BTreeMap<String, InputValue>,
    },
    Delete {
        original: DataRow,
    },
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChangeSet {
    pub changes: Vec<RowChange>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SqlEvent {
    Columns(Vec<String>),
    Rows(Vec<Vec<Value>>),
    Complete { affected: u64, truncated: bool },
    Error(Error),
    Finished { elapsed_ms: u128 },
}
