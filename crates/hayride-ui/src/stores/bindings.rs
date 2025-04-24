use serde::{Deserialize,Serialize};

mod generated {
    wit_bindgen::generate!({
        generate_all,
        generate_unused_types: true,
        path: "../../wit",
        world: "hayride-api",

        additional_derives: [serde::Serialize, serde::Deserialize],
    });
}

pub use self::generated::hayride::core::api;
pub use self::generated::hayride::ai::types::{Role, Content, TextContent, ToolInput, ToolOutput, ToolSchema};


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
    Messages(Vec<api::Message>),
}


impl From<api::Data> for Data {
    fn from(d: api::Data) -> Self {
        match d {
            api::Data::Messages(msgs) => Data::Messages(msgs),
        }
    }
}

impl From<Data> for api::Data {
    fn from(dc: Data) -> Self {
        match dc {
            Data::Messages(msgs) => api::Data::Messages(msgs),
        }
    }
}