//! 与 Go 版兼容的 TCP 协议。所有网络/磁盘操作只在工作线程执行。
use anyhow::{Context, Result, bail, ensure};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};

pub const DEFAULT_PORT: u16 = 23456;
const CHUNK: usize = 32 * 1024;
const IO_POLL: Duration = Duration::from_millis(200);
const IDLE: Duration = Duration::from_secs(10);

#[derive(Debug, Clone)]
pub enum Event {
    Listening {
        addresses: Vec<String>,
        port: u16,
        code: String,
    },
    Status(String),
    RetryCode(u8),
    Progress {
        done: u64,
        total: u64,
        speed: f64,
    },
    Finished(PathBuf),
    Failed(String),
}

#[derive(Default)]
pub struct Control {
    cancelled: AtomicBool,
    stream: Mutex<Option<TcpStream>>,
}
impl Control {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        if let Ok(guard) = self.stream.lock()
            && let Some(s) = guard.as_ref()
        {
            let _ = s.shutdown(Shutdown::Both);
        }
    }
    fn check(&self) -> Result<()> {
        ensure!(!self.cancelled.load(Ordering::Acquire), "已取消传输");
        Ok(())
    }
    fn attach(&self, s: &TcpStream) -> Result<()> {
        *self
            .stream
            .lock()
            .map_err(|_| anyhow::anyhow!("连接状态异常"))? = Some(s.try_clone()?);
        self.check()
    }
}

pub struct Task {
    pub events: mpsc::Receiver<Event>,
    pub control: Arc<Control>,
    pub codes: mpsc::Sender<String>,
}
impl Drop for Task {
    fn drop(&mut self) {
        self.control.cancel();
    }
}

pub enum Request {
    Send {
        path: PathBuf,
        port: u16,
    },
    Receive {
        address: String,
        code: String,
        directory: PathBuf,
    },
}
pub fn start(request: Request) -> Task {
    let (tx, events) = mpsc::channel();
    let (codes, rx) = mpsc::channel();
    let control = Arc::new(Control::default());
    let worker = control.clone();
    std::thread::spawn(move || {
        let result = match request {
            Request::Send { path, port } => send(&path, port, &worker, &tx),
            Request::Receive {
                address,
                code,
                directory,
            } => receive(&address, code, &directory, &worker, &tx, &rx),
        };
        let event = match result {
            Ok(path) => Event::Finished(path),
            Err(e) => Event::Failed(format!("{e:#}")),
        };
        let _ = tx.send(event);
    });
    Task {
        events,
        control,
        codes,
    }
}

pub fn normalize_address(input: &str) -> Result<String> {
    let s = input.trim();
    ensure!(!s.is_empty(), "请输入发送方地址");
    if let Ok(ip) = s.parse::<std::net::IpAddr>() {
        return Ok(format_address(&ip.to_string(), DEFAULT_PORT));
    }
    if s.starts_with('[') && s.ends_with(']') {
        let ip: std::net::Ipv6Addr = s[1..s.len() - 1].parse().context("IPv6 地址无效")?;
        return Ok(format!("[{ip}]:{DEFAULT_PORT}"));
    }
    let (host, port) = match s.rsplit_once(':') {
        Some((host, port)) => (host, port.parse::<u16>().context("端口必须是 1–65535")?),
        None => (s, DEFAULT_PORT),
    };
    ensure!(
        !host.is_empty() && !host.chars().any(char::is_whitespace) && port != 0,
        "地址或端口无效"
    );
    Ok(format!("{host}:{port}"))
}
fn format_address(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}
pub fn local_addresses() -> Vec<String> {
    let mut ips: Vec<String> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_loopback())
        .filter_map(|i| match i.ip() {
            std::net::IpAddr::V4(ip) if !ip.is_link_local() => Some(ip.to_string()),
            _ => None,
        })
        .collect();
    ips.sort();
    ips.dedup();
    if ips.is_empty() {
        ips.push("127.0.0.1".into());
    }
    ips
}
fn valid_code(code: &str) -> Result<()> {
    ensure!(
        code.len() == 4 && code.bytes().all(|b| b.is_ascii_digit()),
        "取件码必须是 4 位数字"
    );
    Ok(())
}
fn valid_filename(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty()
            && name != "."
            && name != ".."
            && !name.ends_with(['.', ' '])
            && !name
                .chars()
                .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c)),
        "文件名不安全或无法跨平台传输"
    );
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    ensure!(
        ![
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
            "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9"
        ]
        .contains(&stem.as_str()),
        "不支持 Windows 保留文件名"
    );
    Ok(())
}

