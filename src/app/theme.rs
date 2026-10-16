//! 固定浅色主题。主题颜色与面板填充必须同时设置，不能依赖 eframe 的清屏色。
use eframe::egui::{self, Color32, RichText, Stroke};

pub const CANVAS: Color32 = Color32::from_rgb(244, 247, 251);
pub const INK: Color32 = Color32::from_rgb(30, 43, 62);
pub const MUTED: Color32 = Color32::from_rgb(89, 106, 128);
pub const BORDER: Color32 = Color32::from_rgb(214, 223, 234);
pub const BLUE: Color32 = Color32::from_rgb(38, 96, 190);
pub const BLUE_SOFT: Color32 = Color32::from_rgb(233, 241, 255);
pub const GREEN: Color32 = Color32::from_rgb(29, 108, 72);
pub const RED: Color32 = Color32::from_rgb(170, 46, 46);

pub fn configure(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = CANVAS;
    visuals.window_fill = Color32::WHITE;
    visuals.extreme_bg_color = Color32::from_rgb(229, 235, 243);
    visuals.faint_bg_color = CANVAS;
    visuals.text_edit_bg_color = Some(Color32::WHITE);
    visuals.override_text_color = Some(INK);
    visuals.weak_text_color = Some(MUTED);
    visuals.hyperlink_color = BLUE;
    visuals.selection.bg_fill = BLUE;
    visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.error_fg_color = RED;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, INK);
    for widget in [&mut visuals.widgets.inactive, &mut visuals.widgets.open] {
        widget.bg_fill = Color32::WHITE;
        widget.weak_bg_fill = Color32::WHITE;
        widget.bg_stroke = Stroke::new(1.0, BORDER);
        widget.fg_stroke = Stroke::new(1.0, INK);
        widget.corner_radius = egui::CornerRadius::same(6);
    }
    for widget in [&mut visuals.widgets.hovered, &mut visuals.widgets.active] {
        widget.bg_fill = BLUE_SOFT;
        widget.weak_bg_fill = BLUE_SOFT;
        widget.bg_stroke = Stroke::new(1.0, BLUE);
        widget.fg_stroke = Stroke::new(1.0, BLUE);
        widget.corner_radius = egui::CornerRadius::same(6);
    }
    // 同时覆盖两套样式，避免系统主题变化时弹出层与主界面采用不同配色。
    ctx.all_styles_mut(|style| {
        style.visuals = visuals.clone();
        style.spacing.item_spacing = egui::vec2(10.0, 12.0);
        style.spacing.button_padding = egui::vec2(14.0, 9.0);
        style.spacing.interact_size = egui::vec2(36.0, 36.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(25.0));
    });
}

pub fn card<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10)
        .inner_margin(20)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            content(ui)
        })
        .inner
}

pub fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(Color32::WHITE).strong())
            .fill(BLUE)
            .stroke(Stroke::NONE)
            .corner_radius(6)
            .min_size(egui::vec2(140.0, 40.0)),
    )
}

pub fn field<'a>(text: &'a mut String, hint: &'a str) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(text)
        .hint_text(hint)
        .desired_width(f32::INFINITY)
        .margin(egui::vec2(10.0, 9.0))
}

pub fn description(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).color(MUTED).size(14.0));
}
