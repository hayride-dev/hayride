use super::errors::ErrorCode;
use std::fmt;

pub trait RagInner: Send + Sync {
    fn connect(&mut self, dsn: String) -> Result<Connection, ErrorCode>;
}

pub trait RagConnection: Send + Sync {
    fn register(&mut self, transformer: Transformer) -> Result<(), ErrorCode>;
    fn embed(&self, table: String, data: String) -> Result<(), ErrorCode>;
    fn query(
        &self,
        table: String,
        data: String,
        options: Vec<RagOption>,
    ) -> Result<Vec<String>, ErrorCode>;
}

/// A backend-defined Rag Connection
pub struct Connection(Box<dyn RagConnection>);
impl From<Box<dyn RagConnection>> for Connection {
    fn from(value: Box<dyn RagConnection>) -> Self {
        Self(value)
    }
}
impl std::ops::Deref for Connection {
    type Target = dyn RagConnection;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

/// A host-side transformer.
#[derive(Debug, Clone, PartialEq)]
pub struct Transformer {
    pub embedding: Embedding,
    pub model: String,
    pub data_column: String,
    pub vector_column: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Embedding {
    Sentence,
}

impl fmt::Display for Embedding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Embedding::Sentence => write!(f, "sentence-transformers"),
        }
    }
}

/// A Rag option.
#[derive(Debug, Clone, PartialEq)]
pub struct RagOption {
    pub name: String,
    pub value: String,
}
