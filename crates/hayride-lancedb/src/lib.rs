use hayride_host_traits::ai::rag::{
    Connection, Embedding, ErrorCode, RagConnection, RagInner, RagOption, Transformer,
};

use std::{iter::once, sync::Arc};

use arrow_array::{RecordBatch, RecordBatchIterator, StringArray};
use arrow_schema::{ArrowError, DataType, Field, Schema};
use tokio::task;

use futures::StreamExt;
use lancedb::embeddings::{EmbeddingDefinition, EmbeddingFunction};
use lancedb::{
    arrow::IntoArrow,
    connect,
    connection::ConnectBuilder,
    embeddings::sentence_transformers::SentenceTransformersEmbeddings,
    query::{ExecutableQuery, QueryBase},
};

#[derive(Default)]
pub struct LanceDBRag {}

impl RagInner for LanceDBRag {
    fn connect(&mut self, dsn: String) -> Result<Connection, ErrorCode> {
        let builder: ConnectBuilder = connect(&dsn);

        tokio::task::block_in_place(|| {
            let db = tokio::runtime::Runtime::new()
                .map_err(|_| ErrorCode::ConnectionFailed)?
                .block_on(LanceDBConnection::new(builder))
                .map_err(|_| ErrorCode::ConnectionFailed)?;

            let connection: Box<dyn RagConnection> = Box::new(db);
            return Ok(connection.into());
        })
    }
}

struct LanceDBConnection {
    conn: Option<lancedb::Connection>,
    embedding: Option<Arc<SentenceTransformersEmbeddings>>,
    transformer: Option<Transformer>,
}

impl LanceDBConnection {
    async fn new(builder: ConnectBuilder) -> Result<Self, Box<dyn std::error::Error>> {
        let conn: lancedb::Connection =
            task::spawn(async move { builder.execute().await }).await??;

        Ok(LanceDBConnection {
            conn: Some(conn),
            embedding: None,
            transformer: None,
        })
    }
}

impl RagConnection for LanceDBConnection {
    fn register(&mut self, transformer: Transformer) -> Result<(), ErrorCode> {
        log::debug!("registering transformer: {:?}", transformer);
        match &self.conn {
            Some(conn) => match transformer.embedding {
                Embedding::Sentence => {
                    let embedding = SentenceTransformersEmbeddings::builder()
                        .model(transformer.model.clone())
                        .build()
                        .map_err(|_| ErrorCode::RegisterFailed)?;
                    let embedding = Arc::new(embedding);
                    self.embedding = Some(embedding.clone());
                    self.transformer = Some(transformer.clone());
                    conn.embedding_registry()
                        .register(&transformer.embedding.to_string(), embedding.clone())
                        .map_err(|_| ErrorCode::RegisterFailed)?;
                }
            },
            None => {
                return Err(ErrorCode::ConnectionFailed);
            }
        }

        return Ok(());
    }

    fn embed(&self, table: String, data: String) -> Result<(), ErrorCode> {
        log::debug!("embedding data into table: {}, data: {}", table, data);

        let transformer = self.transformer.as_ref().ok_or(ErrorCode::RegisterFailed)?;

        match &self.conn {
            Some(conn) => {
                tokio::task::block_in_place(|| {
                    tokio::runtime::Runtime::new()
                        .map_err(|_| ErrorCode::EmbedFailed)?
                        .block_on(async {
                            match conn.open_table(table.clone()).execute().await {
                                Ok(table) => {
                                    log::debug!("table exists, embedding data: {}", table);

                                    match table
                                        .add(
                                            make_data(&transformer.data_column, data)
                                                .map_err(|_| ErrorCode::EmbedFailed)?,
                                        )
                                        .execute()
                                        .await
                                    {
                                        Ok(_) => {}
                                        Err(e) => {
                                            log::warn!("failed to embed data into table: {}", e);
                                            return Err(ErrorCode::EmbedFailed);
                                        }
                                    }

                                    Ok(())
                                }
                                Err(_) => {
                                    log::debug!("table does not exist, creating table: {}", table);

                                    // Try to create the table and store the data
                                    conn.create_table(
                                        table.clone(),
                                        make_data(&transformer.data_column, data)
                                            .map_err(|_| ErrorCode::EmbedFailed)?,
                                    )
                                    .add_embedding(EmbeddingDefinition::new(
                                        transformer.data_column.clone(),
                                        transformer.embedding.to_string(),
                                        Some(transformer.vector_column.clone()),
                                    ))
                                    .map_err(|_| ErrorCode::CreateTableFailed)?
                                    .execute()
                                    .await
                                    .map_err(|_| ErrorCode::CreateTableFailed)?;

                                    log::debug!("table created: {}", table);

                                    Ok(())
                                }
                            }
                        })
                })?
            }
            None => {
                return Err(ErrorCode::ConnectionFailed);
            }
        }

        return Ok(());
    }

