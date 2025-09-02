use crate::db::bindings::db::Statement;
use crate::db::bindings::{db, db::ErrorCode};
use crate::db::{DBImpl, DBView};
use hayride_host_traits::db::db::{DBValue as HostDBValue, Statement as HostStatement};
use hayride_host_traits::db::{Connection, Error, IsolationLevel, Rows};

use wasmtime::component::Resource;
use wasmtime::Result;

use anyhow::anyhow;

// Conversion functions between WIT types and host trait types
fn convert_db_value_to_host(value: db::DbValue) -> HostDBValue {
    match value {
        db::DbValue::Null => HostDBValue::Null,
        db::DbValue::Boolean(b) => HostDBValue::Boolean(b),
        db::DbValue::Int32(i) => HostDBValue::Int32(i),
        db::DbValue::Int64(i) => HostDBValue::Int64(i),
        db::DbValue::Uint32(u) => HostDBValue::Uint32(u),
        db::DbValue::Uint64(u) => HostDBValue::Uint64(u),
        db::DbValue::Float(f) => HostDBValue::Float(f),
        db::DbValue::Double(f) => HostDBValue::Double(f),
        db::DbValue::Str(s) => HostDBValue::Str(s),
        db::DbValue::Binary(b) => HostDBValue::Binary(b),
        db::DbValue::Date(s) => HostDBValue::Date(s),
        db::DbValue::Time(s) => HostDBValue::Time(s),
        db::DbValue::Timestamp(s) => HostDBValue::Timestamp(s),
    }
}

fn convert_host_db_value_to_wit(value: HostDBValue) -> db::DbValue {
    match value {
        HostDBValue::Null => db::DbValue::Null,
        HostDBValue::Boolean(b) => db::DbValue::Boolean(b),
        HostDBValue::Int32(i) => db::DbValue::Int32(i),
        HostDBValue::Int64(i) => db::DbValue::Int64(i),
        HostDBValue::Uint32(u) => db::DbValue::Uint32(u),
        HostDBValue::Uint64(u) => db::DbValue::Uint64(u),
        HostDBValue::Float(f) => db::DbValue::Float(f),
        HostDBValue::Double(f) => db::DbValue::Double(f),
        HostDBValue::Str(s) => db::DbValue::Str(s),
        HostDBValue::Binary(b) => db::DbValue::Binary(b),
        HostDBValue::Date(s) => db::DbValue::Date(s),
        HostDBValue::Time(s) => db::DbValue::Time(s),
        HostDBValue::Timestamp(s) => db::DbValue::Timestamp(s),
    }
}

