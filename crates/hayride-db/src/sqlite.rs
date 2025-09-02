#[cfg(feature = "sqlite")]
use hayride_host_traits::db::{
    errors::ErrorCode, DBConnection, DBRows, DBStatement, IsolationLevel, Rows, Statement,
    Transaction,
};

#[cfg(feature = "sqlite")]
use rusqlite::{params_from_iter, Connection as SqliteConnection};
#[cfg(feature = "sqlite")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "sqlite")]
pub struct SQLiteDBConnection {
    connection: Arc<Mutex<Option<SqliteConnection>>>,
}

#[cfg(feature = "sqlite")]
impl SQLiteDBConnection {
    pub fn new(conn_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Remove sqlite:// prefix if present
        let path = if conn_str.starts_with("sqlite://") {
            &conn_str[9..]
        } else if conn_str.starts_with("file:") {
            &conn_str[5..]
        } else {
            conn_str
        };

        let connection = SqliteConnection::open(path)?;

        Ok(SQLiteDBConnection {
            connection: Arc::new(Mutex::new(Some(connection))),
        })
    }
}

#[cfg(feature = "sqlite")]
impl DBConnection for SQLiteDBConnection {
    fn prepare(&self, query: String) -> Result<Statement, ErrorCode> {
        let connection_guard = self
            .connection
            .lock()
            .map_err(|_| ErrorCode::PrepareFailed)?;
        match connection_guard.as_ref() {
            Some(_conn) => {
                // For SQLite, we'll store the query and prepare it on execution
                let sqlite_statement = SQLiteStatement::new(self.connection.clone(), query);

                let boxed_statement: Box<dyn DBStatement> = Box::new(sqlite_statement);
                Ok(boxed_statement.into())
            }
            None => Err(ErrorCode::PrepareFailed),
        }
    }

    fn begin_transaction(
        &mut self,
        _isolation_level: IsolationLevel,
        _read_only: bool,
    ) -> std::result::Result<Transaction, ErrorCode> {
        // TODO: Implement transaction support for SQLite
        log::warn!("SQLiteDBConnection begin_transaction not yet implemented");
        Err(ErrorCode::NotEnabled)
    }

