use eframe::egui;

/// Windows 11 Fluent Design inspired color palette
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(96, 205, 255);
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::WHITE;
pub const TEXT_SECONDARY: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(200, 200, 200, 200);
pub const TEXT_TERTIARY: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(139, 139, 139, 139);
pub const TEXT_DISABLED: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(92, 92, 92, 92);
#[allow(dead_code)]
pub const FILL_LAYER: egui::Color32 = egui::Color32::from_rgba_premultiplied(8, 8, 8, 8);
// ~12% blanc — fond des controles inactifs (etait 8%)
pub const FILL_CONTROL: egui::Color32 = egui::Color32::from_rgba_premultiplied(30, 30, 30, 30);
// ~23% blanc — fond hover (etait 18%)
pub const FILL_CONTROL_HOVER: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(58, 58, 58, 58);
// ~20% blanc — fond resultat selectionne
pub const FILL_SELECTED: egui::Color32 = egui::Color32::from_rgba_premultiplied(50, 50, 50, 50);
// ~25% blanc — bordure visible sur fond sombre
pub const STROKE_SUBTLE: egui::Color32 = egui::Color32::from_rgba_premultiplied(64, 64, 64, 64);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(255, 100, 100);
pub const SCORE_GREEN: egui::Color32 = egui::Color32::from_rgb(74, 222, 128);
pub const SCORE_YELLOW: egui::Color32 = egui::Color32::from_rgb(250, 204, 21);
pub const SCORE_ORANGE: egui::Color32 = egui::Color32::from_rgb(251, 146, 60);

pub fn score_color(score: f32) -> egui::Color32 {
    if score > 80.0 {
        SCORE_GREEN
    } else if score > 65.0 {
        SCORE_YELLOW
    } else {
        SCORE_ORANGE
    }
}

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(TEXT_PRIMARY);
    style.visuals.panel_fill = egui::Color32::TRANSPARENT;
    // from_rgba_premultiplied coherent avec tout le reste du style
    style.visuals.window_fill = egui::Color32::from_rgba_premultiplied(44, 44, 44, 245);
    style.visuals.window_stroke = egui::Stroke::new(1.0, STROKE_SUBTLE);
    style.visuals.widgets.noninteractive.bg_fill = egui::Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_fill = FILL_CONTROL;
    style.visuals.widgets.hovered.bg_fill = FILL_CONTROL_HOVER;
    style.visuals.widgets.active.bg_fill =
        egui::Color32::from_rgba_premultiplied(255, 255, 255, 30);
    style.visuals.selection.bg_fill =
        egui::Color32::from_rgba_premultiplied(96, 205, 255, 40);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, ACCENT);

    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);

    // Coins arrondis WinUI — 8px pour fenetre compacte Spotlight (etait 12)
    style.visuals.window_corner_radius = egui::CornerRadius::same(8u8);
    style.visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(4u8);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(4u8);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(4u8);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(4u8);

    ctx.set_style(style);
}
