# 传 · 局域网文件传输

Rust + egui/eframe 0.36 原生 GUI，采用与 rsou 相同的图形栈和兼容方案。不需要浏览器、WebView 或 Go 运行时。

## 使用

直接运行 **chuan** 打开中文图形界面：

- **发送文件**：选择或拖入单个文件，点击“开始发送”，将显示的实际地址、端口和 4 位取件码告诉对方。端口占用时自动更换。
- **接收文件**：选择保存目录，填写对方地址与取件码。省略端口时使用 23456；取件码错误可在同一连接重试，最多 3 次。
- **联系人**：新增、编辑、删除或直接选用旧版联系人，接收页也可快速保存地址。
- 后台传输显示进度、平均速度、剩余时间，支持取消；同名文件自动另存，中断清理不完整文件，接收前检查磁盘空间。

双方需位于同一可信局域网，发送端需放行界面显示的 TCP 端口。**取件码不是加密，文件明文传输，不适合公网或不可信网络。** 当前仅传单个文件，不支持目录、续传或内容校验哈希。

程序仅提供图形界面，文件、地址、取件码、保存目录和端口均在界面中设置；不提供独立命令行程序或无头传输参数。Windows 发布程序不显示控制台。

保持旧 Go 版协议兼容：CODE → OK/ERR → FILE <文件名> <大小> → READY → 原始文件字节 → EOF → ACK → BYE。

## 平台兼容

- **Linux**：X11 + glow/OpenGL，不使用 wgpu，纯 Wayland 需 XWayland。沿用 rsou 的 glutin-winit 补丁，EGL 优先、GLX 兜底，规避 UOS 20 旧 Mesa 的 GLXBadContextTag。内置文件选择器不依赖 GTK、Portal 或 zenity。
- **Linux ARM64 发布**：CI 使用 Debian buster 编译，并审计 glibc 需求 ≤ 2.28。提供裸二进制和 deb，显式声明通过 dlopen 加载的图形库。本机 cargo build 只保证宿主系统，不代表 glibc 2.28 发布兼容性。
- **Windows 7+ x64/x86**：rfd 原生文件选择器；专用 win7 target + nightly build-std；windows-link 补丁将 CoTaskMemFree 链接到 ole32.dll；release fat LTO 清除未使用的高版本 API。脚本审计 GUI 的架构和已知 Win7 禁用导入。旧机器仍需可用的 OpenGL 驱动。
- **中文字体**：运行时加载微软雅黑/黑体/宋体、Noto CJK 或文泉驿，不嵌入字体。Linux 缺中文时安装 fonts-noto-cjk 或 fonts-wqy-microhei。

兼容方案来自 rsou；Windows 7、UOS/麒麟真机 GUI 测试仍需人工完成，交叉编译与导入表审计不能替代真机验证。

## 构建

要求支持 Rust 2024 的 stable 工具链。

```bash
./build.sh                       # 本机 release GUI
cargo run --bin chuan            # 开发运行
cargo deb --no-build             # 先执行 ./build.sh；需 cargo-deb
./scripts/build-win.sh           # Linux 交叉构建 Win7 x64/x86，输出 dist/
```

Windows 构建需要 nightly + rust-src，以及 x86_64/i686 的 Linux 原生 MinGW GCC、windres、objdump。不使用 gnullvm、zig 或 MSYS2 链接器。产物为 chuan-win-x64.exe / chuan-win-x32.exe。

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo build --bins
cargo check --bins --target x86_64-pc-windows-gnu
```

沿用手动集成验证：通过界面互传空文件、中文空格文件名与大文件并核对 SHA256；检查错误取件码、取消、同名文件、路径穿越防护与旧 Go 版双向互通。GUI 由用户手动检查，不自动启动或截图。

本次已完成的验证与尚未覆盖的平台见 [VALIDATION.md](VALIDATION.md)。新工程位于 `~/temp/chuan`，启动新 GUI 使用 `~/temp/chuan/target/release/chuan`；`~/chuan` 保留原 Go 工程与原目录结构。

## 数据与诊断

联系人 JSON 兼容旧格式（address、note、RFC3339 updated_at）：
- Linux：$XDG_CONFIG_HOME/chuan/contacts.json，默认 ~/.config/chuan/contacts.json
- Windows：%APPDATA%/chuan/contacts.json

迁移无需复制联系人；损坏数据保留原文件并提示，保存前备份到 contacts.json.bak。

启动日志：Linux 优先 $XDG_STATE_HOME/chuan/startup.log，其次 ~/.cache/chuan/startup.log；Windows 位于 %LOCALAPPDATA%/chuan/startup.log（不可写时退回临时目录）。日志记录图形初始化、panic、Linux 致命信号和启动超时，保留上次日志 startup.log.1。deb 的 /usr/bin/chuan 为启动器，实际程序在 /usr/lib/chuan/chuan，启动器还能捕获进入 main 前的链接错误。

## 源码布局

- src/transfer.rs：32KB 流式 TCP、取消、重试、安全接收
- src/contacts.rs：兼容联系人 JSON
- src/app.rs：发送 / 接收 / 联系人中文 GUI
- src/file_dialog.rs + src/filebrowser.rs：Linux 内置选择器
- src/startup.rs + assets/chuan-launcher.sh：启动诊断
- scripts/、vendor/、.github/workflows/：兼容补丁和构建发布

原 Go 工程及旧规格保留在 `~/chuan`，新工程中不再存放副本。

MIT。