    fn close(&mut self) -> std::result::Result<(), ErrorCode> {
        let mut connection_guard = self.connection.lock().map_err(|_| ErrorCode::CloseFailed)?;
        if let Some(conn) = connection_guard.take() {
            drop(conn);
            log::debug!("SQLiteDBConnection closed");
        }
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
struct SQLiteStatement {
    connection: Arc<Mutex<Option<SqliteConnection>>>,
    query: String,
}

#[cfg(feature = "sqlite")]
impl SQLiteStatement {
    fn new(connection: Arc<Mutex<Option<SqliteConnection>>>, query: String) -> Self {
        Self { connection, query }
    }
}

#[cfg(feature = "sqlite")]
impl DBStatement for SQLiteStatement {
    fn query(
        &self,
        params: Vec<hayride_host_traits::db::db::DBValue>,
    ) -> std::result::Result<Rows, ErrorCode> {
        let connection_guard = self.connection.lock().map_err(|_| ErrorCode::QueryFailed)?;
        match connection_guard.as_ref() {
            Some(conn) => {
                let mut stmt = conn
                    .prepare(&self.query)
                    .map_err(|_| ErrorCode::QueryFailed)?;

                // Convert DBValues to rusqlite parameters
                let sqlite_params: Vec<rusqlite::types::Value> =
                    params.iter().map(dbvalue_to_sqlite_value).collect();

                let rows = stmt
                    .query_map(params_from_iter(sqlite_params.iter()), |row| {
                        // Convert SQLite row to DBValue row
                        sqlite_row_to_dbvalue_row(row)
                    })
                    .map_err(|_| ErrorCode::QueryFailed)?;

                // Collect all rows (SQLite doesn't support streaming)
                let mut collected_rows = Vec::new();
                for row_result in rows {
                    match row_result {
                        Ok(row) => collected_rows.push(row),
                        Err(e) => {
                            log::warn!("Error reading SQLite row: {}", e);
                            return Err(ErrorCode::QueryFailed);
                        }
                    }
                }

                // Get column names
                let column_names: Vec<String> =
                    stmt.column_names().iter().map(|s| s.to_string()).collect();

                let sqlite_rows = SQLiteRows::new(collected_rows, column_names);
                let boxed_rows: Box<dyn DBRows> = Box::new(sqlite_rows);
                Ok(boxed_rows.into())
            }
            None => Err(ErrorCode::QueryFailed),
        }
    }

    fn execute(
        &self,
        params: Vec<hayride_host_traits::db::db::DBValue>,
    ) -> std::result::Result<u64, ErrorCode> {
        let connection_guard = self
            .connection
            .lock()
            .map_err(|_| ErrorCode::ExecuteFailed)?;
        match connection_guard.as_ref() {
            Some(conn) => {
                let mut stmt = conn
                    .prepare(&self.query)
                    .map_err(|_| ErrorCode::ExecuteFailed)?;

                // Convert DBValues to rusqlite parameters
                let sqlite_params: Vec<rusqlite::types::Value> =
                    params.iter().map(dbvalue_to_sqlite_value).collect();

                let result = stmt
                    .execute(params_from_iter(sqlite_params.iter()))
                    .map_err(|_| ErrorCode::ExecuteFailed)?;
                Ok(result as u64)
            }
            None => Err(ErrorCode::ExecuteFailed),
        }
    }

    fn number_parameters(&self) -> Result<u32, ErrorCode> {
        let connection_guard = self
            .connection
            .lock()
            .map_err(|_| ErrorCode::PrepareFailed)?;
        match connection_guard.as_ref() {
            Some(conn) => {
                let stmt = conn
                    .prepare(&self.query)
                    .map_err(|_| ErrorCode::PrepareFailed)?;
                Ok(stmt.parameter_count() as u32)
            }
            None => Err(ErrorCode::PrepareFailed),
        }
    }

    fn close(&mut self) -> std::result::Result<(), ErrorCode> {
        log::debug!("SQLiteStatement closed (no-op)");
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
struct SQLiteRows {
    rows: Vec<hayride_host_traits::db::db::Row>,
    columns: Vec<String>,
    current_index: usize,
}

#[cfg(feature = "sqlite")]
impl SQLiteRows {
    fn new(rows: Vec<hayride_host_traits::db::db::Row>, columns: Vec<String>) -> Self {
        Self {
            rows,
            columns,
            current_index: 0,
        }
    }
}

#[cfg(feature = "sqlite")]
impl DBRows for SQLiteRows {
    fn columns(&self) -> Vec<String> {
        self.columns.clone()
    }

    fn next(&mut self) -> Result<hayride_host_traits::db::db::Row, ErrorCode> {
        if self.current_index >= self.rows.len() {
            return Err(ErrorCode::EndOfRows);
        }

        let row = self.rows[self.current_index].clone();
        self.current_index += 1;
        Ok(row)
    }

    fn close(&mut self) -> Result<(), ErrorCode> {
        log::debug!("SQLiteRows closed");
        Ok(())
    }
}

#[cfg(feature = "sqlite")]
fn dbvalue_to_sqlite_value(
    dbvalue: &hayride_host_traits::db::db::DBValue,
) -> rusqlite::types::Value {
    use hayride_host_traits::db::db::DBValue;
    use rusqlite::types::Value;

    match dbvalue {
        DBValue::Null => Value::Null,
        DBValue::Int32(i) => Value::Integer(*i as i64),
        DBValue::Int64(i) => Value::Integer(*i),
        DBValue::Uint32(i) => Value::Integer(*i as i64),
        DBValue::Uint64(i) => Value::Integer(*i as i64), // Note: potential overflow
        DBValue::Float(f) => Value::Real(*f),
        DBValue::Double(f) => Value::Real(*f),
        DBValue::Str(s) => Value::Text(s.clone()),
        DBValue::Boolean(b) => Value::Integer(if *b { 1 } else { 0 }),
        DBValue::Date(s) => Value::Text(s.clone()),
        DBValue::Time(s) => Value::Text(s.clone()),
        DBValue::Timestamp(s) => Value::Text(s.clone()),
        DBValue::Binary(b) => Value::Blob(b.clone()),
    }
}

#[cfg(feature = "sqlite")]
fn sqlite_row_to_dbvalue_row(
    row: &rusqlite::Row,
) -> Result<hayride_host_traits::db::db::Row, rusqlite::Error> {
    use hayride_host_traits::db::db::DBValue;

    let mut values = Vec::new();

    for i in 0..row.as_ref().column_count() {
        let value = match row.get_ref(i)? {
            rusqlite::types::ValueRef::Null => DBValue::Null,
            rusqlite::types::ValueRef::Integer(i) => DBValue::Int64(i),
            rusqlite::types::ValueRef::Real(f) => DBValue::Double(f),
            rusqlite::types::ValueRef::Text(s) => {
                DBValue::Str(String::from_utf8_lossy(s).to_string())
            }
            rusqlite::types::ValueRef::Blob(b) => DBValue::Binary(b.to_vec()),
        };
        values.push(value);
    }

    Ok(hayride_host_traits::db::db::Row(values))
}

// Provide a stub implementation when SQLite feature is not enabled
#[cfg(not(feature = "sqlite"))]
pub struct SQLiteDBConnection;

#[cfg(not(feature = "sqlite"))]
impl SQLiteDBConnection {
    pub fn new(_conn_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Err("SQLite support not compiled in. Enable the 'sqlite' feature.".into())
    }
}
