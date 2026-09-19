use crate::error::AppError;
use crate::models::{Config, Credentials, State};
use directories::ProjectDirs;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub base_dir: PathBuf,
    pub config_path: PathBuf,
    pub credentials_path: PathBuf,
    pub state_path: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self, AppError> {
        let base_dir = if let Ok(value) = env::var("STELLAR_CARD_HOME") {
            PathBuf::from(value)
        } else {
            let dirs =
                ProjectDirs::from("dev", "stellar-card", "stellar-card").ok_or_else(|| {
                    AppError::new(
                        crate::error::ErrorCode::General,
                        "Unable to resolve config directory",
                    )
                })?;
            dirs.config_dir().to_path_buf()
        };
        fs::create_dir_all(&base_dir)?;
        Ok(Self {
            config_path: base_dir.join("config.toml"),
            credentials_path: base_dir.join("credentials.toml"),
            state_path: base_dir.join("state.json"),
            base_dir,
        })
    }
}

pub fn load_config(paths: &AppPaths) -> Result<Config, AppError> {
    if !paths.config_path.exists() {
        return Ok(Config::default());
    }
    let content = fs::read_to_string(&paths.config_path)?;
    Ok(toml::from_str(&content)?)
}

pub fn save_config(paths: &AppPaths, config: &Config) -> Result<(), AppError> {
    write_text(&paths.config_path, toml::to_string_pretty(config)?)
}

pub fn load_credentials(paths: &AppPaths) -> Result<Option<Credentials>, AppError> {
    if !paths.credentials_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&paths.credentials_path)?;
    Ok(Some(toml::from_str(&content)?))
}

pub fn save_credentials(paths: &AppPaths, credentials: &Credentials) -> Result<(), AppError> {
    write_text(
        &paths.credentials_path,
        toml::to_string_pretty(credentials)?,
    )
}

pub fn load_state(paths: &AppPaths) -> Result<State, AppError> {
    if !paths.state_path.exists() {
        return Ok(State::default());
    }
    let content = fs::read_to_string(&paths.state_path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn save_state(paths: &AppPaths, state: &State) -> Result<(), AppError> {
    write_text(&paths.state_path, serde_json::to_string_pretty(state)?)
}

fn write_text(path: &Path, text: String) -> Result<(), AppError> {
    fs::write(path, text)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(path, perms)?;
    }
    Ok(())
}
