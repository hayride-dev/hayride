use hayride_host_traits::core::{
    Ai, Cli, Config, Feature, Http, Logging, Morph, Server, Ui, Websocket,
};

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
impl From<self::generated::hayride::core::types::Config> for Config {
    fn from(value: self::generated::hayride::core::types::Config) -> Self {
        let core = value
            .core
            .into_iter()
            .map(|m| match m {
                self::generated::hayride::core::types::Morph::Cli(cli) => Morph::Cli(Cli {
                    bin: cli.bin,
                    logging: Logging {
                        enabled: cli.logging.enabled,
                        level: cli.logging.level,
                        file: cli.logging.file,
                    },
                }),
                self::generated::hayride::core::types::Morph::Server(server) => {
                    Morph::Server(Server {
                        bin: server.bin,
                        logging: Logging {
                            enabled: server.logging.enabled,
                            level: server.logging.level,
                            file: server.logging.file,
                        },
                        http: Http {
                            address: server.http.address,
                        },
                    })
                }
            })
            .collect();

        let features = value
            .features
            .into_iter()
            .map(|f| match f {
                self::generated::hayride::core::types::Feature::Ai(ai) => Feature::Ai(Ai {
                    bin: ai.bin,
                    logging: Logging {
                        enabled: ai.logging.enabled,
                        level: ai.logging.level,
                        file: ai.logging.file,
                    },
                    http: Http {
                        address: ai.http.address,
                    },
                    websocket: Websocket {
                        address: ai.websocket.address,
                    },
                }),
                self::generated::hayride::core::types::Feature::Ui(ui) => Feature::Ui(Ui {
                    bin: ui.bin,
                    logging: Logging {
                        enabled: ui.logging.enabled,
                        level: ui.logging.level,
                        file: ui.logging.file,
                    },
                    http: Http {
                        address: ui.http.address,
                    },
                }),
            })
            .collect();

        Config {
            version: value.version,
            license: value.license,
            core,
            features,
        }
    }
}

impl From<Config> for self::generated::hayride::core::types::Config {
    fn from(value: Config) -> Self {
        let core = value
            .core
            .into_iter()
            .map(|m| match m {
                Morph::Cli(cli) => self::generated::hayride::core::types::Morph::Cli(
                    self::generated::hayride::core::types::Cli {
                        bin: cli.bin,
                        logging: self::generated::hayride::core::types::Logging {
                            enabled: cli.logging.enabled,
                            level: cli.logging.level,
                            file: cli.logging.file,
                        },
                    },
                ),
                Morph::Server(server) => self::generated::hayride::core::types::Morph::Server(
                    self::generated::hayride::core::types::Server {
                        bin: server.bin,
                        logging: self::generated::hayride::core::types::Logging {
                            enabled: server.logging.enabled,
                            level: server.logging.level,
                            file: server.logging.file,
                        },
                        http: self::generated::hayride::core::types::Http {
                            address: server.http.address,
                        },
                    },
                ),
            })
            .collect();

        let features = value
            .features
            .into_iter()
            .map(|f| match f {
                Feature::Ai(ai) => self::generated::hayride::core::types::Feature::Ai(
                    self::generated::hayride::core::types::Ai {
                        bin: ai.bin,
                        logging: self::generated::hayride::core::types::Logging {
                            enabled: ai.logging.enabled,
                            level: ai.logging.level,
                            file: ai.logging.file,
                        },
                        http: self::generated::hayride::core::types::Http {
                            address: ai.http.address,
                        },
                        websocket: self::generated::hayride::core::types::Websocket {
                            address: ai.websocket.address,
                        },
                    },
                ),
                Feature::Ui(ui) => self::generated::hayride::core::types::Feature::Ui(
                    self::generated::hayride::core::types::Ui {
                        bin: ui.bin,
                        logging: self::generated::hayride::core::types::Logging {
                            enabled: ui.logging.enabled,
                            level: ui.logging.level,
                            file: ui.logging.file,
                        },
                        http: self::generated::hayride::core::types::Http {
                            address: ui.http.address,
                        },
                    },
                ),
            })
            .collect();

        self::generated::hayride::core::types::Config {
            version: value.version,
            license: value.license,
            core,
            features,
        }
    }
}
