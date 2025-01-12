use anyhow::{anyhow, Result};
use directories::BaseDirs;
use std::{fs, path::PathBuf};

pub fn app_data_path() -> Result<PathBuf> {
    let Some(base_dirs) = BaseDirs::new() else {
        return Err(anyhow!("Could not find base_dirs"));
    };

    let mut app_data_path = base_dirs.data_dir().to_path_buf();

    #[cfg(debug_assertions)]
    {
        app_data_path = app_data_path.join("rust-text-debug");
    }

    #[cfg(not(debug_assertions))]
    {
        app_data_path = app_data_path.join("rust-text");
    }

    if !app_data_path.exists() {
        _ = fs::create_dir(&app_data_path);
    }

    Ok(app_data_path)
}
