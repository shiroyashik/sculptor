// Environment
pub const LOGGER_ENV: &str = "RUST_LOG";
pub const CONFIG_ENV: &str = "RUST_CONFIG";
pub const LOGS_ENV: &str = "LOGS_FOLDER";
pub const ASSETS_ENV: &str = "ASSETS_FOLDER";
pub const AVATARS_ENV: &str = "AVATARS_FOLDER";

// Instance info
pub const SCULPTOR_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPOSITORY: &str = "shiroyashik/sculptor";

// reqwest parameters
pub const USER_AGENT: &str = "reqwest";
pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

// Figura update checker
pub const FIGURA_RELEASES_URL: &str = "https://api.github.com/repos/figuramc/figura/releases";
pub const FIGURA_DEFAULT_VERSION: &str = "0.1.5";

// Figura Assets
pub const FIGURA_ASSETS_ZIP_URL: &str = "https://github.com/FiguraMC/Assets/archive/refs/heads/main.zip";
pub const FIGURA_ASSETS_COMMIT_URL: &str = "https://api.github.com/repos/FiguraMC/Assets/commits/main";