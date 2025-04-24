use serde::{Deserialize, Serialize};

mod generated {
    wit_bindgen::generate!({
        generate_all,
        generate_unused_types: true,
        path: "../../wit",
        world: "hayride-api",

        additional_derives: [serde::Serialize, serde::Deserialize],
    });
}

pub use self::generated::hayride::ai::types;
pub use self::generated::hayride::core::api;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Request {
    pub data: Data,
    pub metadata: Vec<(String, String)>,
}

impl From<api::Request> for Request {
    fn from(r: api::Request) -> Self {
        Self {
            data: r.data.into(),
            metadata: r.metadata,
        }
    }
}

impl From<Request> for api::Request {
    fn from(rc: Request) -> Self {
        Self {
            data: rc.data.into(),
            metadata: rc.metadata,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Response {
    pub data: Data,
    pub error: String,
    pub next: String,
    pub prev: String,
}

impl From<api::Response> for Response {
    fn from(r: api::Response) -> Self {
        Self {
            data: r.data.into(),
            error: r.error,
            next: r.next,
            prev: r.prev,
        }
    }
}

impl From<Response> for api::Response {
    fn from(rc: Response) -> Self {
        Self {
            data: rc.data.into(),
            error: rc.error,
            next: rc.next,
            prev: rc.prev,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Data {
    Messages(Vec<Message>),
}

impl From<api::Data> for Data {
    fn from(d: api::Data) -> Self {
        match d {
            api::Data::Messages(msgs) => {
                Data::Messages(msgs.into_iter().map(|m| m.into()).collect())
            }
        }
    }
}

impl From<Data> for api::Data {
    fn from(dc: Data) -> Self {
        match dc {
            Data::Messages(msgs) => {
                api::Data::Messages(msgs.into_iter().map(|m| m.into()).collect())
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Message {
    pub role: Role,
    pub content: Vec<Content>,
}

impl From<api::Message> for Message {
    fn from(m: types::Message) -> Self {
        Self {
            role: m.role.into(),
            content: m.content.into_iter().map(|c| c.into()).collect(),
        }
    }
}

impl From<Message> for types::Message {
    fn from(mc: Message) -> Self {
        Self {
            role: mc.role.into(),
            content: mc.content.into_iter().map(|c| c.into()).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
    Unknown,
}

impl From<types::Role> for Role {
    fn from(role: types::Role) -> Self {
        match role {
            types::Role::User => Self::User,
            types::Role::Assistant => Self::Assistant,
            types::Role::System => Self::System,
            types::Role::Tool => Self::Tool,
            types::Role::Unknown => Self::Unknown,
        }
    }
}

impl From<Role> for types::Role {
    fn from(rc: Role) -> Self {
        match rc {
            Role::User => Self::User,
            Role::Assistant => Self::Assistant,
            Role::System => Self::System,
            Role::Tool => Self::Tool,
            Role::Unknown => Self::Unknown,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Content {
    None,
    Text(TextContent),
    ToolInput(ToolInput),
    ToolOutput(ToolOutput),
    ToolSchema(ToolSchema),
}

impl From<types::Content> for Content {
    fn from(c: types::Content) -> Self {
        match c {
            types::Content::None => Content::None,
            types::Content::Text(t) => Content::Text(t.into()),
            types::Content::ToolInput(ti) => Content::ToolInput(ti.into()),
            types::Content::ToolOutput(to) => Content::ToolOutput(to.into()),
            types::Content::ToolSchema(ts) => Content::ToolSchema(ts.into()),
        }
    }
}

impl From<Content> for types::Content {
    fn from(c: Content) -> Self {
        match c {
            Content::None => types::Content::None,
            Content::Text(t) => types::Content::Text(t.into()),
            Content::ToolInput(ti) => types::Content::ToolInput(ti.into()),
            Content::ToolOutput(to) => types::Content::ToolOutput(to.into()),
            Content::ToolSchema(ts) => types::Content::ToolSchema(ts.into()),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TextContent {
    pub text: String,
    pub content_type: String,
}

impl From<types::TextContent> for TextContent {
    fn from(tc: types::TextContent) -> Self {
        Self {
            text: tc.text,
            content_type: tc.content_type,
        }
    }
}

impl From<TextContent> for types::TextContent {
    fn from(tc: TextContent) -> Self {
        Self {
            text: tc.text,
            content_type: tc.content_type,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ToolInput {
    pub content_type: String,
    pub id: String,
    pub name: String,
    pub input: String,
}

impl From<types::ToolInput> for ToolInput {
    fn from(ti: types::ToolInput) -> Self {
        Self {
            content_type: ti.content_type,
            id: ti.id,
            name: ti.name,
            input: ti.input,
        }
    }
}

impl From<ToolInput> for types::ToolInput {
    fn from(ti: ToolInput) -> Self {
        Self {
            content_type: ti.content_type,
            id: ti.id,
            name: ti.name,
            input: ti.input,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ToolOutput {
    pub content_type: String,
    pub id: String,
    pub name: String,
    pub output: String,
}

impl From<types::ToolOutput> for ToolOutput {
    fn from(to: types::ToolOutput) -> Self {
        Self {
            content_type: to.content_type,
            id: to.id,
            name: to.name,
            output: to.output,
        }
    }
}

impl From<ToolOutput> for types::ToolOutput {
    fn from(to: ToolOutput) -> Self {
        Self {
            content_type: to.content_type,
            id: to.id,
            name: to.name,
            output: to.output,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ToolSchema {
    pub id: String,
    pub name: String,
    pub description: String,
    pub params_schema: String,
}

impl From<types::ToolSchema> for ToolSchema {
    fn from(ts: types::ToolSchema) -> Self {
        Self {
            id: ts.id,
            name: ts.name,
            description: ts.description,
            params_schema: ts.params_schema,
        }
    }
}

impl From<ToolSchema> for types::ToolSchema {
    fn from(ts: ToolSchema) -> Self {
        Self {
            id: ts.id,
            name: ts.name,
            description: ts.description,
            params_schema: ts.params_schema,
        }
    }
}
