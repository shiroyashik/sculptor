// Environment
pub const LOGGER_ENV: &'static str = "RUST_LOG";
pub const CONFIG_ENV: &'static str = "RUST_CONFIG";
pub const LOGS_ENV: &'static str = "LOGS_FOLDER";
pub const ASSETS_ENV: &'static str = "ASSETS_FOLDER";
pub const AVATARS_ENV: &'static str = "AVATARS_FOLDER";

// Sculptor update checker
pub const SCULPTOR_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY: &'static str = "shiroyashik/sculptor";

// reqwest parameters
pub const USER_AGENT: &'static str = "reqwest";
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

// Figura update checker
pub const FIGURA_RELEASES_URL: &'static str = "https://api.github.com/repos/figuramc/figura/releases";
pub const FIGURA_DEFAULT_VERSION: &'static str = "0.1.4";

// Figura Assets
pub const FIGURA_ASSETS_ZIP_URL: &'static str = "https://github.com/FiguraMC/Assets/archive/refs/heads/main.zip";
pub const FIGURA_ASSETS_COMMIT_URL: &'static str = "https://api.github.com/repos/FiguraMC/Assets/commits/main";