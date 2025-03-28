#[derive(Clone, Debug)]
pub struct Config {
    pub version: String,
    pub license: String,
    pub logging: Logging,
    pub morphs: Morphs,
}

#[derive(Clone, Debug)]
pub struct Logging {
    pub enabled: bool,
    pub level: String,
    pub file: String,
}

#[derive(Clone, Debug)]
pub struct Morphs {
    pub server: Server,
    pub ai: Ai,
}

#[derive(Clone, Debug)]
pub struct Server {
    pub http: Http,
}

#[derive(Clone, Debug)]
pub struct Ai {
    pub websocket: Websocket,
    pub http: Http,
    pub llm: Llm,
}

#[derive(Clone, Debug)]
pub struct Http {
    pub address: String,
}

#[derive(Clone, Debug)]
pub struct Websocket {
    pub address: String,
}

#[derive(Clone, Debug)]
pub struct Llm {
    pub model: String,
}
