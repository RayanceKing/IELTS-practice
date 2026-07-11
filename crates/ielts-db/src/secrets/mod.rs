//! OS keychain / Stronghold-style secret vault abstraction (Phase 4).
//!
//! Default implementation is a file-backed vault under app data for
//! development and tests. Production can swap to OS keyring later.
//! SQLite only stores `SecretRef` pointers, never secret bytes.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::sqlite::{DbError, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct VaultFile {
    version: u32,
    entries: HashMap<String, VaultEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VaultEntry {
    ref_id: String,
    /// Base64 is fine for local dev vault; not a substitute for OS keychain.
    secret_b64: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SecretVault {
    path: PathBuf,
}

impl SecretVault {
    pub fn open(path: impl Into<PathBuf>) -> DbResult<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            let empty = VaultFile {
                version: 1,
                entries: HashMap::new(),
            };
            let json = serde_json::to_vec_pretty(&empty)
                .map_err(|e| DbError::Message(e.to_string()))?;
            fs::write(&path, json)?;
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn set_secret(&self, name: &str, secret: &str) -> DbResult<String> {
        if name.trim().is_empty() {
            return Err(DbError::Validation("secret name required".into()));
        }
        if secret.is_empty() {
            return Err(DbError::Validation("secret value required".into()));
        }
        let mut vault = self.load()?;
        let ref_id = format!(
            "kv:{}:{}",
            short_hash(name),
            Uuid::new_v4().simple()
        );
        let entry = VaultEntry {
            ref_id: ref_id.clone(),
            secret_b64: base64_encode(secret.as_bytes()),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        vault.entries.insert(name.to_string(), entry);
        self.store(&vault)?;
        Ok(ref_id)
    }

    pub fn get_secret_by_ref(&self, ref_id: &str) -> DbResult<Option<String>> {
        let vault = self.load()?;
        for entry in vault.entries.values() {
            if entry.ref_id == ref_id {
                let bytes = base64_decode(&entry.secret_b64)?;
                let s = String::from_utf8(bytes)
                    .map_err(|e| DbError::Message(format!("secret utf8: {e}")))?;
                return Ok(Some(s));
            }
        }
        Ok(None)
    }

    pub fn delete_secret(&self, name: &str) -> DbResult<bool> {
        let mut vault = self.load()?;
        let removed = vault.entries.remove(name).is_some();
        if removed {
            self.store(&vault)?;
        }
        Ok(removed)
    }

    fn load(&self) -> DbResult<VaultFile> {
        let raw = fs::read_to_string(&self.path)?;
        serde_json::from_str(&raw).map_err(|e| DbError::Message(format!("vault parse: {e}")))
    }

    fn store(&self, vault: &VaultFile) -> DbResult<()> {
        let json =
            serde_json::to_vec_pretty(vault).map_err(|e| DbError::Message(e.to_string()))?;
        fs::write(&self.path, json)?;
        Ok(())
    }
}

fn short_hash(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..8].to_string()
}

fn base64_encode(bytes: &[u8]) -> String {
    // Minimal base64 without extra crate dependency.
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i] as u32;
        let b1 = if i + 1 < bytes.len() {
            bytes[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            bytes[i + 2] as u32
        } else {
            0
        };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn base64_decode(input: &str) -> DbResult<Vec<u8>> {
    fn val(c: u8) -> DbResult<u8> {
        match c {
            b'A'..=b'Z' => Ok(c - b'A'),
            b'a'..=b'z' => Ok(c - b'a' + 26),
            b'0'..=b'9' => Ok(c - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(DbError::Validation("invalid base64".into())),
        }
    }
    let bytes = input.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err(DbError::Validation("invalid base64 length".into()));
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let a = val(bytes[i])?;
        let b = val(bytes[i + 1])?;
        let c = if bytes[i + 2] == b'=' {
            0
        } else {
            val(bytes[i + 2])?
        };
        let d = if bytes[i + 3] == b'=' {
            0
        } else {
            val(bytes[i + 3])?
        };
        let triple = ((a as u32) << 18) | ((b as u32) << 12) | ((c as u32) << 6) | (d as u32);
        out.push(((triple >> 16) & 0xFF) as u8);
        if bytes[i + 2] != b'=' {
            out.push(((triple >> 8) & 0xFF) as u8);
        }
        if bytes[i + 3] != b'=' {
            out.push((triple & 0xFF) as u8);
        }
        i += 4;
    }
    Ok(out)
}
