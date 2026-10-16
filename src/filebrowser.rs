//! 文件浏览纯逻辑:目录列举、排序、过滤与路径工具。
//!
//! 界面上的内置文件对话框(src/file_dialog.rs)只负责画界面与响应鼠标键盘;
//! 「目录怎么列、按什么顺序、哪些该显示」全部在这里实现,不依赖 GUI,可单测。

use std::path::{Path, PathBuf};

/// 一个目录项(文件或文件夹)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    /// 文件字节数(文件夹为 0)
    pub size: u64,
}

impl Entry {
    /// 类型列显示:文件夹 / 后缀大写 / 文件
    pub fn kind_label(&self) -> String {
        if self.is_dir {
            return "文件夹".to_owned();
        }
        match Path::new(&self.name).extension().and_then(|e| e.to_str()) {
            Some(ext) if !ext.is_empty() => ext.to_uppercase(),
            _ => "文件".to_owned(),
        }
    }

    /// 大小列显示:文件夹不显示大小
    pub fn size_label(&self) -> String {
        if self.is_dir {
            "—".to_owned()
        } else {
            format_size(self.size)
        }
    }
}

/// 列目录
///
/// - 文件夹在前、文件在后,各自按名称排序(不区分大小写)
/// - 不返回 `.` / `..`(父目录入口由调用方决定是否加)
/// - 读不到元数据的项(失效软链等)直接跳过,不让个别坏项毁掉整个目录
/// - 软链跟随目标:链到文件夹的要能进去
pub fn list_dir(dir: &Path) -> Result<Vec<Entry>, String> {
    let read = std::fs::read_dir(dir).map_err(|e| read_error(dir, &e))?;
    let mut entries = Vec::new();
    for item in read.flatten() {
        let path = item.path();
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        entries.push(Entry {
            name: item.file_name().to_string_lossy().into_owned(),
            path,
            is_dir: meta.is_dir(),
            size: if meta.is_file() { meta.len() } else { 0 },
        });
    }
    sort_entries(&mut entries);
    Ok(entries)
}

/// 排序:文件夹优先,然后名称不区分大小写(完全同名时按原始名兜底,保证结果稳定)
pub fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// 读目录失败的提示文案(区分「不存在」「没权限」这两种常见情况)
fn read_error(dir: &Path, err: &std::io::Error) -> String {
    match err.kind() {
        std::io::ErrorKind::NotFound => format!("目录不存在: {}", dir.display()),
        std::io::ErrorKind::PermissionDenied => format!("没有权限访问: {}", dir.display()),
        _ => format!("无法读取目录 {}: {err}", dir.display()),
    }
}

/// 上一级目录(已经是根目录时返回 None)
pub fn parent_dir(dir: &Path) -> Option<PathBuf> {
    dir.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

/// 把输入框里的路径展开成绝对路径:`~` 开头按主目录,相对路径按当前目录
pub fn expand_input_path(text: &str, cwd: &Path) -> PathBuf {
    let text = text.trim();
    if text == "~" {
        return home_dir();
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    let path = PathBuf::from(text);
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

/// 主目录:$HOME,取不到时退到根目录
pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

/// 快捷位置(只保留真实存在的目录):主目录、桌面、文档、下载、根目录。
/// 桌面/文档/下载先按 XDG 用户目录(中文桌面环境常是 ~/桌面、~/下载),再退回英文名。
pub fn places() -> Vec<(&'static str, PathBuf)> {
    let home = home_dir();
    let user_dirs = user_dirs_file().and_then(|p| std::fs::read_to_string(p).ok());
    let mut out: Vec<(&'static str, PathBuf)> = vec![("主目录", home.clone())];
    for (label, key, fallback) in [
        ("桌面", "XDG_DESKTOP_DIR", "Desktop"),
        ("文档", "XDG_DOCUMENTS_DIR", "Documents"),
        ("下载", "XDG_DOWNLOAD_DIR", "Downloads"),
    ] {
        let path = user_dirs
            .as_deref()
            .and_then(|text| parse_user_dir(text, key, &home))
            .unwrap_or_else(|| home.join(fallback));
        if path.is_dir() && !out.iter().any(|(_, p)| *p == path) {
            out.push((label, path));
        }
    }
    out.push(("根目录", PathBuf::from("/")));
    out
}

/// XDG 用户目录配置文件的路径(不存在时返回 None)
fn user_dirs_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"));
    let path = base.join("user-dirs.dirs");
    path.is_file().then_some(path)
}

/// 从 user-dirs.dirs 文本里取一个键的路径
///
/// 行格式:`XDG_DESKTOP_DIR="$HOME/桌面"`;值为空表示该项被禁用,返回 None。
pub fn parse_user_dir(content: &str, key: &str, home: &Path) -> Option<PathBuf> {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != key {
            continue;
        }
        // 去掉两侧引号(值里可能带空格)
        let value = value.trim().trim_matches('"');
        if value.is_empty() {
            return None;
        }
        return Some(match value.strip_prefix("$HOME/") {
            Some(rest) => home.join(rest),
            None => PathBuf::from(value),
        });
    }
    None
}

/// 是否隐藏项(以 `.` 开头)
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

/// 名称是否包含关键字(不区分大小写;空关键字恒为真)
pub fn name_matches(name: &str, query: &str) -> bool {
    let query = query.trim();
    if query.is_empty() {
        return true;
    }
    name.to_lowercase().contains(&query.to_lowercase())
}

/// 后缀是否在允许列表内(列表为空 = 全部允许;比较不区分大小写)
pub fn ext_allowed(name: &str, exts: &[String]) -> bool {
    if exts.is_empty() {
        return true;
    }
    match Path::new(name).extension().and_then(|e| e.to_str()) {
        Some(ext) => exts.iter().any(|allowed| allowed.eq_ignore_ascii_case(ext)),
        None => false,
    }
}

/// 目录列表的过滤条件
pub struct FilterOpts<'a> {
    pub show_hidden: bool,
    /// 名称关键字
    pub query: &'a str,
    /// 允许的后缀(小写、不含点);空 = 不限
    pub exts: &'a [String],
}

/// 过滤目录项,返回命中的下标(保持原有顺序:文件夹优先)
///
/// 返回下标而不是引用:界面按行下标取值,避免为了绕开借用冲突而整表克隆。
/// 关键字对文件夹也生效(输入「2024」时同级里的 2024 年目录会浮出来);
/// 后缀只约束文件,文件夹永远可见可进入。
pub fn filter_indices(entries: &[Entry], opts: &FilterOpts<'_>) -> Vec<usize> {
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            (opts.show_hidden || !is_hidden(&e.name))
                && name_matches(&e.name, opts.query)
                && (e.is_dir || ext_allowed(&e.name, opts.exts))
        })
        .map(|(index, _)| index)
        .collect()
}

/// 人类可读的文件大小(1024 进制)
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        return format!("{bytes} B");
    }
    let digits = if value >= 100.0 {
        0
    } else if value >= 10.0 {
        1
    } else {
        2
    };
    format!("{value:.digits$} {}", UNITS[unit])
}

/// 保存时补后缀:已经有相同后缀(不分大小写)就原样返回,否则追加 `.ext`
pub fn ensure_extension(name: &str, ext: &str) -> String {
    if name.is_empty() || ext.is_empty() {
        return name.to_owned();
    }
    if name
        .to_lowercase()
        .ends_with(&format!(".{}", ext.to_lowercase()))
    {
        return name.to_owned();
    }
    format!("{name}.{ext}")
}
