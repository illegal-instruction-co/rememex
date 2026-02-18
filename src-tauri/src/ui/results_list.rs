use eframe::egui;

use crate::state::SearchResult;

use super::style;

pub enum ResultAction {
    None,
    Select(usize),
    Open(usize),
}

const RESULT_H: f32 = 76.0;
const MAX_VISIBLE: usize = 6;

fn get_file_icon(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" | "txt" | "md" => "\u{1F4C4}",
        "rs" | "ts" | "js" | "py" | "go" | "java" | "c" | "cpp" | "cs" => "\u{1F4BB}",
        "json" | "yaml" | "yml" | "toml" => "\u{2699}",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => "\u{1F5BC}",
        _ => "\u{1F4C1}",
    }
}

fn get_filename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

pub fn show(
    ui: &mut egui::Ui,
    results: &[SearchResult],
    selected_index: usize,
) -> ResultAction {
    let mut action = ResultAction::None;

    if results.is_empty() {
        return action;
    }

    egui::ScrollArea::vertical()
        .max_height(MAX_VISIBLE as f32 * RESULT_H)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            for (idx, result) in results.iter().enumerate() {
                let is_selected = idx == selected_index;

                let bg = if is_selected {
                    style::FILL_SELECTED
                } else {
                    egui::Color32::TRANSPARENT
                };

                let frame = egui::Frame::new()
                    .fill(bg)
                    .corner_radius(egui::CornerRadius::same(4u8))
                    .inner_margin(egui::Margin { left: 12, right: 12, top: 10, bottom: 10 });

                let frame_resp = frame.show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(get_file_icon(&result.path))
                                .size(14.0)
                                .color(style::TEXT_SECONDARY),
                        );
                        ui.vertical(|ui| {
                            // Nom du fichier + score
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(get_filename(&result.path))
                                        .size(13.0)
                                        .color(style::TEXT_PRIMARY)
                                        .strong(),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let score_text =
                                            format!("{}%", result.score.round() as i32);
                                        let color = style::score_color(result.score);
                                        ui.label(
                                            egui::RichText::new(score_text)
                                                .size(10.0)
                                                .color(color),
                                        );
                                    },
                                );
                            });

                            // Snippet — 160 chars (etait 120)
                            if !result.snippet.is_empty() {
                                let snippet: String =
                                    result.snippet.chars().take(160).collect();
                                ui.label(
                                    egui::RichText::new(snippet)
                                        .size(11.0)
                                        .color(style::TEXT_SECONDARY),
                                );
                            }

                            // Chemin complet
                            ui.label(
                                egui::RichText::new(result.path.as_str())
                                    .size(10.0)
                                    .color(style::TEXT_DISABLED)
                                    .monospace(),
                            );
                        });
                    });
                });

                // Interaction sur le rect complet du frame
                let response = frame_resp.response.interact(egui::Sense::click());

                // Pill accent — dessine sur le bord gauche du rect frame (fix coordonnees)
                // Utilise ui.painter() apres le frame pour rester dans le clip rect du panel
                if is_selected {
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            response.rect.left_top(),
                            egui::vec2(3.0, response.rect.height()),
                        ),
                        egui::CornerRadius::same(1u8),
                        style::ACCENT,
                    );
                }

                // Simple clic = selectionner ; double-clic = ouvrir
                // Le hover ne selectionne plus automatiquement (etait source de confusion)
                if response.double_clicked() {
                    action = ResultAction::Open(idx);
                } else if response.clicked() {
                    action = ResultAction::Select(idx);
                }

                // Scroll pour maintenir l'element selectionne visible
                if is_selected {
                    response.scroll_to_me(Some(egui::Align::Center));
                }
            }
        });

    action
}
