use std::fs;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPaths {
    pub app_data: PathBuf,
    pub logs: PathBuf,
    pub backups: PathBuf,
    pub imports: PathBuf,
    pub exports: PathBuf,
    pub diagnostics: PathBuf,
    pub db_dir: PathBuf,
    pub legacy_candidates: Vec<PathBuf>,
}

impl AppPaths {
    pub fn discover() -> Self {
        let app_data = default_app_data_dir();
        let logs = app_data.join("logs");
        let backups = app_data.join("backups");
        let imports = app_data.join("imports");
        let exports = app_data.join("exports");
        let diagnostics = app_data.join("diagnostics");
        let db_dir = app_data.join("db");
        let legacy_candidates = discover_legacy_dirs();

        Self {
            app_data,
            logs,
            backups,
            imports,
            exports,
            diagnostics,
            db_dir,
            legacy_candidates,
        }
    }

    pub fn ensure_layout(&self) -> std::io::Result<()> {
        for dir in [
            &self.app_data,
            &self.logs,
            &self.backups,
            &self.imports,
            &self.exports,
            &self.diagnostics,
            &self.db_dir,
        ] {
            fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

fn default_app_data_dir() -> PathBuf {
    if let Some(base) = std::env::var_os("APPDATA") {
        return PathBuf::from(base).join("IELTS Practice");
    }
    if let Some(base) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(base).join("ielts-practice");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("IELTS Practice");
    }
    std::env::temp_dir().join("ielts-practice")
}

/// Locate historical Electron / browser data directories without migrating them.
pub fn discover_legacy_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut push_if_exists = |path: PathBuf| {
        if path.exists() && !out.iter().any(|p| p == &path) {
            out.push(path);
        }
    };

    if let Some(appdata) = std::env::var_os("APPDATA") {
        let base = PathBuf::from(appdata);
        push_if_exists(base.join("IELTS Practice"));
        push_if_exists(base.join("ielts-practice"));
        push_if_exists(base.join("ielts-writing"));
        // electron-builder default style
        push_if_exists(base.join("ielts-practice-app"));
    }

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        push_if_exists(
            home.join("Library")
                .join("Application Support")
                .join("IELTS Practice"),
        );
        push_if_exists(
            home.join("Library")
                .join("Application Support")
                .join("ielts-practice"),
        );
        push_if_exists(home.join(".config").join("ielts-practice"));
        push_if_exists(home.join(".config").join("IELTS Practice"));
    }

    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        let base = PathBuf::from(local);
        push_if_exists(base.join("IELTS Practice"));
        push_if_exists(base.join("ielts-practice"));
    }

    out
}
