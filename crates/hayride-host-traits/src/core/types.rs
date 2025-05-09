#[derive(Clone, Debug)]
pub struct Config {
    pub version: String,
    pub license: String,
    pub core: Vec<Morph>,
    pub features: Vec<Feature>,
}

#[derive(Clone, Debug)]
pub struct Logging {
    pub enabled: bool,
    pub level: String,
    pub file: String,
}

#[derive(Clone, Debug)]
pub enum Morph {
    Cli(Cli),
    Server(Server),
}

#[derive(Clone, Debug)]
pub enum Feature {
    Ai(Ai),
    Ui(Ui),
}

#[derive(Clone, Debug)]
pub struct Server {
    pub bin: String,
    pub logging: Logging,
    pub http: Http,
}

#[derive(Clone, Debug)]
pub struct Cli {
    pub bin: String,
    pub logging: Logging,
}

#[derive(Clone, Debug)]
pub struct Ai {
    pub bin: String,
    pub logging: Logging,
    pub websocket: Websocket,
    pub http: Http,
}

#[derive(Clone, Debug)]
pub struct Ui {
    pub bin: String,
    pub logging: Logging,
    pub http: Http,
}

#[derive(Clone, Debug)]
pub struct Http {
    pub address: String,
}

#[derive(Clone, Debug)]
pub struct Websocket {
    pub address: String,
}
