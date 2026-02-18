use eframe::egui;

use super::style;

/// Affiche la barre de recherche.
///
/// - `focus_pending` : mis a `false` apres avoir applique le focus une seule fois.
/// - `on_settings`  : mis a `true` si le bouton reglages (gear) est clique.
pub fn show(
    ui: &mut egui::Ui,
    query: &mut String,
    placeholder: &str,
    focus_pending: &mut bool,
    on_settings: &mut bool,
) {
    ui.add_space(8.0);

    let frame = egui::Frame::new()
        .fill(egui::Color32::from_rgba_premultiplied(255, 255, 255, 10))
        .corner_radius(egui::CornerRadius::same(8u8))
        .inner_margin(egui::Margin { left: 12, right: 8, top: 10, bottom: 10 })
        .stroke(egui::Stroke::new(
            1.0,
            egui::Color32::from_rgba_premultiplied(255, 255, 255, 15),
        ));

    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            // Icone loupe
            ui.label(
                egui::RichText::new("\u{1F50D}")
                    .size(14.0)
                    .color(style::TEXT_TERTIARY),
            );

            // Zone de texte — prend toute la largeur sauf le bouton gear
            let gear_w = 28.0;
            let text_w = ui.available_width() - gear_w - ui.spacing().item_spacing.x;
            let response = ui.add_sized(
                egui::vec2(text_w, 24.0),
                egui::TextEdit::singleline(query)
                    .hint_text(
                        egui::RichText::new(placeholder)
                            .color(style::TEXT_DISABLED)
                            .size(15.0),
                    )
                    .font(egui::TextStyle::Body)
                    .text_color(style::TEXT_PRIMARY)
                    .frame(false)
                    .desired_width(f32::INFINITY),
            );

            // Focus applique une seule fois (fix : evite de voler le focus chaque frame)
            if *focus_pending {
                response.request_focus();
                *focus_pending = false;
            }

            // Bouton reglages (gear)
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("\u{2699}")
                            .size(14.0)
                            .color(style::TEXT_TERTIARY),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .frame(false),
                )
                .clicked()
            {
                *on_settings = true;
            }
        });
    });

    ui.add_space(4.0);
}