impl<T> db::Host for DBImpl<T>
where
    T: DBView,
{
    fn open(&mut self, name: String) -> Result<Result<Resource<Connection>, Resource<Error>>> {
        let ctx = self.ctx();
        match ctx.db_backend.open(name.into()) {
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
            hayride_host_traits::db::ErrorCode::OpenFailed => Ok(ErrorCode::OpenFailed),
            hayride_host_traits::db::ErrorCode::QueryFailed => Ok(ErrorCode::QueryFailed),
            hayride_host_traits::db::ErrorCode::ExecuteFailed => Ok(ErrorCode::ExecuteFailed),
            hayride_host_traits::db::ErrorCode::PrepareFailed => Ok(ErrorCode::PrepareFailed),
            hayride_host_traits::db::ErrorCode::CloseFailed => Ok(ErrorCode::CloseFailed),
            hayride_host_traits::db::ErrorCode::NumberParametersFailed => {
                Ok(ErrorCode::NumberParametersFailed)
            }
            hayride_host_traits::db::ErrorCode::BeginTransactionFailed => {
                Ok(ErrorCode::BeginTransactionFailed)
            }
            hayride_host_traits::db::ErrorCode::CommitFailed => Ok(ErrorCode::CommitFailed),
            hayride_host_traits::db::ErrorCode::RollbackFailed => Ok(ErrorCode::RollbackFailed),
            hayride_host_traits::db::ErrorCode::NextFailed => Ok(ErrorCode::NextFailed),
            hayride_host_traits::db::ErrorCode::EndOfRows => Ok(ErrorCode::EndOfRows),
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
    fn prepare(
        &mut self,
        self_: Resource<Connection>,
        query: String,
    ) -> wasmtime::Result<Result<Resource<Statement>, Resource<Error>>> {
        let connection: &Connection = self.table().get(&self_)?;
        match connection.prepare(query) {
            Ok(statement) => {
                let resource = self.table().push(statement)?;
                Ok(Ok(resource))
            }
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB prepare error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn begin_transaction(
        &mut self,
        self_: wasmtime::component::Resource<Connection>,
        isolation_level: db::IsolationLevel,
        read_only: bool,
    ) -> wasmtime::Result<
        std::result::Result<
            wasmtime::component::Resource<hayride_host_traits::db::Transaction>,
            wasmtime::component::Resource<Error>,
        >,
    > {
        let connection: &mut Connection = self.table().get_mut(&self_)?;

        let isolation_level = match isolation_level {
            db::IsolationLevel::ReadUncommitted => IsolationLevel::ReadUncommitted,
            db::IsolationLevel::ReadCommitted => IsolationLevel::ReadCommitted,
            db::IsolationLevel::WriteCommitted => IsolationLevel::WriteCommitted,
            db::IsolationLevel::RepeatableRead => IsolationLevel::RepeatableRead,
            db::IsolationLevel::Snapshot => IsolationLevel::Snapshot,
            db::IsolationLevel::Serializable => IsolationLevel::Serializable,
            db::IsolationLevel::Linearizable => IsolationLevel::Linearizable,
        };

        match connection.begin_transaction(isolation_level, read_only) {
            Ok(transaction) => {
                let resource = self.table().push(transaction)?;
                Ok(Ok(resource))
            }
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB begin transaction error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn close(
        &mut self,
        self_: Resource<Connection>,
    ) -> wasmtime::Result<Result<(), Resource<Error>>> {
        let connection: &mut Connection = self.table().get_mut(&self_)?;
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

impl<T> db::HostStatement for DBImpl<T>
where
    T: DBView,
{
    fn query(
        &mut self,
        statement: wasmtime::component::Resource<HostStatement>,
        args: wasmtime::component::__internal::Vec<db::DbValue>,
    ) -> wasmtime::Result<
        std::result::Result<
            wasmtime::component::Resource<Rows>,
            wasmtime::component::Resource<Error>,
        >,
    > {
        let statement: &HostStatement = self.table().get(&statement)?;

        // Convert WIT params to host trait params
        let host_params: Vec<HostDBValue> =
            args.into_iter().map(convert_db_value_to_host).collect();

        match statement.query(host_params) {
            Ok(result) => {
                let resource = self.table().push(result)?;
                Ok(Ok(resource))
            }
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

    fn number_parameters(
        &mut self,
        self_: wasmtime::component::Resource<HostStatement>,
    ) -> wasmtime::Result<u32> {
        let statement: &HostStatement = self.table().get(&self_)?;
        match statement.number_parameters() {
            Ok(num) => Ok(num),
            Err(code) => {
                log::error!("DB number_parameters error: {:?}", code);
                Ok(0)
            }
        }
    }

    fn execute(
        &mut self,
        statement: Resource<Statement>,
        params: Vec<db::DbValue>,
    ) -> Result<Result<u64, Resource<Error>>> {
        let statement: &HostStatement = self.table().get(&statement)?;

        // Convert WIT params to host trait params
        let host_params: Vec<HostDBValue> =
            params.into_iter().map(convert_db_value_to_host).collect();

        match statement.execute(host_params) {
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

    fn close(
        &mut self,
        statement: Resource<Statement>,
    ) -> wasmtime::Result<Result<(), Resource<Error>>> {
        let statement: &mut HostStatement = self.table().get_mut(&statement)?;
        match statement.close() {
            Ok(()) => Ok(Ok(())),
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB statement close error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn drop(&mut self, statement: Resource<Statement>) -> Result<()> {
        self.table().delete(statement)?;
        Ok(())
    }
}

impl<T> db::HostTransaction for DBImpl<T>
where
    T: DBView,
{
    fn commit(
        &mut self,
        self_: wasmtime::component::Resource<hayride_host_traits::db::Transaction>,
    ) -> wasmtime::Result<std::result::Result<(), wasmtime::component::Resource<Error>>> {
        let transaction: &mut hayride_host_traits::db::Transaction =
            self.table().get_mut(&self_)?;
        match transaction.commit() {
            Ok(()) => Ok(Ok(())),
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB transaction commit error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn rollback(
        &mut self,
        self_: wasmtime::component::Resource<hayride_host_traits::db::Transaction>,
    ) -> wasmtime::Result<std::result::Result<(), wasmtime::component::Resource<Error>>> {
        let transaction: &mut hayride_host_traits::db::Transaction =
            self.table().get_mut(&self_)?;
        match transaction.rollback() {
            Ok(()) => Ok(Ok(())),
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB transaction rollback error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn execute(
        &mut self,
        self_: wasmtime::component::Resource<hayride_host_traits::db::Transaction>,
        query: wasmtime::component::__internal::String,
        args: wasmtime::component::__internal::Vec<db::DbValue>,
    ) -> wasmtime::Result<std::result::Result<u64, wasmtime::component::Resource<Error>>> {
        let transaction: &hayride_host_traits::db::Transaction = self.table().get(&self_)?;

        // Convert WIT params to host trait params
        let host_params: Vec<HostDBValue> =
            args.into_iter().map(convert_db_value_to_host).collect();

        match transaction.execute(query, host_params) {
            Ok(affected_rows) => Ok(Ok(affected_rows)),
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB transaction execute error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn query(
        &mut self,
        self_: wasmtime::component::Resource<hayride_host_traits::db::Transaction>,
        query: wasmtime::component::__internal::String,
        args: wasmtime::component::__internal::Vec<db::DbValue>,
    ) -> wasmtime::Result<
        std::result::Result<
            wasmtime::component::Resource<Rows>,
            wasmtime::component::Resource<Error>,
        >,
    > {
        let transaction: &hayride_host_traits::db::Transaction = self.table().get(&self_)?;
        // Convert WIT params to host trait params
        let host_params: Vec<HostDBValue> =
            args.into_iter().map(convert_db_value_to_host).collect();

        match transaction.query(query, host_params) {
            Ok(rows) => {
                let resource = self.table().push(rows)?;
                Ok(Ok(resource))
            }
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB transaction query error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn prepare(
        &mut self,
        self_: wasmtime::component::Resource<hayride_host_traits::db::Transaction>,
        query: wasmtime::component::__internal::String,
    ) -> wasmtime::Result<
        std::result::Result<
            wasmtime::component::Resource<HostStatement>,
            wasmtime::component::Resource<Error>,
        >,
    > {
        let transaction: &hayride_host_traits::db::Transaction = self.table().get(&self_)?;
        match transaction.prepare(query) {
            Ok(statement) => {
                let resource = self.table().push(statement)?;
                Ok(Ok(resource))
            }
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB transaction prepare error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn drop(
        &mut self,
        rep: wasmtime::component::Resource<hayride_host_traits::db::Transaction>,
    ) -> wasmtime::Result<()> {
        self.table().delete(rep)?;
        Ok(())
    }
}

impl<T> db::HostRows for DBImpl<T>
where
    T: DBView,
{
    fn columns(
        &mut self,
        self_: wasmtime::component::Resource<Rows>,
    ) -> wasmtime::Result<
        wasmtime::component::__internal::Vec<wasmtime::component::__internal::String>,
    > {
        let rows: &Rows = self.table().get(&self_)?;
        let columns = rows.columns();
        Ok(columns)
    }

    fn next(
        &mut self,
        self_: wasmtime::component::Resource<Rows>,
    ) -> wasmtime::Result<std::result::Result<db::Row, wasmtime::component::Resource<Error>>> {
        let rows: &mut Rows = self.table().get_mut(&self_)?;
        match rows.next() {
            Ok(row) => {
                let wit_row: db::Row = row
                    .0
                    .into_iter()
                    .map(convert_host_db_value_to_wit)
                    .collect();
                Ok(Ok(wit_row))
            }
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB rows next error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn close(
        &mut self,
        self_: wasmtime::component::Resource<Rows>,
    ) -> wasmtime::Result<std::result::Result<(), wasmtime::component::Resource<Error>>> {
        let rows = self.table().get_mut(&self_)?;
        match rows.close() {
            Ok(()) => Ok(Ok(())),
            Err(code) => {
                let error = Error {
                    code,
                    data: anyhow!("DB rows close error"),
                };
                let resource = self.table().push(error)?;
                Ok(Err(resource))
            }
        }
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Rows>) -> wasmtime::Result<()> {
        self.table().delete(rep)?;
        Ok(())
    }
}
