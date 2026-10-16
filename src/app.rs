mod shell;
mod theme;

use chuan_lib::{
    contacts::{self, Contact},
    transfer::{self, Event, Request, Task, format_bytes},
};
use eframe::egui::{self, Color32, RichText};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

#[derive(PartialEq)]
enum Page {
    Send,
    Receive,
    Contacts,
}
#[derive(Clone, Copy)]
enum DialogRequest {
    File,
    Directory,
}
pub struct ChuanApp {
    page: Page,
    file: String,
    directory: String,
    address: String,
    code: String,
    port: u16,
    status: String,
    error: bool,
    task: Option<Task>,
    retry: bool,
    done: u64,
    total: u64,
    speed: f64,
    finished: bool,
    share: String,
    logs: Vec<String>,
    contacts: Vec<Contact>,
    note: String,
    contact_address: String,
    original_address: String,
    delete_address: Option<String>,
    dialog_request: Option<DialogRequest>,
    #[cfg(target_os = "linux")]
    dialog: Option<(DialogRequest, crate::file_dialog::FileDialog)>,
    ready: Arc<AtomicBool>,
    log_path: Option<PathBuf>,
}
impl ChuanApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        ready: Arc<AtomicBool>,
        log_path: Option<PathBuf>,
    ) -> Self {
        install_cjk_font(&cc.egui_ctx);
        theme::configure(&cc.egui_ctx);
        let (contacts, status, error) = match contacts::load() {
            Ok(c) => (c, "就绪：双方需处于同一局域网".into(), false),
            Err(e) => (Vec::new(), format!("{e:#}"), true),
        };
        Self {
            page: Page::Send,
            file: String::new(),
            directory: std::env::current_dir()
                .unwrap_or_default()
                .display()
                .to_string(),
            address: String::new(),
            code: String::new(),
            port: transfer::DEFAULT_PORT,
            status,
            error,
            task: None,
            retry: false,
            done: 0,
            total: 0,
            speed: 0.0,
            finished: false,
            share: String::new(),
            logs: Vec::new(),
            contacts,
            note: String::new(),
            contact_address: String::new(),
            original_address: String::new(),
            delete_address: None,
            dialog_request: None,
            #[cfg(target_os = "linux")]
            dialog: None,
            ready,
            log_path,
        }
    }
    fn status(&mut self, message: String, error: bool) {
        self.status = message.clone();
        self.error = error;
        self.logs.push(message);
        if self.logs.len() > 200 {
            self.logs.remove(0);
        }
    }
    fn start(&mut self, request: Request) {
        self.done = 0;
        self.total = 0;
        self.speed = 0.0;
        self.finished = false;
        self.retry = false;
        self.share.clear();
        self.task = Some(transfer::start(request));
        self.status("正在启动传输…".into(), false);
    }
    fn poll(&mut self) {
        let events: Vec<_> = self
            .task
            .as_ref()
            .map(|t| t.events.try_iter().collect())
            .unwrap_or_default();
        for e in events {
            match e {
                Event::Listening {
                    addresses,
                    port,
                    code,
                } => {
                    self.share = format!(
                        "发送方地址：{}\n取件码：{code}",
                        addresses
                            .iter()
                            .map(|ip| format!("{ip}:{port}"))
                            .collect::<Vec<_>>()
                            .join("、")
                    );
                    self.status("等待接收方连接，请将地址和取件码告诉对方".into(), false);
                }
                Event::Status(s) => self.status(s, false),
                Event::RetryCode(n) => {
                    self.retry = true;
                    self.status(
                        format!("取件码错误，还可重试 {n} 次；请在 25 秒内重新提交"),
                        true,
                    );
                }
                Event::Progress { done, total, speed } => {
                    self.done = done;
                    self.total = total;
                    self.speed = speed;
                }
                Event::Finished(path) => {
                    self.task = None;
                    self.finished = true;
                    self.retry = false;
                    self.status(format!("传输完成：{}", path.display()), false);
                }
                Event::Failed(e) => {
                    self.task = None;
                    self.retry = false;
                    self.status(e, true);
                }
            }
        }
    }
    fn send_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("选择要发送的文件").strong().size(17.0));
        theme::description(ui, "支持拖入单个文件，也可以粘贴完整路径。");
        ui.add_enabled_ui(self.task.is_none(), |ui| {
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 112.0).max(80.0);
                ui.add(
                    theme::field(&mut self.file, "选择文件或将文件拖到窗口中").desired_width(width),
                );
                if ui.button("选择文件").clicked() {
                    self.dialog_request = Some(DialogRequest::File);
                }
            });
            ui.collapsing("端口设置", |ui| {
                ui.horizontal(|ui| {
                    ui.label("监听端口");
                    ui.add(egui::DragValue::new(&mut self.port).range(0..=65535));
                });
                theme::description(ui, "默认 23456；被占用时自动更换，0 表示随机端口。");
            });
            ui.add_space(4.0);
            if theme::primary(ui, "开始发送").clicked() {
                if self.file.trim().is_empty() {
                    self.status("请先选择文件".into(), true);
                } else {
                    self.start(Request::Send {
                        path: PathBuf::from(&self.file),
                        port: self.port,
                    });
                }
            }
        });
        if !self.share.is_empty() {
            ui.add_space(8.0);
            egui::Frame::new()
                .fill(theme::BLUE_SOFT)
                .corner_radius(8)
                .inner_margin(16)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.label(
                        RichText::new("将下面的信息告诉接收方")
                            .color(theme::BLUE)
                            .strong(),
                    );
                    ui.label(
                        RichText::new(&self.share)
                            .size(19.0)
                            .color(theme::INK)
                            .strong(),
                    );
                    if ui.button("复制连接信息").clicked() {
                        ui.ctx().copy_text(self.share.clone());
                    }
                });
        }
    }
    fn receive_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("填写接收信息").strong().size(17.0));
        ui.add_enabled_ui(self.task.is_none(), |ui| {
            ui.label("发送方地址");
            ui.add(theme::field(&mut self.address, "例如 192.168.1.100:23456"));
            egui::ComboBox::from_id_salt("contacts")
                .selected_text("从联系人选择地址")
                .show_ui(ui, |ui| {
                    for c in &self.contacts {
                        ui.selectable_value(
                            &mut self.address,
                            c.address.clone(),
                            format!("{} {}", c.note, c.address),
                        );
                    }
                });
            ui.label("保存目录");
            ui.horizontal(|ui| {
                let width = (ui.available_width() - 112.0).max(80.0);
                ui.add(theme::field(&mut self.directory, "选择文件保存位置").desired_width(width));
                if ui.button("选择目录").clicked() {
                    self.dialog_request = Some(DialogRequest::Directory);
                }
            });
        });
        ui.add_enabled_ui(self.task.is_none() || self.retry, |ui| {
            ui.label("取件码");
            ui.add(
                theme::field(&mut self.code, "4 位数字")
                    .char_limit(4)
                    .desired_width(160.0),
            );
            if theme::primary(
                ui,
                if self.retry {
                    "重新提交取件码"
                } else {
                    "开始接收"
                },
            )
            .clicked()
            {
                if self.code.len() != 4 || !self.code.bytes().all(|b| b.is_ascii_digit()) {
                    self.status("请输入 4 位数字取件码".into(), true);
                } else if self.retry {
                    if let Some(t) = &self.task {
                        let _ = t.codes.send(self.code.clone());
                    }
                    self.retry = false;
                } else if self.directory.trim().is_empty() {
                    self.status("请选择保存目录".into(), true);
                } else {
                    self.start(Request::Receive {
                        address: self.address.clone(),
                        code: self.code.clone(),
                        directory: PathBuf::from(&self.directory),
                    });
                }
            }
        });
        if !self.address.is_empty() && ui.button("将此地址保存为联系人").clicked() {
            self.contact_address = self.address.clone();
            self.original_address.clear();
            self.note.clear();
            self.page = Page::Contacts;
        }
    }
    fn contacts_page(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("已保存的联系人").strong().size(17.0));
        if self.contacts.is_empty() {
            theme::description(ui, "还没有联系人，在下方添加常用设备地址。");
        }
        let mut selected = None;
        let mut edit = None;
        egui::ScrollArea::vertical()
            .max_height(200.0)
            .show(ui, |ui| {
                for c in &self.contacts {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}  {}", c.note, c.address));
                        if ui.small_button("接收").clicked() {
                            selected = Some(c.address.clone());
                        }
                        if ui.small_button("编辑").clicked() {
                            edit = Some(c.clone());
                        }
                        if ui.small_button("删除").clicked() {
                            self.delete_address = Some(c.address.clone());
                        }
                    });
                }
            });
        if let Some(address) = selected {
            self.address = address;
            self.page = Page::Receive;
        }
        if let Some(c) = edit {
            self.original_address = c.address.clone();
            self.contact_address = c.address;
            self.note = c.note;
        }
        ui.separator();
        ui.label("地址");
        ui.add(theme::field(
            &mut self.contact_address,
            "例如 192.168.1.100:23456",
        ));
        ui.label("备注");
        ui.add(theme::field(&mut self.note, "例如 办公室电脑"));
        ui.horizontal(|ui| {
            if theme::primary(ui, "保存联系人").clicked() {
                let updated_at = timestamp();
                match contacts::upsert(
                    Contact {
                        address: self.contact_address.clone(),
                        note: self.note.clone(),
                        updated_at,
                    },
                    &self.original_address,
                ) {
                    Ok(c) => {
                        self.contacts = c;
                        self.original_address.clear();
                        self.contact_address.clear();
                        self.note.clear();
                        self.status("联系人已保存".into(), false);
                    }
                    Err(e) => self.status(format!("{e:#}"), true),
                }
            }
            if ui.button("新增 / 清空编辑").clicked() {
                self.original_address.clear();
                self.contact_address.clear();
                self.note.clear();
            }
        });
        if let Some(address) = self.delete_address.clone() {
            ui.horizontal(|ui| {
                ui.colored_label(Color32::DARK_RED, format!("确认删除 {address}？"));
                if ui.button("确认删除").clicked() {
                    let result = contacts::load().and_then(|mut c| {
                        c.retain(|c| c.address != address);
                        contacts::save(&c)?;
                        Ok(c)
                    });
                    match result {
                        Ok(c) => {
                            self.contacts = c;
                            self.status("联系人已删除".into(), false);
                        }
                        Err(e) => self.status(format!("{e:#}"), true),
                    }
                    self.delete_address = None;
                }
                if ui.button("取消").clicked() {
                    self.delete_address = None;
                }
            });
        }
    }
    fn drive_dialog(&mut self, ctx: &egui::Context) {
        #[cfg(target_os = "linux")]
        {
            use crate::file_dialog::{DialogAction, FileDialog, Filter};
            if let Some(request) = self.dialog_request.take() {
                let dialog = match request {
                    DialogRequest::File => FileDialog::open(
                        "选择待发送文件",
                        "支持任意类型的单个文件",
                        None,
                        vec![Filter::new("所有文件", &[])],
                    ),
                    DialogRequest::Directory => FileDialog::pick_folder(
                        "选择保存目录",
                        "同名文件自动另存",
                        Some(PathBuf::from(&self.directory)),
                    ),
                };
                self.dialog = Some((request, dialog));
            }
            let action = self.dialog.as_mut().map(|(r, d)| (*r, d.ui(ctx)));
            match action {
                Some((r, DialogAction::Picked(path))) => {
                    self.apply_path(r, path);
                    self.dialog = None;
                }
                Some((_, DialogAction::Cancelled)) => self.dialog = None,
                _ => (),
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = ctx;
            if let Some(request) = self.dialog_request.take() {
                let path = match request {
                    DialogRequest::File => rfd::FileDialog::new().pick_file(),
                    DialogRequest::Directory => rfd::FileDialog::new().pick_folder(),
                };
                if let Some(path) = path {
                    self.apply_path(request, path);
                }
            }
        }
    }
    fn apply_path(&mut self, request: DialogRequest, path: PathBuf) {
        match request {
            DialogRequest::File => self.file = path.display().to_string(),
            DialogRequest::Directory => self.directory = path.display().to_string(),
        }
    }
}
impl eframe::App for ChuanApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        theme::CANVAS.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.ready.store(true, Ordering::Release);
        self.poll();
        let ctx = ui.ctx().clone();
        if self.task.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.task.is_none() {
            let dropped =
                ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()));
            if let Some(path) = dropped {
                self.file = path.display().to_string();
                self.page = Page::Send;
            }
        }
        egui::Panel::left("navigation")
            .exact_size(182.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(16))
            .show(ui, |ui| self.sidebar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(theme::CANVAS).inner_margin(24))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("workspace")
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.workspace(ui));
            });
        self.drive_dialog(&ctx);
    }
}

