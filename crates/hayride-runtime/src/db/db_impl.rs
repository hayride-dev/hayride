use crate::db::bindings::{db::ErrorCode, db};
use crate::db::{DBImpl, DBView};
use hayride_host_traits::db::{Error, Connection, DBConfig, QueryResult as HostQueryResult};
use hayride_host_traits::db::db::{DBValue as HostDBValue, ArrayElement as HostArrayElement};

use wasmtime::component::Resource;
use wasmtime::Result;

use anyhow::anyhow;

// Conversion functions between WIT types and host trait types
fn convert_db_value_to_host(value: db::DbValue) -> HostDBValue {
    match value {
        db::DbValue::Null => HostDBValue::Null,
        db::DbValue::Boolean(b) => HostDBValue::Boolean(b),
        db::DbValue::Int16(i) => HostDBValue::Int16(i),
        db::DbValue::Int32(i) => HostDBValue::Int32(i),
        db::DbValue::Int64(i) => HostDBValue::Int64(i),
        db::DbValue::Float32(f) => HostDBValue::Float32(f),
        db::DbValue::Float64(f) => HostDBValue::Float64(f),
        db::DbValue::Text(s) => HostDBValue::Text(s),
        db::DbValue::Bytes(b) => HostDBValue::Bytes(b),
        db::DbValue::Date(s) => HostDBValue::Date(s),
        db::DbValue::Time(s) => HostDBValue::Time(s),
        db::DbValue::Timestamp(s) => HostDBValue::Timestamp(s),
        db::DbValue::Timestamptz(s) => HostDBValue::TimestampTz(s),
        db::DbValue::Uuid(s) => HostDBValue::Uuid(s),
        db::DbValue::Json(b) => HostDBValue::Json(b),
        db::DbValue::Array(arr) => {
            let host_arr: Vec<HostArrayElement> = arr.into_iter().map(convert_array_element_to_host).collect();
            HostDBValue::Array(host_arr)
        },
        db::DbValue::Numeric(s) => HostDBValue::Numeric(s),
        db::DbValue::Custom(s) => HostDBValue::Custom(s),
    }
}

fn convert_array_element_to_host(element: db::ArrayElement) -> HostArrayElement {
    match element {
        db::ArrayElement::Null => HostArrayElement::Null,
        db::ArrayElement::Boolean(b) => HostArrayElement::Boolean(b),
        db::ArrayElement::Int16(i) => HostArrayElement::Int16(i),
        db::ArrayElement::Int32(i) => HostArrayElement::Int32(i),
        db::ArrayElement::Int64(i) => HostArrayElement::Int64(i),
        db::ArrayElement::Float32(f) => HostArrayElement::Float32(f),
        db::ArrayElement::Float64(f) => HostArrayElement::Float64(f),
        db::ArrayElement::Text(s) => HostArrayElement::Text(s),
        db::ArrayElement::Bytes(b) => HostArrayElement::Bytes(b),
        db::ArrayElement::Date(s) => HostArrayElement::Date(s),
        db::ArrayElement::Time(s) => HostArrayElement::Time(s),
        db::ArrayElement::Timestamp(s) => HostArrayElement::Timestamp(s),
        db::ArrayElement::Timestamptz(s) => HostArrayElement::TimestampTz(s),
        db::ArrayElement::Uuid(s) => HostArrayElement::Uuid(s),
        db::ArrayElement::Json(b) => HostArrayElement::Json(b),
        db::ArrayElement::Numeric(s) => HostArrayElement::Numeric(s),
        db::ArrayElement::Custom(s) => HostArrayElement::Custom(s),
    }
}

fn convert_host_result_to_wit(result: HostQueryResult) -> db::QueryResult {
    let columns: Vec<db::ColumnInfo> = result.columns.into_iter().map(|col| db::ColumnInfo {
        name: col.name,
        type_oid: Some(col.type_oid),
        type_name: Some(col.type_name),
        nullable: Some(col.nullable),
    }).collect();

    let rows: Vec<db::Row> = result.rows.into_iter().map(|row| {
        row.0.into_iter().map(convert_host_db_value_to_wit).collect()
    }).collect();

    db::QueryResult { columns, rows }
}

fn convert_host_db_value_to_wit(value: HostDBValue) -> db::DbValue {
    match value {
        HostDBValue::Null => db::DbValue::Null,
        HostDBValue::Boolean(b) => db::DbValue::Boolean(b),
        HostDBValue::Int16(i) => db::DbValue::Int16(i),
        HostDBValue::Int32(i) => db::DbValue::Int32(i),
        HostDBValue::Int64(i) => db::DbValue::Int64(i),
        HostDBValue::Float32(f) => db::DbValue::Float32(f),
        HostDBValue::Float64(f) => db::DbValue::Float64(f),
        HostDBValue::Text(s) => db::DbValue::Text(s),
        HostDBValue::Bytes(b) => db::DbValue::Bytes(b),
        HostDBValue::Date(s) => db::DbValue::Date(s),
        HostDBValue::Time(s) => db::DbValue::Time(s),
        HostDBValue::Timestamp(s) => db::DbValue::Timestamp(s),
        HostDBValue::TimestampTz(s) => db::DbValue::Timestamptz(s),
        HostDBValue::Uuid(s) => db::DbValue::Uuid(s),
        HostDBValue::Json(b) => db::DbValue::Json(b),
        HostDBValue::Array(arr) => {
            let wit_arr: Vec<db::ArrayElement> = arr.into_iter().map(convert_host_array_element_to_wit).collect();
            db::DbValue::Array(wit_arr)
        },
        HostDBValue::Numeric(s) => db::DbValue::Numeric(s),
        HostDBValue::Custom(s) => db::DbValue::Custom(s),
    }
}