    fn query(
        &self,
        table: String,
        data: String,
        options: Vec<RagOption>,
    ) -> Result<Vec<String>, ErrorCode> {
        log::debug!("querying table: {}, data: {}", table, data);

        // Set default options and parse rag options for overrides
        let mut limit = 1;

        options.iter().for_each(|option| {
            // Match on lowercase option name
            match option.name.to_lowercase().as_str() {
                "limit" => {
                    // Parse the limit value
                    match option.value.parse::<usize>() {
                        Ok(value) => {
                            // Set the limit to the parsed value
                            limit = value;
                        }
                        Err(_) => {
                            // Invalid limit value
                            log::warn!("invalid limit value: {}", option.value);
                        }
                    }
                }
                _ => {
                    // Invalid option
                    log::warn!("unexpected option: {}", option.name);
                }
            }
        });

        match &self.conn {
            Some(conn) => {
                let result = tokio::task::block_in_place(|| {
                    tokio::runtime::Runtime::new()
                        .map_err(|_| ErrorCode::QueryFailed)?
                        .block_on(async {
                            let table = conn
                                .open_table(table.clone())
                                .execute()
                                .await
                                .map_err(|_| ErrorCode::MissingTable)?;

                            // Compute the query vector
                            let query = Arc::new(StringArray::from_iter_values(once(data)));

                            let embedding =
                                self.embedding.as_ref().ok_or(ErrorCode::MissingTable)?;
                            let query_vector = embedding
                                .compute_query_embeddings(query)
                                .map_err(|_| ErrorCode::EmbedFailed)?;
                            let mut results = table
                                .vector_search(query_vector)
                                .map_err(|_| ErrorCode::QueryFailed)?
                                .limit(limit)
                                .execute()
                                .await
                                .map_err(|_| ErrorCode::QueryFailed)?;

                            let rb = results
                                .next()
                                .await
                                .ok_or(ErrorCode::QueryFailed)?
                                .map_err(|_| ErrorCode::QueryFailed)?;
                            let out = rb
                                .column_by_name("text")
                                .ok_or(ErrorCode::QueryFailed)?
                                .as_any()
                                .downcast_ref::<StringArray>()
                                .ok_or(ErrorCode::QueryFailed)?;

                            // Return results filtering out nulls
                            let results: Vec<String> = out
                                .iter()
                                .filter_map(|x| x.map(|s| s.to_string()))
                                .collect();
                            Ok(results)
                        })
                })?;

                return Ok(result);
            }
            None => {
                log::warn!("failed to connect to LanceDB");

                return Err(ErrorCode::ConnectionFailed);
            }
        }
    }
}

fn make_data(data_column: &str, data: String) -> Result<impl IntoArrow, ArrowError> {
    let schema = Schema::new(vec![Field::new(data_column, DataType::Utf8, false)]);
    let schema = Arc::new(schema);
    let source = StringArray::from_iter_values(vec![data]);

    let rb = RecordBatch::try_new(schema.clone(), vec![Arc::new(source)])?;
    Ok(Box::new(RecordBatchIterator::new(vec![Ok(rb)], schema)))
}
