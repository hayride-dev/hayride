use super::errors::ErrorCode;

#[cfg(feature = "postgres")]
use postgres_types::{IsNull, ToSql, Type};

#[cfg(feature = "postgres")]
use std::str::FromStr;

pub trait DBTrait: Send + Sync {
    fn open(&mut self, name: String) -> Result<Connection, ErrorCode>;
}

pub trait DBConnection: Send + Sync {
    fn prepare(&self, query: String) -> Result<Statement, ErrorCode>;
    fn begin_transaction(
        &mut self,
        isolation_level: IsolationLevel,
        read_only: bool,
    ) -> Result<Transaction, ErrorCode>;
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
    bytes
        .iter()
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
    T: Into<DBValue>,
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
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut bytes::BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        // Handle NULL values first, regardless of type
        if matches!(self, DBValue::Null) {
            return Ok(IsNull::Yes);
        }

        // Primary dispatch based on PostgreSQL column type
        match *ty {
            // Numeric types
            Type::INT2 => match self {
                DBValue::Int32(i) => (*i as i16).to_sql(ty, out),
                DBValue::Int64(i) => (*i as i16).to_sql(ty, out),
                DBValue::Uint32(u) => (*u as i16).to_sql(ty, out),
                DBValue::Uint64(u) => (*u as i16).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<i16>()?.to_sql(ty, out),
                _ => self.to_string().parse::<i16>()?.to_sql(ty, out),
            },
            Type::INT4 => match self {
                DBValue::Int32(i) => i.to_sql(ty, out),
                DBValue::Int64(i) => (*i as i32).to_sql(ty, out),
                DBValue::Uint32(u) => (*u as i32).to_sql(ty, out),
                DBValue::Uint64(u) => (*u as i32).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<i32>()?.to_sql(ty, out),
                _ => self.to_string().parse::<i32>()?.to_sql(ty, out),
            },
            Type::INT8 => match self {
                DBValue::Int64(i) => i.to_sql(ty, out),
                DBValue::Int32(i) => (*i as i64).to_sql(ty, out),
                DBValue::Uint32(u) => (*u as i64).to_sql(ty, out),
                DBValue::Uint64(u) => (*u as i64).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<i64>()?.to_sql(ty, out),
                _ => self.to_string().parse::<i64>()?.to_sql(ty, out),
            },
            Type::FLOAT4 => match self {
                DBValue::Float(f) => (*f as f32).to_sql(ty, out),
                DBValue::Double(f) => (*f as f32).to_sql(ty, out),
                DBValue::Int32(i) => (*i as f32).to_sql(ty, out),
                DBValue::Int64(i) => (*i as f32).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<f32>()?.to_sql(ty, out),
                _ => self.to_string().parse::<f32>()?.to_sql(ty, out),
            },
            Type::FLOAT8 => match self {
                DBValue::Double(f) => f.to_sql(ty, out),
                DBValue::Float(f) => f.to_sql(ty, out),
                DBValue::Int32(i) => (*i as f64).to_sql(ty, out),
                DBValue::Int64(i) => (*i as f64).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<f64>()?.to_sql(ty, out),
                _ => self.to_string().parse::<f64>()?.to_sql(ty, out),
            },

            // Boolean type
            Type::BOOL => match self {
                DBValue::Boolean(b) => b.to_sql(ty, out),
                DBValue::Int32(i) => (*i != 0).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<bool>()?.to_sql(ty, out),
                _ => self.to_string().parse::<bool>()?.to_sql(ty, out),
            },

            // String types
            Type::TEXT | Type::VARCHAR | Type::CHAR | Type::NAME => {
                let string_value = match self {
                    DBValue::Str(s) => s.clone(),
                    _ => self.to_string(),
                };
                string_value.to_sql(ty, out)
            }

            // Binary type
            Type::BYTEA => match self {
                DBValue::Binary(b) => b.to_sql(ty, out),
                DBValue::Str(s) => s.as_bytes().to_vec().to_sql(ty, out),
                _ => self.to_string().as_bytes().to_vec().to_sql(ty, out),
            },

            // Date type - extract date from any temporal value
            Type::DATE => {
                let date = Self::extract_date_from_value(self)?;
                date.to_sql(ty, out)
            }

            // Time type - extract time from any temporal value
            Type::TIME => {
                let time = Self::extract_time_from_value(self)?;
                time.to_sql(ty, out)
            }

            // Timestamp type (no timezone)
            Type::TIMESTAMP => {
                let datetime = Self::extract_naive_datetime_from_value(self)?;
                datetime.to_sql(ty, out)
            }

            // Timestamp with timezone
            Type::TIMESTAMPTZ => {
                let datetime = Self::extract_utc_datetime_from_value(self)?;
                datetime.to_sql(ty, out)
            }

            // UUID type
            Type::UUID => match self {
                DBValue::Str(s) => {
                    let uuid = uuid::Uuid::parse_str(s)?;
                    uuid.to_sql(ty, out)
                }
                _ => {
                    let uuid = uuid::Uuid::parse_str(&self.to_string())?;
                    uuid.to_sql(ty, out)
                }
            },

            // JSON types
            Type::JSON | Type::JSONB => match self {
                DBValue::Binary(b) => {
                    let json_value: serde_json::Value = serde_json::from_slice(b)?;
                    json_value.to_sql(ty, out)
                }
                DBValue::Str(s) => {
                    let json_value: serde_json::Value = serde_json::from_str(s)?;
                    json_value.to_sql(ty, out)
                }
                _ => {
                    let json_value: serde_json::Value = serde_json::from_str(&self.to_string())?;
                    json_value.to_sql(ty, out)
                }
            },

            // Numeric/Decimal type
            Type::NUMERIC => match self {
                DBValue::Str(s) => {
                    let decimal = rust_decimal::Decimal::from_str(s)?;
                    decimal.to_sql(ty, out)
                }
                DBValue::Double(f) => {
                    let decimal = rust_decimal::Decimal::try_from(*f)?;
                    decimal.to_sql(ty, out)
                }
                DBValue::Float(f) => {
                    let decimal = rust_decimal::Decimal::try_from(*f)?;
                    decimal.to_sql(ty, out)
                }
                DBValue::Int32(i) => {
                    let decimal = rust_decimal::Decimal::from(*i);
                    decimal.to_sql(ty, out)
                }
                DBValue::Int64(i) => {
                    let decimal = rust_decimal::Decimal::from(*i);
                    decimal.to_sql(ty, out)
                }
                _ => {
                    let decimal = rust_decimal::Decimal::from_str(&self.to_string())?;
                    decimal.to_sql(ty, out)
                }
            },

            // Default fallback - try to convert to string
            _ => {
                let string_value = match self {
                    DBValue::Str(s) => s.clone(),
                    _ => self.to_string(),
                };
                string_value.to_sql(ty, out)
            }
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
impl DBValue {
    /// Extract a NaiveDate from any DBValue that might contain date information
    fn extract_date_from_value(
        value: &DBValue,
    ) -> Result<chrono::NaiveDate, Box<dyn std::error::Error + Sync + Send>> {
        let date_str = match value {
            DBValue::Date(s) => s,
            DBValue::Timestamp(s) => s,
            DBValue::Time(s) => s, // Might contain date part
            DBValue::Str(s) => s,
            _ => &value.to_string(),
        };

        // Try various date formats
        // First try parsing as a full datetime and extract date
        if let Ok(dt) = Self::parse_datetime_string(date_str) {
            return Ok(dt.date());
        }

        // Try parsing as date directly
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            return Ok(date);
        }

        // Try parsing from RFC3339 and extract date
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
            return Ok(dt.date_naive());
        }

        Err(format!("Cannot parse date from: {}", date_str).into())
    }

