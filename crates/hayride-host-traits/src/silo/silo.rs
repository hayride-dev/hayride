#[derive(Debug, Clone, PartialEq)]
pub enum ThreadStatus {
    Unknown,
    Processing,
    Exited,
    Killed,
}

/// A host-side thread.
#[derive(Clone, PartialEq)]
pub struct Thread {
    pub id: String,
    pub pkg: String,
    pub function: String,
    pub args: Vec<String>,
    pub status: ThreadStatus,
}