// 超时只用于轮询取消；真正的空闲超时独立计时，不因分片重试而破坏协议。
struct Wire<'a> {
    reader: BufReader<TcpStream>,
    control: &'a Control,
    idle: Duration,
}
impl<'a> Wire<'a> {
    fn new(s: TcpStream, control: &'a Control) -> Result<Self> {
        s.set_read_timeout(Some(IO_POLL))?;
        s.set_write_timeout(Some(IO_POLL))?;
        control.attach(&s)?;
        Ok(Self {
            reader: BufReader::new(s),
            control,
            idle: Duration::from_secs(30),
        })
    }
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let start = Instant::now();
        loop {
            self.control.check()?;
            match self.reader.read(buf) {
                Ok(n) => return Ok(n),
                Err(e) if transient(&e) && start.elapsed() < self.idle => (),
                Err(e) => return Err(e).context("读取连接失败或空闲超时"),
            }
        }
    }
    fn line(&mut self) -> Result<String> {
        let mut bytes = Vec::new();
        loop {
            let mut b = [0];
            ensure!(self.read(&mut b)? != 0, "连接已断开");
            if b[0] == b'\n' {
                break;
            }
            bytes.push(b[0]);
            ensure!(bytes.len() <= 8192, "协议行过长");
        }
        Ok(String::from_utf8(bytes)
            .context("协议文本不是 UTF-8")?
            .trim_end_matches('\r')
            .to_owned())
    }
    fn write(&mut self, mut bytes: &[u8]) -> Result<()> {
        let mut last = Instant::now();
        while !bytes.is_empty() {
            self.control.check()?;
            match self.reader.get_mut().write(bytes) {
                Ok(0) => bail!("连接已断开"),
                Ok(n) => {
                    bytes = &bytes[n..];
                    last = Instant::now();
                }
                Err(e) if transient(&e) && last.elapsed() < IDLE => (),
                Err(e) => return Err(e).context("写入连接失败或空闲超时"),
            }
        }
        Ok(())
    }
    fn expect(&mut self, expected: &str) -> Result<()> {
        let line = self.line()?;
        ensure!(line == expected, "协议错误：期望 {expected}，收到 {line}");
        Ok(())
    }
}
fn transient(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}
fn progress(tx: &mpsc::Sender<Event>, done: u64, total: u64, start: Instant) {
    let _ = tx.send(Event::Progress {
        done,
        total,
        speed: done as f64 / start.elapsed().as_secs_f64().max(0.001),
    });
}
fn send(path: &Path, port: u16, control: &Control, tx: &mpsc::Sender<Event>) -> Result<PathBuf> {
    let mut file = File::open(path).context("无法打开待发送文件")?;
    let meta = file.metadata()?;
    ensure!(meta.is_file(), "只能发送普通文件");
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .context("文件名不是 UTF-8")?;
    valid_filename(name)?;
    let listener = TcpListener::bind(("0.0.0.0", port))
        .or_else(|_| TcpListener::bind(("0.0.0.0", 0)))
        .context("无法监听端口")?;
    listener.set_nonblocking(true)?;
    let mut random = [0u8; 4];
    getrandom::getrandom(&mut random).context("无法生成安全取件码")?;
    let code = format!("{:04}", u32::from_le_bytes(random) % 10000);
    let _ = tx.send(Event::Listening {
        addresses: local_addresses(),
        port: listener.local_addr()?.port(),
        code: code.clone(),
    });
    let s = loop {
        control.check()?;
        match listener.accept() {
            Ok((s, _)) => break s,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(IO_POLL),
            Err(e) => return Err(e).context("接收连接失败"),
        }
    };
    drop(listener);
    let mut wire = Wire::new(s, control)?;
    let mut authenticated = false;
    for _ in 0..3 {
        if wire.line()? == format!("CODE {code}") {
            wire.write(b"OK\n")?;
            authenticated = true;
            break;
        }
        wire.write(b"ERR\n")?;
    }
    ensure!(authenticated, "取件码错误已达 3 次");
    wire.idle = IDLE;
    let _ = tx.send(Event::Status("已连接，正在发送".into()));
    wire.write(format!("FILE {name} {}\n", meta.len()).as_bytes())?;
    wire.expect("READY")?;
    let start = Instant::now();
    let mut last = start;
    let mut done = 0;
    let mut buffer = [0u8; CHUNK];
    while done < meta.len() {
        control.check()?;
        let len = ((meta.len() - done).min(CHUNK as u64)) as usize;
        let n = file.read(&mut buffer[..len])?;
        ensure!(n > 0, "发送期间文件变短");
        wire.write(&buffer[..n])?;
        done += n as u64;
        if last.elapsed() >= IO_POLL || done == meta.len() {
            progress(tx, done, meta.len(), start);
            last = Instant::now();
        }
    }
    progress(tx, done, meta.len(), start);
    wire.write(b"EOF\n")?;
    wire.expect("ACK")?;
    wire.write(b"BYE\n")?;
    Ok(path.to_path_buf())
}