fn convert_host_array_element_to_wit(element: HostArrayElement) -> db::ArrayElement {
    match element {
        HostArrayElement::Null => db::ArrayElement::Null,
        HostArrayElement::Boolean(b) => db::ArrayElement::Boolean(b),
        HostArrayElement::Int16(i) => db::ArrayElement::Int16(i),
        HostArrayElement::Int32(i) => db::ArrayElement::Int32(i),
        HostArrayElement::Int64(i) => db::ArrayElement::Int64(i),
        HostArrayElement::Float32(f) => db::ArrayElement::Float32(f),
        HostArrayElement::Float64(f) => db::ArrayElement::Float64(f),
        HostArrayElement::Text(s) => db::ArrayElement::Text(s),
        HostArrayElement::Bytes(b) => db::ArrayElement::Bytes(b),
        HostArrayElement::Date(s) => db::ArrayElement::Date(s),
        HostArrayElement::Time(s) => db::ArrayElement::Time(s),
        HostArrayElement::Timestamp(s) => db::ArrayElement::Timestamp(s),
        HostArrayElement::TimestampTz(s) => db::ArrayElement::Timestamptz(s),
        HostArrayElement::Uuid(s) => db::ArrayElement::Uuid(s),
        HostArrayElement::Json(b) => db::ArrayElement::Json(b),
        HostArrayElement::Numeric(s) => db::ArrayElement::Numeric(s),
        HostArrayElement::Custom(s) => db::ArrayElement::Custom(s),
    }
}

impl<T> db::Host for DBImpl<T>
where
    T: DBView,
{
   fn connect(&mut self,config:db::ConnectionConfig,) -> Result<Result<Resource<Connection>,Resource<Error>>> {
         let ctx = self.ctx();
         let config = DBConfig {
              host: config.host,
              port: config.port,
              username: config.username,
              password: config.password,
              database: config.database,
              ssl_mode: config.ssl_mode,
              connect_timeout: config.connect_timeout,
              params: config.params,
         };
         match ctx.db_backend.connect(config) {
              Ok(conn) => {
                let resource = self.table().push(conn)?;
                Ok(Ok(resource))
              }
              Err(code) => {
                let error = hayride_host_traits::db::Error {
                     code,
                     data: anyhow!("DB connection error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
              }
         }
   }

   fn connect_string(&mut self, connection_string: String,) -> Result<Result<Resource<Connection>,Resource<Error>>> {
        let ctx = self.ctx();
        match ctx.db_backend.connect_string(connection_string.into()) {
            Ok(conn) => {
                let resource = self.table().push(conn)?;
                Ok(Ok(resource))
            }
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB connection error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
   }
}

impl<T> db::HostError for DBImpl<T>
where
    T: DBView,
{
    fn code(&mut self, error: Resource<Error>) -> Result<ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            hayride_host_traits::db::ErrorCode::ConnectionFailed => Ok(ErrorCode::ConnectionFailed),
            hayride_host_traits::db::ErrorCode::QueryFailed => Ok(ErrorCode::QueryFailed),
            hayride_host_traits::db::ErrorCode::ExecuteFailed => Ok(ErrorCode::ExecuteFailed),
            hayride_host_traits::db::ErrorCode::CloseFailed => Ok(ErrorCode::CloseFailed),
            hayride_host_traits::db::ErrorCode::NotEnabled => Ok(ErrorCode::NotEnabled),
            hayride_host_traits::db::ErrorCode::Unknown => Ok(ErrorCode::Unknown),
        }
    }

    fn data(&mut self, error: Resource<Error>) -> Result<String> {
        let error = self.table().get(&error)?;
        return Ok(error.data.to_string());
    }

    fn drop(&mut self, error: Resource<Error>) -> Result<()> {
        self.table().delete(error)?;
        return Ok(());
    }
}

impl<T> db::HostConnection for DBImpl<T>
where
    T: DBView,
{
    fn query(&mut self, self_: Resource<Connection>, statement: String, params: Vec<db::DbValue>) -> wasmtime::Result<Result<db::QueryResult, Resource<Error>>> {
        let connection = self.table().get(&self_)?;
        
        // Convert WIT params to host trait params
        let host_params: Vec<HostDBValue> = params.into_iter().map(convert_db_value_to_host).collect();
        
        match connection.query(statement, host_params) {
            Ok(result) => {
                // Convert host trait result to WIT result
                let wit_result = convert_host_result_to_wit(result);
                Ok(Ok(wit_result))
            },
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB query error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn execute(&mut self, self_: Resource<Connection>, statement: String, params: Vec<db::DbValue>) -> Result<Result<u64, Resource<Error>>> {
        let connection = self.table().get(&self_)?;
        
        // Convert WIT params to host trait params
        let host_params: Vec<HostDBValue> = params.into_iter().map(convert_db_value_to_host).collect();
        
        match connection.execute(statement, host_params) {
            Ok(affected_rows) => Ok(Ok(affected_rows)),
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB execute error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn close(&mut self, self_: Resource<Connection>) -> wasmtime::Result<Result<(), Resource<Error>>> {
        let connection = self.table().get_mut(&self_)?;
        match connection.close() {
            Ok(()) => Ok(Ok(())),
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB close error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn drop(&mut self, connection: Resource<Connection>) -> Result<()> {
        self.table().delete(connection)?;
        Ok(())
    }
}