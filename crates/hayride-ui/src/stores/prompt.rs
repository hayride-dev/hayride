use reactive_stores::Store;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Store)]
#[repr(u8)]
pub enum Role {
    #[default]
    User = 0,
    Assistant = 1,
    System = 2,
    Tool = 3,
    Unknown = 4,
}

// Serialize/Deserialize manually as u8
impl Serialize for Role {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u8(*self as u8)
    }
}

impl<'de> Deserialize<'de> for Role {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u8::deserialize(deserializer)?;
        Ok(match value {
            0 => Role::User,
            1 => Role::Assistant,
            2 => Role::System,
            3 => Role::Tool,
            4 => Role::Unknown,
            _ => Role::Unknown,
        })
    }
}

#[derive(Clone, Serialize, Deserialize, Default, Store)]
pub struct Prompt {
    pub messages: Vec<Message>,
    // pub system: String,
    pub agent: String,
    pub options: PromptOptions,
}

#[derive(Clone, Serialize, Deserialize, Default, Store)]
pub struct Message {
    pub role: Role,
    pub content: Vec<String>,
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
