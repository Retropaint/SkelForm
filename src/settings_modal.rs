use crate::{config_path, shared, ui};
use std::{fs, io::Write};

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
                        shared::SettingsState::General => general(ui, shared),
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

fn general(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    macro_rules! input {
        ($id:expr, $field:expr, $ui:expr) => {
            let (edited, val, _) = ui::float_input($id.to_string(), shared, $ui, $field.into(), 1.);
            if edited {
                $field = val as u8;
                crate::utils::save_config(&shared.config);
            }
        };
    }

    macro_rules! color {
        ($title:expr, $color:expr, $ui:expr) => {
            $ui.horizontal(|ui| {
                ui.label($title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    input!($title.to_string() + "_r", $color.r, ui);
                    input!($title.to_string() + "_g", $color.g, ui);
                    input!($title.to_string() + "_b", $color.b, ui);
                });
            });
        };
    }

    ui.horizontal(|ui| {
        ui.heading("Color");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Reset").clicked() {
                shared.config.ui_colors = crate::ColorConfig::default();
                crate::utils::save_config(&shared.config);
            }
        });
    });

    #[rustfmt::skip]
    {
        color!("Main",      shared.config.ui_colors.main,      ui);
        color!("Accent",    shared.config.ui_colors.accent,    ui);
        color!("Border",    shared.config.ui_colors.border,    ui);
        color!("Text",      shared.config.ui_colors.text,      ui);
        color!("Frameline", shared.config.ui_colors.frameline, ui);
        color!("Gradient",  shared.config.ui_colors.gradient,  ui);
    };
}

fn keyboard(ui: &mut egui::Ui) {
    ui.label("keyboard");
}
