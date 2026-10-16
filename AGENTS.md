# 传：开发约定

本项目为 Rust 2024 + egui/eframe 0.36 原生 GUI。旧 Go 工程保留在 `~/chuan`，本目录不再保留其副本，不要修改原工程。

- 仅保留 GUI，入口为 `src/main.rs` + `src/app.rs`；业务核心 `chuan_lib` 禁止依赖 egui。不提供独立 CLI 或 GUI 内无头参数分发，传输参数通过界面设置。
- 所有用户可见文案使用中文；错误进入页面状态行，不反复弹窗。
- GUI 必须通过 CentralPanel/Panel 显式绘制不透明背景，clear_color 与固定浅色主题保持一致；仅 set_visuals 不会覆盖 eframe 的深色默认清屏色。统一配色在 `src/app/theme.rs`，外壳在 `src/app/shell.rs`，不得回退成无面板的顶层控件堆叠。
- 网络与传输在后台线程执行；取消/退出关闭连接，清理不完整文件，不覆盖已有文件。
- 保持 Go 版 TCP 协议兼容，文件名可含空格，元数据从最后一个空格切分。
- 联系人路径与 JSON 格式兼容 Go 版，`updated_at` 使用 RFC3339；损坏数据保留，不静默覆盖。
- 图形兼容措施参考 `~/temp/rsou`：X11 + glow、EGL 优先/GLX 兜底补丁、Win7 ole32 补丁、release fat LTO。
- Linux 使用内置 egui 文件选择器，不依赖 GTK、Portal、zenity；Windows 使用 rfd；对话框帧末驱动。
- 中文字体运行时加载，不嵌入。保留启动诊断与 Linux 启动器，覆盖 main 之前的链接错误。
- Win7 使用 `scripts/build-win.sh`：nightly build-std + x86_64/i686-win7-windows-gnu；GUI 须审计 PE 导入。
- Linux ARM64 发布在 Debian buster/glibc 2.28 容器编译并审计符号，不能以新宿主系统构建替代。
- 临时验证二进制放 `build/temp/`；Cargo 输出 `target/`，发布输出 `dist/`。
- 沿用构建与手动集成检查，不添加自动化测试；GUI 端到端由用户手动验证，不自动启动窗口或截图。
- 验证：`cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo build --bins`。涉及 Windows 再跑 `cargo check --bins --target x86_64-pc-windows-gnu`，发布必须 Win7 双架构构建与审计。
- 提交信息使用中文。
