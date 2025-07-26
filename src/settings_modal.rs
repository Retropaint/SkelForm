use crate::{config_path, shared, ui};
use std::{fs, io::Write};

pub fn draw(shared: &mut shared::Shared, ctx: &egui::Context) {
    egui::Modal::new("test".into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: shared.config.ui_colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., shared.config.ui_colors.light_accent),
            ..Default::default()
        })
        .show(ctx, |modal_ui| {
            modal_ui.set_width(500.);
            modal_ui.set_height(500.);

            modal_ui.horizontal(|ui| {
                egui::Frame::new()
                    .fill(shared.config.ui_colors.dark_accent.into())
                    .inner_margin(egui::Margin::same(5))
                    .show(ui, |ui| {
                        ui.set_width(100.);
                        ui.set_height(475.);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            macro_rules! tab {
                                ($name:expr, $state:expr) => {
                                    if ui::selection_button(
                                        $name,
                                        shared.ui.settings_state == $state,
                                        ui,
                                    )
                                    .clicked()
                                    {
                                        shared.ui.settings_state = $state;
                                    }
                                };
                            }

                            tab!("General", shared::SettingsState::General);
                            tab!("Keyboard", shared::SettingsState::Keyboard);
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
            $ui.add(egui::DragValue::new(&mut $field).speed(0.1));
            // let (edited, val, _) = ui::float_input($id.to_string(), shared, $ui, $field.into(), 1.);
            // if edited {
            //     $field = val as u8;
            //     crate::utils::save_config(&shared.config);
            // }
        };
    }

    macro_rules! color {
        ($title:expr, $color:expr, $ui:expr) => {
            $ui.horizontal(|ui| {
                ui.label($title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    input!($title.to_string() + "_b", $color.b, ui);
                    input!($title.to_string() + "_g", $color.g, ui);
                    input!($title.to_string() + "_r", $color.r, ui);
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
        color!("Light Accent",    shared.config.ui_colors.light_accent,    ui);
        color!("Dark Accent",    shared.config.ui_colors.dark_accent,    ui);
        color!("Text",      shared.config.ui_colors.text,      ui);
        color!("Frameline", shared.config.ui_colors.frameline, ui);
        color!("Gradient",  shared.config.ui_colors.gradient,  ui);
    };
}

fn keyboard(ui: &mut egui::Ui) {
    ui.heading("Keyboard");
}
