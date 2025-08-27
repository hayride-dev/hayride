use super::errors::ErrorCode;

#[cfg(feature = "postgres")]
use postgres_types::{ToSql, Type, IsNull};

pub trait DBTrait: Send + Sync {
    fn open(&mut self, name: String) -> Result<Connection, ErrorCode>;
}

pub trait DBConnection: Send + Sync {
    fn prepare(&self, query: String) -> Result<Statement, ErrorCode>;
    fn begin_transaction(&mut self, isolation_level: IsolationLevel, read_only: bool) -> Result<Transaction, ErrorCode>;
    fn close(&mut self) -> Result<(), ErrorCode>;
}

pub trait DBStatement: Send + Sync {
    fn query(&self, params: Vec<DBValue>) -> Result<Rows, ErrorCode>;
    fn execute(&self, params: Vec<DBValue>) -> Result<u64, ErrorCode>;
    fn number_parameters(&self) -> Result<u32, ErrorCode>;
    fn close(&mut self) -> Result<(), ErrorCode>;
}

pub trait DBRows: Send + Sync {
    fn columns(&self) -> Vec<String>;
    fn next(&mut self) -> Result<Row, ErrorCode>;
    fn close(&mut self) -> Result<(), ErrorCode>;
}

pub trait DBTransaction: Send + Sync {
    fn commit(&mut self) -> Result<(), ErrorCode>;
    fn rollback(&mut self) -> Result<(), ErrorCode>;
    fn query(&self, query: String, params: Vec<DBValue>) -> Result<Rows, ErrorCode>;
    fn execute(&self, query: String, params: Vec<DBValue>) -> Result<u64, ErrorCode>;
    fn prepare(&self, query: String) -> Result<Statement, ErrorCode>;
}

