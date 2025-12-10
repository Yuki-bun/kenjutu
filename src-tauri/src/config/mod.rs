use std::fs::File;
use std::path::PathBuf;

pub fn get_config_dir() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|dir| dir.join("pr-manager"))
        .ok_or_else(|| "Could not find config directory".to_string())
}

pub fn get_token_path() -> Result<PathBuf, String> {
    get_config_dir().map(|dir| dir.join("token"))
}

pub fn load_token() -> Result<String, String> {
    let token_path = get_token_path()?;
    let file =
        File::open(&token_path).map_err(|err| format!("Failed to open token file: {}", err))?;
    let token = std::io::read_to_string(file)
        .map_err(|err| format!("Failed to read token file: {}", err))?;
    Ok(token.trim().to_string())
}
