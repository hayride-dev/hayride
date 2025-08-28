use anyhow::Result;
use hayride_host_traits::db::{errors::ErrorCode, DBTrait, Connection, DBConnection};
use tokio::runtime::Runtime;
use std::sync::OnceLock;

pub mod postgres;
pub mod connection_string;
pub mod sqlite;

use connection_string::{ConnectionStringParser, DatabaseType};

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

    /// Create a database connection based on the connection string
    fn create_connection(&self, connection_string: &str) -> Result<Box<dyn DBConnection>, ErrorCode> {
        let parser = ConnectionStringParser::new(connection_string);
        let db_type = parser.get_database_type().map_err(|_| ErrorCode::OpenFailed)?;
        
        match db_type {
            DatabaseType::PostgreSQL => {
                tokio::task::block_in_place(|| {
                    let rt = get_db_runtime();
                    rt.block_on(async {
                        postgres::PostgresDBConnection::new(connection_string)
                            .await
                            .map(|conn| Box::new(conn) as Box<dyn DBConnection>)
                            .map_err(|_| ErrorCode::OpenFailed)
                    })
                })
            },
            DatabaseType::SQLite => {
                #[cfg(feature = "sqlite")]
                {
                    sqlite::SQLiteDBConnection::new(connection_string)
                        .map(|conn| Box::new(conn) as Box<dyn DBConnection>)
                        .map_err(|_| ErrorCode::OpenFailed)
                }
                #[cfg(not(feature = "sqlite"))]
                {
                    log::warn!("SQLite support not compiled in. Enable the 'sqlite' feature.");
                    Err(ErrorCode::NotEnabled)
                }
            },
            DatabaseType::MySQL => {
                // TODO: Implement MySQL support  
                log::warn!("MySQL support not yet implemented");
                Err(ErrorCode::NotEnabled)
            },
            DatabaseType::Unknown => {
                log::error!("Unknown database type in connection string: {}", connection_string);
                Err(ErrorCode::OpenFailed)
            }
        }
    }
}

impl DBTrait for DBBackend {
    fn open(&mut self, connection_string: String) -> Result<Connection, ErrorCode> {
        let connection = self.create_connection(&connection_string)?;
        Ok(connection.into())
    }
}