/// A backend-defined DB Connection
pub struct Connection(Box<dyn DBConnection>);
impl From<Box<dyn DBConnection>> for Connection {
    fn from(value: Box<dyn DBConnection>) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for Connection {
    type Target = dyn DBConnection;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

/// A backend-defined prepared statement
pub struct Statement(Box<dyn DBStatement>);
impl From<Box<dyn DBStatement>> for Statement {
    fn from(value: Box<dyn DBStatement>) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for Statement {
    type Target = dyn DBStatement;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Statement {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

pub struct Rows(Box<dyn DBRows>);
impl From<Box<dyn DBRows>> for Rows {
    fn from(value: Box<dyn DBRows>) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for Rows {
    type Target = dyn DBRows;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Rows {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

pub struct Transaction(Box<dyn DBTransaction>);
impl From<Box<dyn DBTransaction>> for Transaction {
    fn from(value: Box<dyn DBTransaction>) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for Transaction {
    type Target = dyn DBTransaction;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Transaction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    WriteCommitted,
    RepeatableRead,
    Snapshot,
    Serializable,
    Linearizable,
}

/// A single row of DB values.
#[derive(Debug, Clone, PartialEq)]
pub struct Row(pub Vec<DBValue>);

/// Database value types (matching WIT db-value variant)
#[derive(Debug, Clone, PartialEq)]
pub enum DBValue {
    Int32(i32),
    Int64(i64),
    Uint32(u32),
    Uint64(u64),
    Float(f64),
    Double(f64),
    Str(String),
    Boolean(bool),
    Date(String),
    Time(String),
    Timestamp(String),
    Binary(Vec<u8>),
    Null,
}

impl DBValue {
    /// Check if the value is NULL
    pub fn is_null(&self) -> bool {
        matches!(self, DBValue::Null)
    }

    /// Convert to a string representation (for debugging/display)
    pub fn to_string(&self) -> String {
        match self {
            DBValue::Int32(i) => i.to_string(),
            DBValue::Int64(i) => i.to_string(),
            DBValue::Uint32(i) => i.to_string(),
            DBValue::Uint64(i) => i.to_string(),
            DBValue::Float(f) => f.to_string(),
            DBValue::Double(f) => f.to_string(),
            DBValue::Str(s) => s.clone(),
            DBValue::Boolean(b) => b.to_string(),
            DBValue::Date(s) => s.clone(),
            DBValue::Time(s) => s.clone(),
            DBValue::Timestamp(s) => s.clone(),
            DBValue::Binary(b) => format!("\\x{}", bytes_to_hex(b)),
            DBValue::Null => "NULL".to_string(),
        }
    }

    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self {
            DBValue::Int32(_) => "int32",
            DBValue::Int64(_) => "int64",
            DBValue::Uint32(_) => "uint32",
            DBValue::Uint64(_) => "uint64",
            DBValue::Float(_) => "float",
            DBValue::Double(_) => "double",
            DBValue::Str(_) => "str",
            DBValue::Boolean(_) => "boolean",
            DBValue::Date(_) => "date",
            DBValue::Time(_) => "time",
            DBValue::Timestamp(_) => "timestamp",
            DBValue::Binary(_) => "binary",
            DBValue::Null => "null",
        }
    }
}

/// Helper function to convert bytes to hex string without external dependencies
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<String>()
}

/// Convenient conversion traits for common Rust types to DBValue
impl From<bool> for DBValue {
    fn from(value: bool) -> Self {
        DBValue::Boolean(value)
    }
}

impl From<i32> for DBValue {
    fn from(value: i32) -> Self {
        DBValue::Int32(value)
    }
}

impl From<i64> for DBValue {
    fn from(value: i64) -> Self {
        DBValue::Int64(value)
    }
}

impl From<u32> for DBValue {
    fn from(value: u32) -> Self {
        DBValue::Uint32(value)
    }
}

impl From<u64> for DBValue {
    fn from(value: u64) -> Self {
        DBValue::Uint64(value)
    }
}

impl From<f64> for DBValue {
    fn from(value: f64) -> Self {
        DBValue::Double(value)
    }
}

impl From<String> for DBValue {
    fn from(value: String) -> Self {
        DBValue::Str(value)
    }
}

impl From<&str> for DBValue {
    fn from(value: &str) -> Self {
        DBValue::Str(value.to_string())
    }
}

impl From<Vec<u8>> for DBValue {
    fn from(value: Vec<u8>) -> Self {
        DBValue::Binary(value)
    }
}

impl From<&[u8]> for DBValue {
    fn from(value: &[u8]) -> Self {
        DBValue::Binary(value.to_vec())
    }
}

impl<T> From<Option<T>> for DBValue 
where 
    T: Into<DBValue>
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(val) => val.into(),
            None => DBValue::Null,
        }
    }
}

#[cfg(feature = "postgres")]
impl ToSql for DBValue {
    fn to_sql(&self, ty: &Type, out: &mut bytes::BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        match self {
            DBValue::Null => Ok(IsNull::Yes),
            DBValue::Int32(i) => i.to_sql(ty, out),
            DBValue::Int64(i) => i.to_sql(ty, out),
            DBValue::Uint32(i) => (*i as i64).to_sql(ty, out), // Convert to i64 for PostgreSQL
            DBValue::Uint64(i) => (*i as i64).to_sql(ty, out), // Convert to i64 for PostgreSQL
            DBValue::Float(f) => f.to_sql(ty, out),
            DBValue::Double(f) => f.to_sql(ty, out),
            DBValue::Str(s) => s.to_sql(ty, out),
            DBValue::Boolean(b) => b.to_sql(ty, out),
            DBValue::Date(s) => s.to_sql(ty, out),
            DBValue::Time(s) => s.to_sql(ty, out),
            DBValue::Timestamp(s) => s.to_sql(ty, out),
            DBValue::Binary(b) => b.to_sql(ty, out),
        }
    }

    fn accepts(_ty: &Type) -> bool {
        // DBValue can potentially accept any PostgreSQL type
        // since it's designed to be a universal value container
        true
    }

    postgres_types::to_sql_checked!();
}
