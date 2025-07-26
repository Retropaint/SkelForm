use crate::{shared, ui};

pub fn draw(shared: &mut shared::Shared, ctx: &egui::Context) {
    egui::Modal::new("test".into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: shared.config.ui_colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., shared.config.ui_colors.accent),
            ..Default::default()
        })
        .show(ctx, |modal_ui| {
            modal_ui.set_width(500.);
            modal_ui.set_height(500.);

            modal_ui.horizontal(|ui| {
                egui::Frame::new()
                    .fill(shared.config.ui_colors.accent.into())
                    .show(ui, |ui| {
                        ui.set_width(100.);
                        ui.set_height(475.);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            if ui.button("General").clicked() {
                                shared.ui.settings_state = shared::SettingsState::General;
                            }
                            if ui.button("Keyboard").clicked() {
                                shared.ui.settings_state = shared::SettingsState::Keyboard;
                            }
                        });
                    });
                egui::Frame::new().show(ui, |ui| {
                    ui.set_width(400.);
                    ui.set_height(475.);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| match shared
                        .ui
                        .settings_state
                    {
                        shared::SettingsState::General => general(ui),
                        shared::SettingsState::Keyboard => keyboard(ui),
                        _ => {}
                    });
                })
            });

            modal_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui::button("Apply", ui).clicked() {
                    shared.ui.set_state(shared::UiState::SettingsModal, false);
                }
            })
        });
}

fn general(ui: &mut egui::Ui) {
    ui.label("general");
}

fn keyboard(ui: &mut egui::Ui) {
    ui.label("keyboard");
}