    /// Extract a NaiveTime from any DBValue that might contain time information
    fn extract_time_from_value(
        value: &DBValue,
    ) -> Result<chrono::NaiveTime, Box<dyn std::error::Error + Sync + Send>> {
        let time_str = match value {
            DBValue::Time(s) => s,
            DBValue::Timestamp(s) => s,
            DBValue::Str(s) => s,
            _ => &value.to_string(),
        };

        // Try various time formats
        // First try parsing as a full datetime and extract time
        if let Ok(dt) = Self::parse_datetime_string(time_str) {
            return Ok(dt.time());
        }

        // Try parsing as time directly
        if let Ok(time) = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S") {
            return Ok(time);
        }

        if let Ok(time) = chrono::NaiveTime::parse_from_str(time_str, "%H:%M:%S%.f") {
            return Ok(time);
        }

        Err(format!("Cannot parse time from: {}", time_str).into())
    }

    /// Extract a NaiveDateTime from any DBValue that might contain datetime information
    fn extract_naive_datetime_from_value(
        value: &DBValue,
    ) -> Result<chrono::NaiveDateTime, Box<dyn std::error::Error + Sync + Send>> {
        let datetime_str = match value {
            DBValue::Timestamp(s) => s,
            DBValue::Date(s) => s,
            DBValue::Str(s) => s,
            _ => &value.to_string(),
        };

        Self::parse_datetime_string(datetime_str)
    }

    /// Extract a DateTime<Utc> from any DBValue that might contain datetime information
    fn extract_utc_datetime_from_value(
        value: &DBValue,
    ) -> Result<chrono::DateTime<chrono::Utc>, Box<dyn std::error::Error + Sync + Send>> {
        let datetime_str = match value {
            DBValue::Timestamp(s) => s,
            DBValue::Date(s) => s,
            DBValue::Str(s) => s,
            _ => &value.to_string(),
        };

        // Try parsing as RFC3339 first (with timezone)
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(datetime_str) {
            return Ok(dt.with_timezone(&chrono::Utc));
        }

        // Fall back to naive datetime and assume UTC
        let naive_dt = Self::parse_datetime_string(datetime_str)?;
        Ok(chrono::DateTime::from_naive_utc_and_offset(
            naive_dt,
            chrono::Utc,
        ))
    }

    /// Parse a datetime string with various common formats
    fn parse_datetime_string(
        s: &str,
    ) -> Result<chrono::NaiveDateTime, Box<dyn std::error::Error + Sync + Send>> {
        // Strip Z suffix if present since NaiveDateTime doesn't handle timezone indicators
        let s_clean = if s.ends_with('Z') {
            &s[..s.len() - 1]
        } else {
            s
        };

        // Try ISO format without fractional seconds
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s_clean, "%Y-%m-%dT%H:%M:%S") {
            return Ok(ndt);
        }

        // Try ISO format with fractional seconds
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s_clean, "%Y-%m-%dT%H:%M:%S%.f") {
            return Ok(ndt);
        }

        // Try space-separated format
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
            return Ok(ndt);
        }

        // Try space-separated format with fractional seconds
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S%.f") {
            return Ok(ndt);
        }

        Err(format!("Cannot parse datetime from: {}", s).into())
    }
}
