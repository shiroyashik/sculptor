pub const LOGGER_ENV: &'static str = "RUST_LOG";
pub const CONFIG_ENV: &'static str = "RUST_CONFIG";
pub const LOGS_ENV: &'static str = "LOGS_FOLDER";

pub const SCULPTOR_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY: &'static str = "shiroyashik/sculptor";

pub const USER_AGENT: &'static str = "reqwest";
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub const FIGURA_RELEASES_URL: &'static str = "https://api.github.com/repos/figuramc/figura/releases";
pub const FIGURA_DEFAULT_VERSION: &'static str = "0.1.4";