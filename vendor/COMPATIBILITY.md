# 兼容补丁来源

以下源码与兼容措施取自本机参考项目 `~/temp/rsou`，不是新增框架：

- `glutin-winit` 0.5.0：Linux EGL 优先、GLX 兜底。补丁说明见 `glutin-winit/RSOU_PATCH.md`。上游 MIT 许可证保留在该目录。
- `windows-link` 0.2.1：将 windows-sys 0.61 声明的 CoTaskMemFree 从 combase.dll 改为 ole32.dll，支持 Win7；其余链接宏不变。上游 MIT/Apache-2.0。
- `src/startup.rs`、`src/file_dialog.rs`、`src/filebrowser.rs`、Linux 启动器、Win7 构建/审计脚本和 ARM64 工作流以 rsou 对应实现为基础，移除资料库专用过滤器并改为 chuan 名称。Windows 补充 LOCALAPPDATA 日志位置。

更新这些依赖时应同时复查补丁、Win7 PE 导入与 UOS/Mesa 真机启动，不要直接删除 patch 后宣称兼容。