fn connect(address: &str, control: &Control) -> Result<TcpStream> {
    // DNS 单独解析，防止阻塞 GUI；连接每 200ms 检查取消，总预算 30 秒。
    let addresses: Vec<_> = address
        .to_socket_addrs()
        .context("无法解析发送方地址")?
        .collect();
    ensure!(!addresses.is_empty(), "未找到发送方地址");
    let start = Instant::now();
    let mut last = None;
    while start.elapsed() < Duration::from_secs(30) {
        for a in &addresses {
            control.check()?;
            match TcpStream::connect_timeout(a, IO_POLL) {
                Ok(s) => return Ok(s),
                Err(e) => last = Some(e),
            }
        }
        std::thread::sleep(IO_POLL);
    }
    bail!(
        "无法连接发送方：{}",
        last.map(|e| e.to_string()).unwrap_or_default()
    )
}
struct PartialFile {
    path: PathBuf,
    keep: bool,
}
impl Drop for PartialFile {
    fn drop(&mut self) {
        if !self.keep {
            let _ = fs::remove_file(&self.path);
        }
    }
}
fn receive(
    address: &str,
    mut code: String,
    directory: &Path,
    control: &Control,
    tx: &mpsc::Sender<Event>,
    codes: &mpsc::Receiver<String>,
) -> Result<PathBuf> {
    let address = normalize_address(address)?;
    valid_code(&code)?;
    let _ = tx.send(Event::Status(format!("正在连接 {address}")));
    let mut wire = Wire::new(connect(&address, control)?, control)?;
    for attempt in 1..=3 {
        wire.write(format!("CODE {code}\n").as_bytes())?;
        let reply = wire.line()?;
        if reply == "OK" {
            break;
        }
        ensure!(reply == "ERR", "发送方返回无效响应：{reply}");
        ensure!(attempt < 3, "取件码错误已达 3 次");
        let _ = tx.send(Event::RetryCode(3 - attempt));
        let start = Instant::now();
        loop {
            control.check()?;
            ensure!(
                start.elapsed() < Duration::from_secs(25),
                "等待新取件码超时，请重新发送"
            );
            match codes.recv_timeout(IO_POLL) {
                Ok(c) => {
                    if valid_code(&c).is_ok() {
                        code = c;
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(_) => bail!("已取消接收"),
            }
        }
    }
    wire.idle = IDLE;
    let line = wire.line()?;
    let (name, size) = line
        .strip_prefix("FILE ")
        .and_then(|s| s.rsplit_once(' '))
        .context("无效的文件元数据")?;
    valid_filename(name)?;
    let total: u64 = size.parse().context("文件大小无效")?;
    ensure!(total <= i64::MAX as u64, "文件大小超出协议范围");
    fs::create_dir_all(directory).context("无法创建保存目录")?;
    if fs2::available_space(directory).context("无法检查磁盘空间")? < total {
        wire.write(b"ERR_NOSPC\n")?;
        bail!("磁盘剩余空间不足");
    }
    // create_new 杜绝覆盖或跟随已有符号链接；失败/取消自动清理未完成文件。
    let mut selected = None;
    for index in 0..10000 {
        let filename = if index == 0 {
            name.to_owned()
        } else {
            let p = Path::new(name);
            let stem = p.file_stem().unwrap_or_default().to_string_lossy();
            match p.extension() {
                Some(ext) => format!("{stem} ({index}).{}", ext.to_string_lossy()),
                None => format!("{stem} ({index})"),
            }
        };
        let path = directory.join(filename);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(f) => {
                selected = Some((path, f));
                break;
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
            Err(e) => return Err(e).context("无法创建接收文件"),
        }
    }
    let (path, mut file) = selected.context("同名文件过多，请更换保存目录")?;
    let mut partial = PartialFile {
        path: path.clone(),
        keep: false,
    };
    // Windows 下先关闭文件再执行 PartialFile 的清理。
    let result = (|| -> Result<()> {
        wire.write(b"READY\n")?;
        let _ = tx.send(Event::Status(format!("正在接收 {name}")));
        let start = Instant::now();
        let mut last = start;
        let mut done = 0;
        let mut buffer = [0u8; CHUNK];
        while done < total {
            let len = ((total - done).min(CHUNK as u64)) as usize;
            let n = wire.read(&mut buffer[..len])?;
            ensure!(n > 0, "连接中断，文件未接收完整");
            file.write_all(&buffer[..n]).context("写入接收文件失败")?;
            done += n as u64;
            if last.elapsed() >= IO_POLL || done == total {
                progress(tx, done, total, start);
                last = Instant::now();
            }
        }
        wire.expect("EOF")?;
        file.sync_all().context("保存文件失败")?;
        control.check()?;
        // 数据已经完整落盘。即使确认包丢失也保留文件，发送端会明确报告失败。
        partial.keep = true;
        wire.write(b"ACK\n")?;
        let _ = wire.line(); // 兼容旧版：BYE 是可选结束标记。
        progress(tx, done, total, start);
        Ok(())
    })();
    drop(file);
    result?;
    Ok(path)
}

pub fn format_bytes(bytes: f64) -> String {
    for (unit, scale) in [("GB", 1073741824.0), ("MB", 1048576.0), ("KB", 1024.0)] {
        if bytes >= scale {
            return format!("{:.2} {unit}", bytes / scale);
        }
    }
    format!("{bytes:.0} B")
}