// 保持 Go time.Time 可读取的 RFC3339 字符串；只用 std，不引入系统时间 API 依赖。
fn timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86400) as i64;
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        seconds / 3600 % 24,
        seconds / 60 % 60,
        seconds % 60
    )
}

fn install_cjk_font(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let windows = std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("C:\\Windows"));
    let candidates: Vec<(PathBuf, u32)> = if cfg!(windows) {
        vec![
            (windows.join("Fonts/msyh.ttc"), 1),
            (windows.join("Fonts/simhei.ttf"), 0),
            (windows.join("Fonts/simsun.ttc"), 0),
        ]
    } else {
        vec![
            (
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc".into(),
                2,
            ),
            (
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc".into(),
                2,
            ),
            ("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc".into(), 0),
            ("/usr/share/fonts/wqy-microhei/wqy-microhei.ttc".into(), 0),
        ]
    };
    for (path, index) in candidates {
        if let Ok(bytes) = std::fs::read(&path) {
            let mut data = egui::FontData::from_owned(bytes);
            data.index = index;
            fonts.font_data.insert("system_ui".into(), Arc::new(data));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                let list = fonts.families.entry(family).or_default();
                if cfg!(windows) {
                    list.insert(0, "system_ui".into());
                } else {
                    list.push("system_ui".into());
                }
            }
            break;
        }
    }
    ctx.set_fonts(fonts);
}
