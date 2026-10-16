use crate::transfer::normalize_address;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Contact {
    pub address: String,
    pub note: String,
    #[serde(default)]
    pub updated_at: String,
}
pub fn path() -> Result<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|s| PathBuf::from(s).join(".config")))
    }
    .context("无法获取联系人配置目录")?;
    Ok(base.join("chuan/contacts.json"))
}
pub fn load() -> Result<Vec<Contact>> {
    let path = path()?;
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).context("读取联系人失败"),
    };
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    let mut contacts: Vec<Contact> = serde_json::from_slice(&bytes)
        .context("联系人文件损坏，已保留原文件；请修复或备份后移走该文件")?;
    contacts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    for c in &mut contacts {
        if let Ok(addr) = normalize_address(&c.address) {
            c.address = addr;
        }
    }
    Ok(contacts)
}
pub fn save(contacts: &[Contact]) -> Result<()> {
    let path = path()?;
    std::fs::create_dir_all(path.parent().context("配置路径无效")?)?;
    let bytes = serde_json::to_vec_pretty(contacts)?;
    // 保存前保留上次有效数据；损坏数据不会在 load 失败后被静默覆盖。
    if path.exists() {
        std::fs::copy(&path, path.with_extension("json.bak")).context("备份联系人失败")?;
    }
    std::fs::write(&path, bytes).context("保存联系人失败")
}
pub fn upsert(contact: Contact, original: &str) -> Result<Vec<Contact>> {
    let mut contacts = load()?;
    let address = normalize_address(&contact.address)?;
    contacts.retain(|c| c.address != address && c.address != original);
    contacts.insert(0, Contact { address, ..contact });
    save(&contacts)?;
    Ok(contacts)
}
