# 本次改造验证记录

## 已完成

- Rust + egui/eframe **0.36.2**（与参考项目当前 Cargo.lock 一致）。
- 修复用户截图中的黑底低对比度问题：显式 CentralPanel 背景、统一不透明浅色清屏色、固定主题；界面整理为左侧导航、白色表单卡片和独立传输状态区。未自动启动 GUI 或截图，新版实际视觉效果仍由用户手动验收。
- `cargo fmt --all -- --check`、`cargo clippy --locked --all-targets -- -D warnings`。
- Linux 本机 GUI debug、release 构建，Windows x64 GNU 交叉检查。
- 本机 amd64 deb 打包，检查启动器、GUI、桌面文件、图标和显式图形依赖；仅打包 GUI。
- 旧 Go → Rust、Rust → 旧 Go、Rust → Rust：中文空格文件名，8 MiB + 17 字节，SHA256 一致；另检查空文件。
- 同名文件另存且原文件不变；路径穿越、Windows 盘符/保留文件名、负数大小均拒绝。
- 端口被占用自动回退；发送端与接收端同一连接取件码重试成功。
- 取消监听、取消接收及时关闭连接；中途断线、取消均清理不完整文件。
- 磁盘不足拒绝接收；启动器记录进入 main 前的缺失二进制错误。

一次性核心集成驱动保留在忽略的 `build/temp/`，未添加自动化测试套件。旧 Go 验证二进制与新工程中的旧 Go 源码副本已清理，原始 Go 工程仍完整保留在 `~/chuan`。

以上协议互通记录来自移除命令行入口之前的手动检查；传输核心保持不变。当前仅保留 GUI，已删除独立 CLI、无头参数处理和相关发布产物，后续 GUI 验证由用户手动完成。

## 发布兼容性边界

- Windows 7 x64/x86 使用专用 win7 target、nightly build-std、fat LTO，GUI 通过已知禁用导入及架构审计；**未在 Windows 7 真机启动 GUI**。产物见 `dist/`。
- 本机 Linux x86_64 环境较新，实际二进制与本地 deb **要求 glibc 2.39**，仅用于当前机器，不是 UOS/glibc 2.28 发布包。
- ARM64/glibc 2.28 构建、符号审计和 deb 发布已配置在 `.github/workflows/linux-arm64.yml`，本次未运行远端 CI。正式旧 Linux 发布须由该流程产出。
- 按项目约定没有自动启动 GUI、截图或端到端操作；Linux UOS/麒麟旧 Mesa、Windows 7 真机界面、字体、原生文件选择器、实际局域网防火墙仍需人工验收。

## 手动启动

```bash
~/temp/chuan/target/release/chuan
```

旧 Go 工程与原二进制仍保留在 `~/chuan`，目录结构和源码已恢复。新 Rust GUI 工程位于 `~/temp/chuan`，其源码入口及 `build.sh` 均使用 Rust。
