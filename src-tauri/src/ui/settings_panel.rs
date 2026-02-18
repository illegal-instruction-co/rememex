use eframe::egui;

use crate::i18n::{self, Language};
use crate::state::ContainerListItem;

use super::style;

pub enum SettingsAction {
    None,
    Close,
    SwitchContainer(String),
    CreateContainer,
    DeleteContainer,
    ClearIndex,
    ReindexAll,
    AddFolder,
    CycleLocale,
}

/// Panneau de reglages flottant (overlay Area), positionne sous le bouton gear.
/// Ferme par clic en dehors ou par Echap.
pub fn show(
    ctx: &egui::Context,
    containers: &[ContainerListItem],
    active_container: &str,
    is_indexing: bool,
    locale: Language,
) -> SettingsAction {
    let mut action = SettingsAction::None;
    let mut close = false;

    let area = egui::Area::new(egui::Id::new("settings_panel"))
        .order(egui::Order::Foreground)
        // Coin superieur droit de la fenetre, decale sous le bouton gear
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 56.0));

    let area_resp = area.show(ctx, |ui| {
        let frame = egui::Frame::new()
            .fill(egui::Color32::from_rgba_premultiplied(30, 30, 30, 240))
            .corner_radius(egui::CornerRadius::same(8u8))
            .stroke(egui::Stroke::new(1.0, style::STROKE_SUBTLE))
            .inner_margin(egui::Margin::same(12i8));

        frame.show(ui, |ui| {
            ui.set_min_width(280.0);
            ui.set_max_width(280.0);

            // ─── Section Conteneurs ───
            ui.label(
                egui::RichText::new(
                    i18n::ts(locale, "settings_containers_section").to_uppercase(),
                )
                .size(10.0)
                .color(style::TEXT_TERTIARY)
                .strong(),
            );
            ui.add_space(4.0);

            for container in containers {
                let is_active = container.name == active_container;
                let text_color = if is_active {
                    style::TEXT_PRIMARY
                } else {
                    style::TEXT_SECONDARY
                };
                let suffix = if is_active { " [active]" } else { "" };

                let resp = ui.add(
                    egui::Button::new(
                        egui::RichText::new(format!("\u{25A0} {}{}", container.name, suffix))
                            .size(12.0)
                            .color(text_color),
                    )
                    .fill(if is_active {
                        style::FILL_CONTROL_HOVER
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .frame(false),
                );
                if resp.clicked() && !is_active {
                    action = SettingsAction::SwitchContainer(container.name.clone());
                    close = true;
                }
            }

            // + Nouveau conteneur
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(format!(
                            "+ {}",
                            i18n::ts(locale, "sidebar_create")
                        ))
                        .size(12.0)
                        .color(style::ACCENT),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .frame(false),
                )
                .clicked()
            {
                action = SettingsAction::CreateContainer;
                close = true;
            }

            ui.add_space(8.0);
            ui.add(egui::Separator::default());
            ui.add_space(4.0);

            // ─── Section Dossiers indexes ───
            ui.label(
                egui::RichText::new(
                    i18n::ts(locale, "settings_folders_section").to_uppercase(),
                )
                .size(10.0)
                .color(style::TEXT_TERTIARY)
                .strong(),
            );
            ui.add_space(4.0);

            if let Some(container) = containers.iter().find(|c| c.name == active_container) {
                if container.indexed_paths.is_empty() {
                    ui.label(
                        egui::RichText::new(i18n::ts(locale, "sidebar_no_folders"))
                            .size(11.0)
                            .color(style::TEXT_DISABLED)
                            .italics(),
                    );
                } else {
                    for path in &container.indexed_paths {
                        let short: String = path
                            .rsplit(['/', '\\'])
                            .take(2)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                            .collect::<Vec<_>>()
                            .join("/");
                        ui.label(
                            egui::RichText::new(format!("\u{1F4C2} {}", short))
                                .size(11.0)
                                .color(style::TEXT_SECONDARY),
                        )
                        .on_hover_text(path.as_str());
                    }
                }
            }

            // Bouton ajout dossier
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(i18n::ts(locale, "settings_add_folder"))
                            .size(12.0)
                            .color(style::TEXT_SECONDARY),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .frame(false),
                )
                .clicked()
            {
                action = SettingsAction::AddFolder;
                close = true;
            }

            ui.add_space(8.0);
            ui.add(egui::Separator::default());
            ui.add_space(4.0);

            // ─── Actions ───
            let rebuild_btn = ui.add_enabled(
                !is_indexing,
                egui::Button::new(
                    egui::RichText::new(format!(
                        "\u{21BB} {}",
                        i18n::ts(locale, "sidebar_rebuild")
                    ))
                    .size(12.0)
                    .color(style::TEXT_SECONDARY),
                )
                .fill(egui::Color32::TRANSPARENT)
                .frame(false),
            );
            if rebuild_btn.clicked() {
                action = SettingsAction::ReindexAll;
                close = true;
            }

            let clear_btn = ui.add_enabled(
                !is_indexing,
                egui::Button::new(
                    egui::RichText::new(format!(
                        "\u{1F5D1} {}",
                        i18n::ts(locale, "sidebar_clear")
                    ))
                    .size(12.0)
                    .color(style::DANGER),
                )
                .fill(egui::Color32::TRANSPARENT)
                .frame(false),
            );
            if clear_btn.clicked() {
                action = SettingsAction::ClearIndex;
                close = true;
            }

            // Supprimer le conteneur (uniquement si ce n'est pas "Default")
            if active_container != "Default" {
                let delete_btn = ui.add_enabled(
                    !is_indexing,
                    egui::Button::new(
                        egui::RichText::new(format!(
                            "\u{1F5D1} {}",
                            i18n::ts(locale, "sidebar_delete")
                        ))
                        .size(12.0)
                        .color(style::DANGER),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .frame(false),
                );
                if delete_btn.clicked() {
                    action = SettingsAction::DeleteContainer;
                    close = true;
                }
            }

            ui.add_space(8.0);
            ui.add(egui::Separator::default());
            ui.add_space(4.0);

            // ─── Locale + version ───
            if ui
                .add(
                    egui::Button::new(
                        egui::RichText::new(format!(
                            "\u{1F310} {}",
                            locale.code().to_uppercase()
                        ))
                        .size(12.0)
                        .color(style::TEXT_TERTIARY),
                    )
                    .fill(egui::Color32::TRANSPARENT)
                    .frame(false),
                )
                .on_hover_text(locale.label())
                .clicked()
            {
                action = SettingsAction::CycleLocale;
                close = true;
            }

            ui.label(
                egui::RichText::new("v2.0.0")
                    .size(10.0)
                    .color(style::TEXT_DISABLED),
            );
        });
    });

    // Fermeture sur clic en dehors du panneau
    if area_resp.response.clicked_elsewhere() {
        close = true;
    }

    // Fermeture sur Echap (gere aussi dans handle_keyboard, mais doublon sans danger)
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }

    if close {
        if matches!(action, SettingsAction::None) {
            return SettingsAction::Close;
        }
    }

    action
}
