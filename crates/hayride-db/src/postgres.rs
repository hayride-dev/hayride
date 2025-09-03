use hayride_host_traits::db::{
    errors::ErrorCode, DBConnection, DBRows, DBStatement, IsolationLevel, Rows, Statement,
    Transaction,
};

use futures::stream::Stream;
use futures::StreamExt;
use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_postgres::Row;
use tokio_util::sync::CancellationToken;

use crate::get_db_runtime;

// PostgreSQL-specific trait implementations for DBValue
use hayride_host_traits::db::db::DBValue;
use postgres_types::{IsNull, ToSql, Type};

/// Wrapper for DBValue to implement PostgreSQL ToSql trait
#[derive(Debug)]
struct PostgresDBValue<'a>(&'a DBValue);

impl<'a> ToSql for PostgresDBValue<'a> {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut bytes::BytesMut,
    ) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        // Handle NULL values first, regardless of type
        if matches!(self.0, DBValue::Null) {
            return Ok(IsNull::Yes);
        }

        // Primary dispatch based on PostgreSQL column type
        match *ty {
            // Numeric types
            Type::INT2 => match self.0 {
                DBValue::Int32(i) => (*i as i16).to_sql(ty, out),
                DBValue::Int64(i) => (*i as i16).to_sql(ty, out),
                DBValue::Uint32(u) => (*u as i16).to_sql(ty, out),
                DBValue::Uint64(u) => (*u as i16).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<i16>()?.to_sql(ty, out),
                _ => self.0.to_string().parse::<i16>()?.to_sql(ty, out),
            },
            Type::INT4 => match self.0 {
                DBValue::Int32(i) => i.to_sql(ty, out),
                DBValue::Int64(i) => (*i as i32).to_sql(ty, out),
                DBValue::Uint32(u) => (*u as i32).to_sql(ty, out),
                DBValue::Uint64(u) => (*u as i32).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<i32>()?.to_sql(ty, out),
                _ => self.0.to_string().parse::<i32>()?.to_sql(ty, out),
            },
            Type::INT8 => match self.0 {
                DBValue::Int64(i) => i.to_sql(ty, out),
                DBValue::Int32(i) => (*i as i64).to_sql(ty, out),
                DBValue::Uint32(u) => (*u as i64).to_sql(ty, out),
                DBValue::Uint64(u) => (*u as i64).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<i64>()?.to_sql(ty, out),
                _ => self.0.to_string().parse::<i64>()?.to_sql(ty, out),
            },
            Type::FLOAT4 => match self.0 {
                DBValue::Float(f) => (*f as f32).to_sql(ty, out),
                DBValue::Double(f) => (*f as f32).to_sql(ty, out),
                DBValue::Int32(i) => (*i as f32).to_sql(ty, out),
                DBValue::Int64(i) => (*i as f32).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<f32>()?.to_sql(ty, out),
                _ => self.0.to_string().parse::<f32>()?.to_sql(ty, out),
            },
            Type::FLOAT8 => match self.0 {
                DBValue::Double(f) => f.to_sql(ty, out),
                DBValue::Float(f) => f.to_sql(ty, out),
                DBValue::Int32(i) => (*i as f64).to_sql(ty, out),
                DBValue::Int64(i) => (*i as f64).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<f64>()?.to_sql(ty, out),
                _ => self.0.to_string().parse::<f64>()?.to_sql(ty, out),
            },

            // Boolean type
            Type::BOOL => match self.0 {
                DBValue::Boolean(b) => b.to_sql(ty, out),
                DBValue::Int32(i) => (*i != 0).to_sql(ty, out),
                DBValue::Str(s) => s.parse::<bool>()?.to_sql(ty, out),
                _ => self.0.to_string().parse::<bool>()?.to_sql(ty, out),
            },

            // String types
            Type::TEXT | Type::VARCHAR | Type::CHAR | Type::NAME => {
                let string_value = match self.0 {
                    DBValue::Str(s) => s.clone(),
                    _ => self.0.to_string(),
                };
                string_value.to_sql(ty, out)
            }

            // Binary type
            Type::BYTEA => match self.0 {
                DBValue::Binary(b) => b.to_sql(ty, out),
                DBValue::Str(s) => s.as_bytes().to_vec().to_sql(ty, out),
                _ => self.0.to_string().as_bytes().to_vec().to_sql(ty, out),
            },

            // Date type - extract date from any temporal value
            Type::DATE => {
                let date = extract_date_from_value(self.0)?;
                date.to_sql(ty, out)
            }

            // Time type - extract time from any temporal value
            Type::TIME => {
                let time = extract_time_from_value(self.0)?;
                time.to_sql(ty, out)
            }

            // Timestamp type (no timezone)
            Type::TIMESTAMP => {
                let datetime = extract_naive_datetime_from_value(self.0)?;
                datetime.to_sql(ty, out)
            }

            // Timestamp with timezone
            Type::TIMESTAMPTZ => {
                let datetime = extract_utc_datetime_from_value(self.0)?;
                datetime.to_sql(ty, out)
            }

            // UUID type
            Type::UUID => match self.0 {
                DBValue::Str(s) => {
                    let uuid = uuid::Uuid::parse_str(s)?;
                    uuid.to_sql(ty, out)
                }
                _ => {
                    let uuid = uuid::Uuid::parse_str(&self.0.to_string())?;
                    uuid.to_sql(ty, out)
                }
            },

            // JSON types
            Type::JSON | Type::JSONB => match self.0 {
                DBValue::Binary(b) => {
                    let json_value: serde_json::Value = serde_json::from_slice(b)?;
                    json_value.to_sql(ty, out)
                }
                DBValue::Str(s) => {
                    let json_value: serde_json::Value = serde_json::from_str(s)?;
                    json_value.to_sql(ty, out)
                }
                _ => {
                    let json_value: serde_json::Value = serde_json::from_str(&self.0.to_string())?;
                    json_value.to_sql(ty, out)
                }
            },

            // Numeric/Decimal type
            Type::NUMERIC => match self.0 {
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
                    let decimal = rust_decimal::Decimal::from_str(&self.0.to_string())?;
                    decimal.to_sql(ty, out)
                }
            },

            // Default fallback - try to convert to string
            _ => {
                let string_value = match self.0 {
                    DBValue::Str(s) => s.clone(),
                    _ => self.0.to_string(),
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
    if let Ok(dt) = parse_datetime_string(date_str) {
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
    if let Ok(dt) = parse_datetime_string(time_str) {
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

    parse_datetime_string(datetime_str)
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
    let naive_dt = parse_datetime_string(datetime_str)?;
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

pub struct PostgresDBConnection {
    client: Arc<Mutex<Option<tokio_postgres::Client>>>,
    cancellation_token: CancellationToken,
}

impl PostgresDBConnection {
    pub async fn new(conn_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tls = {
            let builder = TlsConnector::builder();
            MakeTlsConnector::new(builder.build()?)
        };

        let (client, connection) = tokio_postgres::connect(&conn_str, tls).await?;
        let cancellation_token = CancellationToken::new();
        let cancel_clone = cancellation_token.clone();

        // Drive the connection on a background task with cancellation support
        tokio::spawn(async move {
            tokio::select! {
                result = connection => {
                    if let Err(e) = result {
                        log::debug!("PostgresDBConnection error: {e}");
                    }
                }
                _ = cancel_clone.cancelled() => {
                    log::debug!("PostgresDBConnection task cancelled");
                }
            }
        });

        Ok(PostgresDBConnection {
            client: Arc::new(Mutex::new(Some(client))),
            cancellation_token,
        })
    }
}

impl Drop for PostgresDBConnection {
    fn drop(&mut self) {
        // Signal cancellation when the connection is dropped
        self.cancellation_token.cancel();
        log::debug!("PostgresDBConnection dropped, cancellation token triggered");
    }
}

impl DBConnection for PostgresDBConnection {
    fn prepare(&self, query: String) -> Result<Statement, ErrorCode> {
        tokio::task::block_in_place(|| {
            let rt = get_db_runtime();
            rt.block_on(async {
                let client_guard = self.client.lock().await;
                match client_guard.as_ref() {
                    Some(client) => {
                        let statement = client.prepare(&query).await.map_err(|e| {
                            log::warn!("PostgresDBConnection prepare failed with error: {}", e);
                            ErrorCode::PrepareFailed
                        })?;

                        let postgres_statement =
                            PostgresStatement::new(self.client.clone(), statement);

                        let boxed_statement: Box<dyn DBStatement> = Box::new(postgres_statement);
                        Ok(boxed_statement.into())
                    }
                    None => Err(ErrorCode::PrepareFailed),
                }
            })
        })
    }

    fn begin_transaction(
        &mut self,
        _isolation_level: IsolationLevel,
        _read_only: bool,
    ) -> std::result::Result<Transaction, ErrorCode> {
        // TODO: Handle transactions properly with tokio-postgres
        log::warn!("PostgresDBConnection begin_transaction not fully implemented");
        Err(ErrorCode::NotEnabled)
    }

    fn close(&mut self) -> std::result::Result<(), ErrorCode> {
        tokio::task::block_in_place(|| {
            let rt = get_db_runtime();
            rt.block_on(async {
                // Signal the background task to stop
                self.cancellation_token.cancel();

                // Close the client connection
                let mut client_guard = self.client.lock().await;
                if let Some(client) = client_guard.take() {
                    // The client will be dropped here, which closes the connection
                    drop(client);
                    log::debug!("PostgresDBConnection closed");
                }

                Ok(())
            })
        })
    }
}

struct PostgresStatement {
    client: Arc<Mutex<Option<tokio_postgres::Client>>>,
    statement: tokio_postgres::Statement,
}

impl PostgresStatement {
    fn new(
        client: Arc<Mutex<Option<tokio_postgres::Client>>>,
        statement: tokio_postgres::Statement,
    ) -> Self {
        Self { client, statement }
    }
}

impl DBStatement for PostgresStatement {
    fn query(
        &self,
        params: Vec<hayride_host_traits::db::db::DBValue>,
    ) -> std::result::Result<Rows, ErrorCode> {
        tokio::task::block_in_place(|| {
            let rt = get_db_runtime();
            rt.block_on(async {
                let client_guard = self.client.lock().await;
                match client_guard.as_ref() {
                    Some(client) => {
                        // Convert DBValues to ToSql references for parameter passing
                        let wrapped_params: Vec<PostgresDBValue> =
                            params.iter().map(|p| PostgresDBValue(p)).collect();
                        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                            wrapped_params
                                .iter()
                                .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                                .collect();

                        let stream = client
                            .query_raw(&self.statement, param_refs)
                            .await
                            .map_err(|e| {
                                log::warn!("PostgresStatement Query failed with error: {}", e);
                                ErrorCode::QueryFailed
                            })?;

                        // Get column information from the prepared statement
                        let columns: Vec<String> = self
                            .statement
                            .columns()
                            .iter()
                            .map(|col| col.name().to_string())
                            .collect();

                        log::debug!(
                            "PostgresStatement Query executed successfully, streaming results"
                        );

                        let boxed_stream: Pin<
                            Box<
                                dyn Stream<
                                        Item = Result<tokio_postgres::Row, tokio_postgres::Error>,
                                    > + Send
                                    + Sync,
                            >,
                        > = Box::pin(stream);
                        let postgres_rows = PostgresRows::new(boxed_stream, columns);
                        let boxed_rows: Box<dyn DBRows> = Box::new(postgres_rows);
                        Ok(boxed_rows.into())
                    }
                    None => Err(ErrorCode::QueryFailed),
                }
            })
        })
    }

    fn execute(
        &self,
        params: Vec<hayride_host_traits::db::db::DBValue>,
    ) -> std::result::Result<u64, ErrorCode> {
        tokio::task::block_in_place(|| {
            let rt = get_db_runtime();
            rt.block_on(async {
                let client_guard = self.client.lock().await;
                match client_guard.as_ref() {
                    Some(client) => {
                        // Convert DBValues to ToSql references for parameter passing
                        let wrapped_params: Vec<PostgresDBValue> =
                            params.iter().map(|p| PostgresDBValue(p)).collect();
                        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
                            wrapped_params
                                .iter()
                                .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                                .collect();

                        let result = client
                            .execute(&self.statement, &param_refs)
                            .await
                            .map_err(|_| ErrorCode::ExecuteFailed)?;
                        Ok(result)
                    }
                    None => Err(ErrorCode::ExecuteFailed),
                }
            })
        })
    }

    fn number_parameters(&self) -> Result<u32, ErrorCode> {
        Ok(self.statement.params().len() as u32)
    }

    fn close(&mut self) -> std::result::Result<(), ErrorCode> {
        log::debug!("PostgresStatement closed (no-op)");
        Ok(())
    }
}

struct PostgresRows {
    stream: Pin<
        Box<dyn Stream<Item = Result<tokio_postgres::Row, tokio_postgres::Error>> + Send + Sync>,
    >,
    columns: Vec<String>,
    finished: bool,
}

impl PostgresRows {
    fn new(
        stream: Pin<
            Box<
                dyn Stream<Item = Result<tokio_postgres::Row, tokio_postgres::Error>> + Send + Sync,
            >,
        >,
        columns: Vec<String>,
    ) -> Self {
        Self {
            stream,
            columns,
            finished: false,
        }
    }
}

impl DBRows for PostgresRows {
    fn columns(&self) -> Vec<String> {
        self.columns.clone()
    }

    fn next(&mut self) -> Result<hayride_host_traits::db::db::Row, ErrorCode> {
        if self.finished {
            return Err(ErrorCode::EndOfRows);
        }

        tokio::task::block_in_place(|| {
            let rt = get_db_runtime();
            rt.block_on(async {
                match self.stream.next().await {
                    Some(Ok(row)) => {
                        let db_row = row_to_dbvalue_row(&row);
                        Ok(db_row)
                    }
                    Some(Err(e)) => {
                        log::warn!("Error reading row from stream: {}", e);
                        self.finished = true;
                        Err(ErrorCode::QueryFailed)
                    }
                    None => {
                        self.finished = true;
                        Err(ErrorCode::EndOfRows)
                    }
                }
            })
        })
    }

    fn close(&mut self) -> Result<(), ErrorCode> {
        self.finished = true;
        log::debug!("PostgresRows closed");
        Ok(())
    }
}

/// Convert a tokio_postgres::Row to a hayride Row containing DBValues
fn row_to_dbvalue_row(row: &Row) -> hayride_host_traits::db::db::Row {
    let mut values = Vec::new();

    for i in 0..row.len() {
        let value = postgres_value_to_dbvalue(row, i);
        values.push(value);
    }

    hayride_host_traits::db::db::Row(values)
}

/// Convert a PostgreSQL value at a specific column index to DBValue
fn postgres_value_to_dbvalue(row: &Row, col_idx: usize) -> hayride_host_traits::db::db::DBValue {
    use hayride_host_traits::db::db::DBValue;
    use tokio_postgres::types::Type;

    let column = &row.columns()[col_idx];
    let pg_type = column.type_();

    // Handle NULL values first
    if let Ok(None) = row.try_get::<_, Option<String>>(col_idx) {
        return DBValue::Null;
    }

    match *pg_type {
        Type::BOOL => match row.try_get::<_, bool>(col_idx) {
            Ok(val) => DBValue::Boolean(val),
            Err(_) => DBValue::Null,
        },
        Type::INT2 => match row.try_get::<_, i16>(col_idx) {
            Ok(val) => DBValue::Int32(val as i32),
            Err(_) => DBValue::Null,
        },
        Type::INT4 => match row.try_get::<_, i32>(col_idx) {
            Ok(val) => DBValue::Int32(val),
            Err(_) => DBValue::Null,
        },
        Type::INT8 => match row.try_get::<_, i64>(col_idx) {
            Ok(val) => DBValue::Int64(val),
            Err(_) => DBValue::Null,
        },
        Type::FLOAT4 => match row.try_get::<_, f32>(col_idx) {
            Ok(val) => DBValue::Float(val as f64),
            Err(_) => DBValue::Null,
        },
        Type::FLOAT8 => match row.try_get::<_, f64>(col_idx) {
            Ok(val) => DBValue::Double(val),
            Err(_) => DBValue::Null,
        },
        Type::TEXT | Type::VARCHAR | Type::CHAR | Type::NAME => {
            match row.try_get::<_, String>(col_idx) {
                Ok(val) => DBValue::Str(val),
                Err(_) => DBValue::Null,
            }
        }
        Type::BYTEA => match row.try_get::<_, Vec<u8>>(col_idx) {
            Ok(val) => DBValue::Binary(val),
            Err(_) => DBValue::Null,
        },
        Type::DATE => {
            match row.try_get::<_, chrono::NaiveDate>(col_idx) {
                Ok(val) => DBValue::Date(val.to_string()),
                Err(_) => {
                    // Fallback to string
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Date(val),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        }
        Type::TIME => {
            match row.try_get::<_, chrono::NaiveTime>(col_idx) {
                Ok(val) => DBValue::Time(val.to_string()),
                Err(_) => {
                    // Fallback to string
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Time(val),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        }
        Type::TIMESTAMP => {
            match row.try_get::<_, chrono::NaiveDateTime>(col_idx) {
                Ok(val) => DBValue::Timestamp(val.to_string()),
                Err(_) => {
                    // Fallback to string
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Timestamp(val),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        }
        Type::TIMESTAMPTZ => {
            match row.try_get::<_, chrono::DateTime<chrono::Utc>>(col_idx) {
                Ok(val) => DBValue::Timestamp(val.to_rfc3339()),
                Err(_) => {
                    // Fallback to string
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Timestamp(val),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        }
        Type::UUID => {
            match row.try_get::<_, uuid::Uuid>(col_idx) {
                Ok(val) => DBValue::Str(val.to_string()),
                Err(_) => {
                    // Fallback to string
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Str(val),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        }
        Type::JSON | Type::JSONB => {
            match row.try_get::<_, serde_json::Value>(col_idx) {
                Ok(val) => match serde_json::to_vec(&val) {
                    Ok(bytes) => DBValue::Binary(bytes),
                    Err(_) => DBValue::Null,
                },
                Err(_) => {
                    // Fallback to string and convert to bytes
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Binary(val.into_bytes()),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        }
        Type::NUMERIC => {
            // Preferred: exact decimal
            if let Ok(val) = row.try_get::<_, rust_decimal::Decimal>(col_idx) {
                return hayride_host_traits::db::db::DBValue::Str(val.normalize().to_string());
            }

            if let Ok(val) = row.try_get::<_, f64>(col_idx) {
                return hayride_host_traits::db::db::DBValue::Double(val);
            }
            hayride_host_traits::db::db::DBValue::Null
        }
        _ => {
            // For any other type, try to convert to string
            match row.try_get::<_, String>(col_idx) {
                Ok(val) => DBValue::Str(val),
                Err(_) => DBValue::Null,
            }
        }
    }
}
