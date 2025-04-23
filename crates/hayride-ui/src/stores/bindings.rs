mod generated {
    wit_bindgen::generate!({
        generate_all,
        generate_unused_types: true,
        path: "../../wit",
        world: "hayride-api",

        additional_derives: [serde::Serialize, serde::Deserialize],
    });
}

pub use self::generated::hayride::core::api::*;
pub use self::generated::hayride::ai::types::{Role, Content, TextContent, ToolInput, ToolOutput, ToolSchema};
