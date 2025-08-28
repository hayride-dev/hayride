use hayride_host_traits::db::{
    errors::ErrorCode, 
    DBConnection, 
    Statement, 
    Transaction, 
    Rows, 
    DBRows, 
    DBStatement, 
    IsolationLevel
};

use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use tokio_postgres::Row;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use futures::stream::Stream;
use futures::StreamExt;
use std::pin::Pin;

use crate::get_db_runtime;

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
                        
                        let postgres_statement = PostgresStatement::new(
                            self.client.clone(),
                            statement,
                        );
                        
                        let boxed_statement: Box<dyn DBStatement> = Box::new(postgres_statement);
                        Ok(boxed_statement.into())
                    },
                    None => Err(ErrorCode::PrepareFailed),
                }
            })
        })
    }

    fn begin_transaction(&mut self, _isolation_level: IsolationLevel, _read_only: bool) -> std::result::Result<Transaction, ErrorCode> {
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
    fn new(client: Arc<Mutex<Option<tokio_postgres::Client>>>, statement: tokio_postgres::Statement) -> Self {
        Self {
            client,
            statement,
        }
    }
}

impl DBStatement for PostgresStatement {
    fn query(&self, params: Vec<hayride_host_traits::db::db::DBValue>) -> std::result::Result<Rows, ErrorCode> {
        tokio::task::block_in_place(|| {
            let rt = get_db_runtime();
            rt.block_on(async {
                let client_guard = self.client.lock().await;
                match client_guard.as_ref() {
                    Some(client) => {
                        // Convert DBValues to ToSql references for parameter passing
                        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter()
                            .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                            .collect();

                        let stream = client.query_raw(&self.statement, param_refs).await.map_err(|e| {
                            log::warn!("PostgresStatement Query failed with error: {}", e);
                            ErrorCode::QueryFailed
                        })?;

                        // Get column information from the prepared statement
                        let columns: Vec<String> = self.statement.columns().iter()
                            .map(|col| col.name().to_string())
                            .collect();

                        log::debug!("PostgresStatement Query executed successfully, streaming results");

                        let boxed_stream: Pin<Box<dyn Stream<Item = Result<tokio_postgres::Row, tokio_postgres::Error>> + Send + Sync>> = Box::pin(stream);
                        let postgres_rows = PostgresRows::new(boxed_stream, columns);
                        let boxed_rows: Box<dyn DBRows> = Box::new(postgres_rows);
                        Ok(boxed_rows.into())
                    },
                    None => Err(ErrorCode::QueryFailed),
                }
            })
        })
    }

    fn execute(&self, params: Vec<hayride_host_traits::db::db::DBValue>) -> std::result::Result<u64, ErrorCode> {
        tokio::task::block_in_place(|| {
            let rt = get_db_runtime();
            rt.block_on(async {
                let client_guard = self.client.lock().await;
                match client_guard.as_ref() {
                    Some(client) => {
                        // Convert DBValues to ToSql references for parameter passing
                        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter()
                            .map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync))
                            .collect();

                        let result = client.execute(&self.statement, &param_refs).await.map_err(|_| ErrorCode::ExecuteFailed)?;
                        Ok(result)
                    },
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
    stream: Pin<Box<dyn Stream<Item = Result<tokio_postgres::Row, tokio_postgres::Error>> + Send + Sync>>,
    columns: Vec<String>,
    finished: bool,
}

impl PostgresRows {
    fn new(stream: Pin<Box<dyn Stream<Item = Result<tokio_postgres::Row, tokio_postgres::Error>> + Send + Sync>>, columns: Vec<String>) -> Self {
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
                    },
                    Some(Err(e)) => {
                        log::warn!("Error reading row from stream: {}", e);
                        self.finished = true;
                        Err(ErrorCode::QueryFailed)
                    },
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
        Type::BOOL => {
            match row.try_get::<_, bool>(col_idx) {
                Ok(val) => DBValue::Boolean(val),
                Err(_) => DBValue::Null,
            }
        },
        Type::INT2 => {
            match row.try_get::<_, i16>(col_idx) {
                Ok(val) => DBValue::Int32(val as i32),
                Err(_) => DBValue::Null,
            }
        },
        Type::INT4 => {
            match row.try_get::<_, i32>(col_idx) {
                Ok(val) => DBValue::Int32(val),
                Err(_) => DBValue::Null,
            }
        },
        Type::INT8 => {
            match row.try_get::<_, i64>(col_idx) {
                Ok(val) => DBValue::Int64(val),
                Err(_) => DBValue::Null,
            }
        },
        Type::FLOAT4 => {
            match row.try_get::<_, f32>(col_idx) {
                Ok(val) => DBValue::Float(val as f64),
                Err(_) => DBValue::Null,
            }
        },
        Type::FLOAT8 => {
            match row.try_get::<_, f64>(col_idx) {
                Ok(val) => DBValue::Double(val),
                Err(_) => DBValue::Null,
            }
        },
        Type::TEXT | Type::VARCHAR | Type::CHAR | Type::NAME => {
            match row.try_get::<_, String>(col_idx) {
                Ok(val) => DBValue::Str(val),
                Err(_) => DBValue::Null,
            }
        },
        Type::BYTEA => {
            match row.try_get::<_, Vec<u8>>(col_idx) {
                Ok(val) => DBValue::Binary(val),
                Err(_) => DBValue::Null,
            }
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
        },
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
        },
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
        },
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
        },
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
        },
        Type::JSON | Type::JSONB => {
            match row.try_get::<_, serde_json::Value>(col_idx) {
                Ok(val) => {
                    match serde_json::to_vec(&val) {
                        Ok(bytes) => DBValue::Binary(bytes),
                        Err(_) => DBValue::Null,
                    }
                },
                Err(_) => {
                    // Fallback to string and convert to bytes
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Binary(val.into_bytes()),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        },
        Type::NUMERIC => {
            // Fallback to string representation
            match row.try_get::<_, String>(col_idx) {
                Ok(val) => DBValue::Str(val),
                Err(_) => DBValue::Null,
            }
        },
        _ => {
            // For any other type, try to convert to string
            match row.try_get::<_, String>(col_idx) {
                Ok(val) => DBValue::Str(val),
                Err(_) => DBValue::Null,
            }
        }
    }
}
