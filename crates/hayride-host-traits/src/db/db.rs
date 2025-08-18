use super::errors::ErrorCode;

#[cfg(feature = "postgres")]
use postgres_types::{ToSql, Type, IsNull};

pub trait DBTrait: Send + Sync {
    fn connect(&mut self, config: DBConfig) -> Result<Connection, ErrorCode>;
    fn connect_string(&mut self, connection_string: String) -> Result<Connection, ErrorCode>;
}

pub trait DBConnection: Send + Sync {
    fn query(
        &self,
        statement: String,
        params: Vec<DBValue>,
    ) -> Result<QueryResult, ErrorCode>;
    fn execute(
        &self,
        statement: String,
        params: Vec<DBValue>,
    ) -> Result<u64, ErrorCode>; // returns number of affected rows
    fn close(&mut self) -> Result<(), ErrorCode>;
}

/// Configuration parameters for connecting to a database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DBConfig {
    pub host: String,
    pub port: Option<u16>,
    pub database: String,
    pub username: String,
    pub password: Option<String>,
    pub ssl_mode: Option<String>,
    pub connect_timeout: Option<u32>, // in seconds
    // Additional options can be added as needed
    pub params: Option<Vec<(String, String)>>,
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

/// Information about a single column in a query result set.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnInfo {
    pub name: String,
    // DB-specific type OID or identifier
    pub type_oid: u32,
    // Human-readable type name
    pub type_name: String,
    // Whether the column can be NULL
    pub nullable: bool,
}

/// A single row of DB values.
#[derive(Debug, Clone, PartialEq)]
pub struct Row(pub Vec<DBValue>);

/// A Query result set.
#[derive(Debug, Clone, PartialEq)]
pub struct QueryResult {
    pub columns: Vec<ColumnInfo>,
    pub rows: Vec<Row>,
}

/// Array element types (non-recursive, matching WIT array-element)
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayElement {
    /// NULL value
    Null,
    Boolean(bool),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Text(String),
    Bytes(Vec<u8>),
    Date(String),
    Time(String),
    Timestamp(String),
    TimestampTz(String),
    Uuid(String),
    Json(Vec<u8>),
    /// Numeric/decimal value - string representation for precision
    Numeric(String),
    /// DB-specific types as raw string
    Custom(String),
}

/// Database value types (matching WIT db-value variant)
#[derive(Debug, Clone, PartialEq)]
pub enum DBValue {
    /// NULL value
    Null,
    Boolean(bool),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Float32(f32),
    Float64(f64),
    Text(String),
    Bytes(Vec<u8>),
    Date(String),
    Time(String),
    Timestamp(String),
    TimestampTz(String),
    Uuid(String),
    Json(Vec<u8>),
    /// Array element types (one-dimensional)
    Array(Vec<ArrayElement>),
    /// Numeric/decimal value - string representation for precision
    Numeric(String),
    /// DB-specific types as raw string
    Custom(String),
}

impl DBValue {
    /// Check if the value is NULL
    pub fn is_null(&self) -> bool {
        matches!(self, DBValue::Null)
    }

    /// Convert to a string representation (for debugging/display)
    pub fn to_string(&self) -> String {
        match self {
            DBValue::Null => "NULL".to_string(),
            DBValue::Boolean(b) => b.to_string(),
            DBValue::Int16(i) => i.to_string(),
            DBValue::Int32(i) => i.to_string(),
            DBValue::Int64(i) => i.to_string(),
            DBValue::Float32(f) => f.to_string(),
            DBValue::Float64(f) => f.to_string(),
            DBValue::Text(s) => s.clone(),
            DBValue::Bytes(b) => format!("\\x{}", bytes_to_hex(b)),
            DBValue::Date(s) => s.clone(),
            DBValue::Time(s) => s.clone(),
            DBValue::Timestamp(s) => s.clone(),
            DBValue::TimestampTz(s) => s.clone(),
            DBValue::Uuid(s) => s.clone(),
            DBValue::Json(b) => String::from_utf8_lossy(b).to_string(),
            DBValue::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|e| e.to_string()).collect();
                format!("[{}]", elements.join(", "))
            }
            DBValue::Numeric(s) => s.clone(),
            DBValue::Custom(s) => s.clone(),
        }
    }

    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self {
            DBValue::Null => "null",
            DBValue::Boolean(_) => "boolean",
            DBValue::Int16(_) => "int16",
            DBValue::Int32(_) => "int32",
            DBValue::Int64(_) => "int64",
            DBValue::Float32(_) => "float32",
            DBValue::Float64(_) => "float64",
            DBValue::Text(_) => "text",
            DBValue::Bytes(_) => "bytes",
            DBValue::Date(_) => "date",
            DBValue::Time(_) => "time",
            DBValue::Timestamp(_) => "timestamp",
            DBValue::TimestampTz(_) => "timestamptz",
            DBValue::Uuid(_) => "uuid",
            DBValue::Json(_) => "json",
            DBValue::Array(_) => "array",
            DBValue::Numeric(_) => "numeric",
            DBValue::Custom(_) => "custom",
        }
    }
}

