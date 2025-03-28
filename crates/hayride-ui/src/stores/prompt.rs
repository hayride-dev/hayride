use reactive_stores::Store;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default, Store)]
pub struct Prompt {
    pub message: String,
    pub system: String,
    pub agent: String,
    pub options: PromptOptions,
}

// Global State for prompt options
#[derive(Clone, Serialize, Deserialize, Default, Store)]
pub struct PromptOptions {
    pub temperature: f32,
    pub num_context: i32,
    pub num_batch: i32,
    pub max_predict: i32,
    pub top_k: i32,
    pub top_p: f32,
    pub seed: u32,
}
