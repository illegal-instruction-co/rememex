use eframe::egui;

use crate::i18n::{self, Language};
use crate::state::IndexingProgress;

use super::style;

pub fn show(
    ui: &mut egui::Ui,
    status: &str,
    is_indexing: bool,
    index_progress: Option<&IndexingProgress>,
    active_container: &str,
    folder_count: usize,
    result_count: usize,
    locale: Language,
) {
    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_premultiplied(15, 15, 15, 150))
        .inner_margin(egui::Margin { left: 12, right: 12, top: 0, bottom: 0 });

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());

        // Layout horizontal simple — pas de bottom_up (etait la cause des boutons invisibles)
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 24.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                // Nom du container actif
                ui.label(
                    egui::RichText::new(active_container)
                        .size(11.0)
                        .color(style::ACCENT)
                        .strong(),
                );

                ui.label(
                    egui::RichText::new("\u{2502}")
                        .size(11.0)
                        .color(style::STROKE_SUBTLE),
                );

                // Statut ou nombre de dossiers/resultats
                if !status.is_empty() {
                    if is_indexing {
                        // Prefixe pourcentage si progression disponible
                        let pct_prefix = if let Some(p) = index_progress {
                            if p.total > 0 {
                                let pct =
                                    (p.current as f32 / p.total as f32 * 100.0) as i32;
                                format!("{}% \u{00B7} ", pct)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{}\u{23F3} {}",
                                pct_prefix, status
                            ))
                            .size(11.0)
                            .color(style::TEXT_TERTIARY),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(status)
                                .size(11.0)
                                .color(style::TEXT_TERTIARY),
                        );
                    }
                } else {
                    ui.label(
                        egui::RichText::new(i18n::t(
                            locale,
                            "status_indexed_folders",
                            &[("count", &folder_count.to_string())],
                        ))
                        .size(11.0)
                        .color(style::TEXT_TERTIARY),
                    );

                    if result_count > 0 {
                        ui.label(
                            egui::RichText::new(format!(
                                "\u{00B7} {}",
                                i18n::t(
                                    locale,
                                    "status_result_count",
                                    &[("count", &result_count.to_string())]
                                )
                            ))
                            .size(11.0)
                            .color(style::TEXT_TERTIARY),
                        );
                    }
                }
            },
        );
    });
}
