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
pub use self::generated::hayride::core::types as api;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Request {
    pub data: RequestData,
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
    pub data: ResponseData,
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
pub enum RequestData {
    Unknown,
    Cast(api::Cast),
    SessionId(String),
    Generate(api::Generate),
}

impl From<api::RequestData> for RequestData {
    fn from(d: api::RequestData) -> Self {
        match d {
            api::RequestData::Unknown => RequestData::Unknown,
            api::RequestData::Cast(c) => RequestData::Cast(c.into()),
            api::RequestData::SessionId(id) => RequestData::SessionId(id),
            api::RequestData::Generate(g) => RequestData::Generate(g.into()),
        }
    }
}

impl From<RequestData> for api::RequestData {
    fn from(dc: RequestData) -> Self {
        match dc {
            RequestData::Unknown => api::RequestData::Unknown,
            RequestData::Cast(c) => api::RequestData::Cast(c.into()),
            RequestData::SessionId(id) => api::RequestData::SessionId(id),
            RequestData::Generate(g) => api::RequestData::Generate(g.into()),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ResponseData {
    Unknown,
    Sessions(Vec<api::ThreadMetadata>),
    SessionId(String),
    SessionStatus(api::ThreadStatus),
    Messages(Vec<Message>),
    Path(String),
    Paths(Vec<String>),
    Version(String),
}

impl From<api::ResponseData> for ResponseData {
    fn from(d: api::ResponseData) -> Self {
        match d {
            api::ResponseData::Unknown => ResponseData::Unknown,
            api::ResponseData::Sessions(sessions) => {
                ResponseData::Sessions(sessions.into_iter().map(|s| s.into()).collect())
            }
            api::ResponseData::SessionId(id) => ResponseData::SessionId(id),
            api::ResponseData::SessionStatus(status) => ResponseData::SessionStatus(status.into()),
            api::ResponseData::Messages(msgs) => {
                ResponseData::Messages(msgs.into_iter().map(|m| m.into()).collect())
            }
            api::ResponseData::Path(path) => ResponseData::Path(path),
            api::ResponseData::Paths(paths) => ResponseData::Paths(paths),
            api::ResponseData::Version(v) => ResponseData::Version(v),
        }
    }
}

impl From<ResponseData> for api::ResponseData {
    fn from(dc: ResponseData) -> Self {
        match dc {
            ResponseData::Unknown => api::ResponseData::Unknown,
            ResponseData::Sessions(sessions) => {
                api::ResponseData::Sessions(sessions.into_iter().map(|s| s.into()).collect())
            }
            ResponseData::SessionId(id) => api::ResponseData::SessionId(id),
            ResponseData::SessionStatus(status) => api::ResponseData::SessionStatus(status.into()),
            ResponseData::Messages(msgs) => {
                api::ResponseData::Messages(msgs.into_iter().map(|m| m.into()).collect())
            }
            ResponseData::Path(path) => api::ResponseData::Path(path),
            ResponseData::Paths(paths) => api::ResponseData::Paths(paths),
            ResponseData::Version(v) => api::ResponseData::Version(v),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Message {
    pub role: Role,
    pub content: Vec<MessageContent>,
    pub final_: bool,
}

impl From<api::Message> for Message {
    fn from(m: types::Message) -> Self {
        Self {
            role: m.role.into(),
            content: m.content.into_iter().map(|c| c.into()).collect(),
            final_: m.final_,
        }
    }
}

impl From<Message> for types::Message {
    fn from(mc: Message) -> Self {
        Self {
            role: mc.role.into(),
            content: mc.content.into_iter().map(|c| c.into()).collect(),
            final_: mc.final_,
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
pub enum MessageContent {
    None,
    Text(String),
    Blob(Vec<u8>),
    Tools(Vec<types::Tool>),
    ToolInput(types::CallToolParams),
    ToolOutput(types::CallToolResult),
}

impl From<types::MessageContent> for MessageContent {
    fn from(c: types::MessageContent) -> Self {
        match c {
            types::MessageContent::None => MessageContent::None,
            types::MessageContent::Text(t) => MessageContent::Text(t.into()),
            types::MessageContent::Blob(b) => MessageContent::Blob(b),
            types::MessageContent::Tools(ts) => MessageContent::Tools(ts.into()),
            types::MessageContent::ToolInput(ti) => MessageContent::ToolInput(ti.into()),
            types::MessageContent::ToolOutput(to) => MessageContent::ToolOutput(to.into()),
        }
    }
}

impl From<MessageContent> for types::MessageContent {
    fn from(c: MessageContent) -> Self {
        match c {
            MessageContent::None => types::MessageContent::None,
            MessageContent::Text(t) => types::MessageContent::Text(t.into()),
            MessageContent::Blob(b) => types::MessageContent::Blob(b),
            MessageContent::Tools(ts) => types::MessageContent::Tools(ts.into()),
            MessageContent::ToolInput(ti) => types::MessageContent::ToolInput(ti.into()),
            MessageContent::ToolOutput(to) => types::MessageContent::ToolOutput(to.into()),
        }
    }
}
