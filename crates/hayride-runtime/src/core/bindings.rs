use hayride_host_traits::core::{Ai, Config, Http, Llm, Logging, Morphs, Server, Websocket};

mod generated {
    wasmtime::component::bindgen!({
        path: "../../wit",
        world: "hayride-core",

        // Wrap functions returns with a result with error
        trappable_imports: true,
        with: {
            "hayride:core/types/error": hayride_host_traits::core::Error,
        }
    });
}

pub use self::generated::hayride::core::*;

// Convert from generated types to hayride_host_traits types
impl Into<Config> for self::generated::hayride::core::types::Config {
    fn into(self) -> Config {
        Config {
            version: self.version,
            license: self.license,
            logging: Logging {
                enabled: self.logging.enabled,
                level: self.logging.level,
                file: self.logging.file,
            },
            morphs: Morphs {
                server: Server {
                    http: Http {
                        address: self.morphs.server.http.address,
                    },
                },
                ai: Ai {
                    websocket: Websocket {
                        address: self.morphs.ai.websocket.address,
                    },
                    http: Http {
                        address: self.morphs.ai.http.address,
                    },
                    llm: Llm {
                        model: self.morphs.ai.llm.model,
                    },
                },
            },
        }
    }
}

impl Into<self::generated::hayride::core::types::Config> for Config {
    fn into(self) -> self::generated::hayride::core::config::Config {
        self::generated::hayride::core::types::Config {
            version: self.version,
            license: self.license,
            logging: self::generated::hayride::core::types::Logging {
                enabled: self.logging.enabled,
                level: self.logging.level,
                file: self.logging.file,
            },
            morphs: self::generated::hayride::core::types::Morphs {
                server: self::generated::hayride::core::types::Server {
                    http: self::generated::hayride::core::types::Http {
                        address: self.morphs.server.http.address,
                    },
                },
                ai: self::generated::hayride::core::types::Ai {
                    websocket: self::generated::hayride::core::types::Websocket {
                        address: self.morphs.ai.websocket.address,
                    },
                    http: self::generated::hayride::core::types::Http {
                        address: self.morphs.ai.http.address,
                    },
                    llm: self::generated::hayride::core::types::Llm {
                        model: self.morphs.ai.llm.model,
                    },
                },
            },
        }
    }
}
