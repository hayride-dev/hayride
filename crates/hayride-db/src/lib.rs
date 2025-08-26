use anyhow::Result;

use hayride_host_traits::db::{errors::ErrorCode, DBTrait, Connection, DBConnection};
use hayride_host_traits::db::db::ColumnInfo;

use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use tokio_postgres::Row;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::runtime::Runtime;
use std::sync::OnceLock;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct DBBackend {
}

// Global runtime for database operations
static DB_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_db_runtime() -> &'static Runtime {
    DB_RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create database runtime")
    })
}

impl DBBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl DBTrait for DBBackend {
    fn connect(&mut self, _config: hayride_host_traits::db::db::DBConfig) -> std::result::Result<Connection, ErrorCode> {
        // TODO: Use config to create connection
        Err(ErrorCode::NotEnabled)
    }

    fn connect_string(&mut self, connection_string: String) -> Result<Connection, ErrorCode> {
         tokio::task::block_in_place(|| {
            let rt = get_db_runtime();
            let db = rt.block_on(PostgresDBConnection::new(&connection_string))
                .map_err(|_| ErrorCode::ConnectionFailed)?;

            let connection: Box<dyn DBConnection> = Box::new(db);
            return Ok(connection.into());
        })
    }

}


struct PostgresDBConnection {
    client: Arc<Mutex<Option<tokio_postgres::Client>>>,
    cancellation_token: CancellationToken,
}

impl PostgresDBConnection {
    async fn new(conn_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
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

/// Convert a tokio_postgres::Row to a hayride Row containing DBValues
fn row_to_dbvalue_row(row: &Row) -> hayride_host_traits::db::db::Row {
    let mut values = Vec::new();
    
    for i in 0..row.len() {
        let value = postgres_value_to_dbvalue(row, i);
        values.push(value);
    }
    
    hayride_host_traits::db::db::Row(values)
}

/// Extract column information from a postgres row
fn extract_column_info(row: &Row) -> Vec<ColumnInfo> {
    row.columns().iter().map(|col| {
        let pg_type = col.type_();
        ColumnInfo {
            name: col.name().to_string(),
            type_oid: pg_type.oid(),
            type_name: pg_type.name().to_string(),
            nullable: true, // PostgreSQL doesn't provide nullable info in query results
        }
    }).collect()
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
                Ok(val) => DBValue::Int16(val),
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
                Ok(val) => DBValue::Float32(val),
                Err(_) => DBValue::Null,
            }
        },
        Type::FLOAT8 => {
            match row.try_get::<_, f64>(col_idx) {
                Ok(val) => DBValue::Float64(val),
                Err(_) => DBValue::Null,
            }
        },
        Type::TEXT | Type::VARCHAR | Type::CHAR | Type::NAME => {
            match row.try_get::<_, String>(col_idx) {
                Ok(val) => DBValue::Text(val),
                Err(_) => DBValue::Null,
            }
        },
        Type::BYTEA => {
            match row.try_get::<_, Vec<u8>>(col_idx) {
                Ok(val) => DBValue::Bytes(val),
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
                Ok(val) => DBValue::TimestampTz(val.to_rfc3339()),
                Err(_) => {
                    // Fallback to string
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::TimestampTz(val),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        },
        Type::UUID => {
            match row.try_get::<_, uuid::Uuid>(col_idx) {
                Ok(val) => DBValue::Uuid(val.to_string()),
                Err(_) => {
                    // Fallback to string
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Uuid(val),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        },
        Type::JSON | Type::JSONB => {
            match row.try_get::<_, serde_json::Value>(col_idx) {
                Ok(val) => {
                    match serde_json::to_vec(&val) {
                        Ok(bytes) => DBValue::Json(bytes),
                        Err(_) => DBValue::Null,
                    }
                },
                Err(_) => {
                    // Fallback to string and convert to bytes
                    match row.try_get::<_, String>(col_idx) {
                        Ok(val) => DBValue::Json(val.into_bytes()),
                        Err(_) => DBValue::Null,
                    }
                }
            }
        },
        Type::NUMERIC => {
            // Fallback to string representation
            match row.try_get::<_, String>(col_idx) {
                Ok(val) => DBValue::Numeric(val),
                Err(_) => DBValue::Null,
            }
        },
        _ => {
            // For any other type, try to convert to string
            match row.try_get::<_, String>(col_idx) {
                Ok(val) => DBValue::Custom(val),
                Err(_) => DBValue::Null,
            }
        }
    }
}

impl DBConnection for PostgresDBConnection {
    fn query(
            &self,
            statement: String,
            params: Vec<hayride_host_traits::db::db::DBValue>,
        ) -> std::result::Result<hayride_host_traits::db::QueryResult, ErrorCode> {
            tokio::task::block_in_place(|| {
                let rt = get_db_runtime();
                rt.block_on(async {
                    let client_guard = self.client.lock().await;
                    match client_guard.as_ref() {
                        Some(client) => {
                            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
                            let rows = client.query(&statement, &param_refs[..]).await.map_err(|e| {
                                log::warn!("PostgresDBConnection Query failed with error: {}", e);
                                ErrorCode::QueryFailed
                            })?;

                            log::debug!("PostgresDBConnection Query executed successfully, processing {} rows", rows.len());

                            if rows.is_empty() {
                                return Ok(hayride_host_traits::db::QueryResult {
                                    columns: vec![],
                                    rows: vec![],
                                });
                            }

                            // Extract column information from the first row
                            let columns = extract_column_info(&rows[0]);
                            
                            // Convert all rows to DBValue rows
                            let converted_rows: Vec<hayride_host_traits::db::db::Row> = rows.iter()
                                .map(|row| row_to_dbvalue_row(row))
                                .collect();

                            let query_result = hayride_host_traits::db::QueryResult {
                                columns,
                                rows: converted_rows,
                            };
                            Ok(query_result)
                        },
                        None => Err(ErrorCode::ConnectionFailed),
                    }
                })
            })
        }

    fn execute(
            &self,
            statement: String,
            params: Vec<hayride_host_traits::db::db::DBValue>,
        ) -> std::result::Result<u64, ErrorCode> {
            tokio::task::block_in_place(|| {
                let rt = get_db_runtime();
                rt.block_on(async {
                    let client_guard = self.client.lock().await;
                    match client_guard.as_ref() {
                        Some(client) => {
                            let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
                            let result = client.execute(&statement, &param_refs[..]).await.map_err(|_| ErrorCode::QueryFailed)?;
                            Ok(result)
                        },
                        None => Err(ErrorCode::ConnectionFailed),
                    }
                })
            })
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