use super::*;

impl ChuanApp {
    pub(super) fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            egui::Frame::new()
                .fill(theme::BLUE)
                .corner_radius(9)
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("传")
                            .size(23.0)
                            .color(Color32::WHITE)
                            .strong(),
                    );
                });
            ui.vertical(|ui| {
                ui.label(RichText::new("局域网传输").strong().size(16.0));
                ui.label(
                    RichText::new("简单 · 直接 · 高效")
                        .size(12.0)
                        .color(theme::MUTED),
                );
            });
        });
        ui.add_space(26.0);
        ui.label(RichText::new("文件传输").color(theme::MUTED).size(12.0));
        for (page, label) in [
            (Page::Send, "发送文件"),
            (Page::Receive, "接收文件"),
            (Page::Contacts, "常用联系人"),
        ] {
            let active = self.page == page;
            let button = egui::Button::new(
                RichText::new(label)
                    .color(if active { theme::BLUE } else { theme::INK })
                    .strong(),
            )
            .fill(if active {
                theme::BLUE_SOFT
            } else {
                Color32::TRANSPARENT
            })
            .stroke(egui::Stroke::NONE)
            .corner_radius(7)
            .min_size(egui::vec2(ui.available_width(), 43.0));
            if ui.add_enabled(self.task.is_none(), button).clicked() {
                self.page = page;
            }
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.label(
                RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .size(12.0)
                    .color(theme::MUTED),
            );
            ui.label(
                RichText::new("同一局域网内直连")
                    .size(12.0)
                    .color(theme::MUTED),
            );
        });
    }

    pub(super) fn workspace(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        let (title, subtitle) = match self.page {
            Page::Send => ("发送文件", "选择文件，分享地址和取件码，即可开始传输。"),
            Page::Receive => (
                "接收文件",
                "输入对方的连接信息。同名文件自动另存，不会覆盖。",
            ),
            Page::Contacts => ("常用联系人", "保存常用设备，下次接收时直接选用地址。"),
        };
        ui.heading(RichText::new(title).strong().color(theme::INK));
        theme::description(ui, subtitle);
        ui.add_space(8.0);
        theme::card(ui, |ui| match self.page {
            Page::Send => self.send_page(ui),
            Page::Receive => self.receive_page(ui),
            Page::Contacts => self.contacts_page(ui),
        });
        ui.add_space(6.0);
        theme::card(ui, |ui| self.transfer_status(ui));
        ui.add_space(4.0);
        ui.collapsing("运行日志与诊断", |ui| {
            egui::ScrollArea::vertical()
                .id_salt("transfer_logs")
                .max_height(150.0)
                .show(ui, |ui| {
                    if self.logs.is_empty() {
                        theme::description(ui, "暂无传输记录");
                    }
                    for line in &self.logs {
                        ui.label(line);
                    }
                    if let Some(path) = &self.log_path {
                        theme::description(ui, &format!("启动日志：{}", path.display()));
                    }
                });
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "安全提示：文件以明文传输，请仅在可信局域网使用。连接失败时，请检查发送方防火墙。",
            )
            .size(13.0)
            .color(theme::MUTED),
        );
    }

    fn transfer_status(&mut self, ui: &mut egui::Ui) {
        let title = if self.error {
            "需要处理"
        } else if self.finished {
            "传输完成"
        } else if self.task.is_some() {
            "传输状态"
        } else {
            "准备就绪"
        };
        let (color, background) = if self.error {
            (theme::RED, Color32::from_rgb(255, 240, 238))
        } else if self.task.is_some() {
            (theme::BLUE, theme::BLUE_SOFT)
        } else {
            (theme::GREEN, Color32::from_rgb(235, 247, 240))
        };
        ui.horizontal(|ui| {
            egui::Frame::new()
                .fill(background)
                .corner_radius(5)
                .inner_margin(egui::Margin::symmetric(9, 5))
                .show(ui, |ui| {
                    ui.label(RichText::new(title).color(color).strong().size(13.0));
                });
            if let Some(task) = &self.task
                && ui.button("取消传输").clicked()
            {
                task.control.cancel();
            }
        });
        ui.label(RichText::new(&self.status).color(if self.error {
            theme::RED
        } else {
            theme::INK
        }));
        if self.task.is_some() || self.finished {
            let fraction = if self.finished {
                1.0
            } else if self.total > 0 {
                self.done as f32 / self.total as f32
            } else {
                0.0
            };
            ui.add(
                egui::ProgressBar::new(fraction)
                    .fill(theme::BLUE)
                    .desired_height(10.0),
            );
            let eta = if self.finished {
                "已完成".into()
            } else if self.speed > 0.0 {
                format!(
                    "{:.0} 秒",
                    self.total.saturating_sub(self.done) as f64 / self.speed
                )
            } else {
                "等待传输".into()
            };
            ui.columns(3, |columns| {
                theme::description(&mut columns[0], "传输进度");
                columns[0].label(format!(
                    "{:.0}% · {} / {}",
                    fraction * 100.0,
                    format_bytes(self.done as f64),
                    format_bytes(self.total as f64)
                ));
                theme::description(&mut columns[1], "平均速度");
                columns[1].label(format!("{}/s", format_bytes(self.speed)));
                theme::description(&mut columns[2], "预计剩余");
                columns[2].label(eta);
            });
        }
    }
}
