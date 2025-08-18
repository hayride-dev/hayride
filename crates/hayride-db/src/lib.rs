use anyhow::Result;

use hayride_host_traits::db::{errors::ErrorCode, DBTrait, Connection, DBConnection};

use native_tls::TlsConnector;
use postgres_native_tls::MakeTlsConnector;
use tokio_postgres::Row;

#[derive(Clone)]
pub struct DBBackend {
    // TODO: Postgres connection
}

impl DBBackend {
    pub fn new() -> Self {
        Self {}
    }
}

impl DBTrait for DBBackend {
    fn connect(&mut self, config: hayride_host_traits::db::db::DBConfig) -> std::result::Result<Connection, ErrorCode> {
        // TODO: Use config to create connection
        Err(ErrorCode::NotEnabled)
    }

    fn connect_string(&mut self, connection_string: String) -> Result<Connection, ErrorCode> {
         tokio::task::block_in_place(|| {
            let db = tokio::runtime::Runtime::new()
                .map_err(|_| ErrorCode::ConnectionFailed)?
                .block_on(PostgresDBConnection::new(&connection_string))
                .map_err(|_| ErrorCode::ConnectionFailed)?;

            let connection: Box<dyn DBConnection> = Box::new(db);
            return Ok(connection.into());
        })
    }

}


struct PostgresDBConnection {
    client: Option<tokio_postgres::Client>,
    // conn: Option<tokio_postgres::Connection<tokio_postgres::Socket, postgres_native_tls::TlsStream<tokio_postgres::Socket>>>,
}

impl PostgresDBConnection {
    async fn new(conn_str: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tls = {
            let builder = TlsConnector::builder();
            MakeTlsConnector::new(builder.build()?)
        };

        let (client, connection) = tokio_postgres::connect(&conn_str, tls).await?;

        // Drive the connection on a background task
        // TODO: Handle closed connection
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("hayride db connection error: {e}");
            }
        });

        Ok(PostgresDBConnection {
            client: Some(client),
        })
    }
}

impl DBConnection for PostgresDBConnection {
    fn query(
            &self,
            statement: String,
            params: Vec<hayride_host_traits::db::db::DBValue>,
        ) -> std::result::Result<hayride_host_traits::db::QueryResult, ErrorCode> {
            match &self.client {
            Some(client) => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Runtime::new().unwrap().block_on(async move {
                        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
                        let row = client.query_one(&statement, &param_refs[..]).await.map_err(|_| ErrorCode::QueryFailed)?;
                        // Ok(hayride_host_traits::db::QueryResult::from_row(&row))
                        Err(ErrorCode::NotEnabled) // TODO: Implement from_row
                    })
                })
            },
            None => Err(ErrorCode::ConnectionFailed),
        }
    }

    fn execute(
            &self,
            statement: String,
            params: Vec<hayride_host_traits::db::db::DBValue>,
        ) -> std::result::Result<u64, ErrorCode> {
            match &self.client {
            Some(client) => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Runtime::new().unwrap().block_on(async move {
                        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|p| p as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
                        let result = client.execute(&statement, &param_refs[..]).await.map_err(|_| ErrorCode::QueryFailed)?;
                        Ok(result)
                    })
                })
            },
            None => Err(ErrorCode::ConnectionFailed),
        }
    }

    fn close(&mut self) -> std::result::Result<(), ErrorCode> {
        // TODO: Properly close the connection
        self.client = None;
        Ok(())
    }
}