impl ArrayElement {
    /// Convert to a string representation (for debugging/display)
    pub fn to_string(&self) -> String {
        match self {
            ArrayElement::Null => "NULL".to_string(),
            ArrayElement::Boolean(b) => b.to_string(),
            ArrayElement::Int16(i) => i.to_string(),
            ArrayElement::Int32(i) => i.to_string(),
            ArrayElement::Int64(i) => i.to_string(),
            ArrayElement::Float32(f) => f.to_string(),
            ArrayElement::Float64(f) => f.to_string(),
            ArrayElement::Text(s) => s.clone(),
            ArrayElement::Bytes(b) => format!("\\x{}", bytes_to_hex(b)),
            ArrayElement::Date(s) => s.clone(),
            ArrayElement::Time(s) => s.clone(),
            ArrayElement::Timestamp(s) => s.clone(),
            ArrayElement::TimestampTz(s) => s.clone(),
            ArrayElement::Uuid(s) => s.clone(),
            ArrayElement::Json(b) => String::from_utf8_lossy(b).to_string(),
            ArrayElement::Numeric(s) => s.clone(),
            ArrayElement::Custom(s) => s.clone(),
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

impl From<i16> for DBValue {
    fn from(value: i16) -> Self {
        DBValue::Int16(value)
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

impl From<f32> for DBValue {
    fn from(value: f32) -> Self {
        DBValue::Float32(value)
    }
}

impl From<f64> for DBValue {
    fn from(value: f64) -> Self {
        DBValue::Float64(value)
    }
}

impl From<String> for DBValue {
    fn from(value: String) -> Self {
        DBValue::Text(value)
    }
}

impl From<&str> for DBValue {
    fn from(value: &str) -> Self {
        DBValue::Text(value.to_string())
    }
}

impl From<Vec<u8>> for DBValue {
    fn from(value: Vec<u8>) -> Self {
        DBValue::Bytes(value)
    }
}

impl From<&[u8]> for DBValue {
    fn from(value: &[u8]) -> Self {
        DBValue::Bytes(value.to_vec())
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
            DBValue::Boolean(b) => b.to_sql(ty, out),
            DBValue::Int16(i) => i.to_sql(ty, out),
            DBValue::Int32(i) => i.to_sql(ty, out),
            DBValue::Int64(i) => i.to_sql(ty, out),
            DBValue::Float32(f) => f.to_sql(ty, out),
            DBValue::Float64(f) => f.to_sql(ty, out),
            DBValue::Text(s) => s.to_sql(ty, out),
            DBValue::Bytes(b) => b.to_sql(ty, out),
            DBValue::Date(s) => {
                // For date, we'll pass it as a string and let PostgreSQL handle the conversion
                s.to_sql(ty, out)
            },
            DBValue::Time(s) => {
                // For time, we'll pass it as a string and let PostgreSQL handle the conversion
                s.to_sql(ty, out)
            },
            DBValue::Timestamp(s) => {
                // For timestamp, we'll pass it as a string and let PostgreSQL handle the conversion
                s.to_sql(ty, out)
            },
            DBValue::TimestampTz(s) => {
                // For timestamptz, we'll pass it as a string and let PostgreSQL handle the conversion
                s.to_sql(ty, out)
            },
            DBValue::Uuid(s) => {
                // For UUID, we'll pass it as a string and let PostgreSQL handle the conversion
                s.to_sql(ty, out)
            },
            DBValue::Json(b) => {
                // For JSON, convert bytes to string and pass as string
                let json_str = String::from_utf8_lossy(b);
                json_str.to_sql(ty, out)
            },
            DBValue::Array(arr) => {
                // Convert to Vec of strings as a fallback
                let string_vec: Vec<String> = arr.iter().map(|e| e.to_string()).collect();
                string_vec.to_sql(ty, out)
            },
            DBValue::Numeric(s) => {
                s.to_sql(ty, out)
            },
            DBValue::Custom(s) => {
                // For custom types, pass as string
                s.to_sql(ty, out)
            },
        }
    }

    fn accepts(_ty: &Type) -> bool {
        // DBValue can potentially accept any PostgreSQL type
        // since it's designed to be a universal value container
        true
    }

    postgres_types::to_sql_checked!();
}

#[cfg(feature = "postgres")]
impl ToSql for ArrayElement {
    fn to_sql(&self, ty: &Type, out: &mut bytes::BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        match self {
            ArrayElement::Null => Ok(IsNull::Yes),
            ArrayElement::Boolean(b) => b.to_sql(ty, out),
            ArrayElement::Int16(i) => i.to_sql(ty, out),
            ArrayElement::Int32(i) => i.to_sql(ty, out),
            ArrayElement::Int64(i) => i.to_sql(ty, out),
            ArrayElement::Float32(f) => f.to_sql(ty, out),
            ArrayElement::Float64(f) => f.to_sql(ty, out),
            ArrayElement::Text(s) => s.to_sql(ty, out),
            ArrayElement::Bytes(b) => b.to_sql(ty, out),
            ArrayElement::Date(s) => s.to_sql(ty, out),
            ArrayElement::Time(s) => s.to_sql(ty, out),
            ArrayElement::Timestamp(s) => s.to_sql(ty, out),
            ArrayElement::TimestampTz(s) => s.to_sql(ty, out),
            ArrayElement::Uuid(s) => s.to_sql(ty, out),
            ArrayElement::Json(b) => {
                let json_str = String::from_utf8_lossy(b);
                json_str.to_sql(ty, out)
            },
            ArrayElement::Numeric(s) => {
                s.to_sql(ty, out)
            },
            ArrayElement::Custom(s) => s.to_sql(ty, out),
        }
    }

    fn accepts(_ty: &Type) -> bool {
        true
    }

    postgres_types::to_sql_checked!();
}
