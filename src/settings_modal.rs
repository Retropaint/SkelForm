use crate::{shared, ui};

pub fn draw(shared: &mut shared::Shared, ctx: &egui::Context) {
    egui::Modal::new("test".into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: ui::COLOR_MAIN,
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., ui::COLOR_ACCENT),
            ..Default::default()
        })
        .show(ctx, |modal_ui| {
            modal_ui.set_width(500.);
            modal_ui.set_height(500.);

            modal_ui.horizontal(|ui| {
                egui::Frame::new().fill(ui::COLOR_ACCENT).show(ui, |ui| {
                    ui.set_width(100.);
                    ui.set_height(500.);
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
                    ui.set_height(500.);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| match shared
                        .ui
                        .settings_state
                    {
                        shared::SettingsState::General => general(ui),
                        shared::SettingsState::Keyboard => keyboard(ui),
                        _ => {}
                    });
                })
            })
        });
}

fn general(ui: &mut egui::Ui) {
    ui.label("general");
}

fn keyboard(ui: &mut egui::Ui) {
    ui.label("keyboard");
}
