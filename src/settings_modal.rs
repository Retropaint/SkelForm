use serde::Serialize;

use crate::{shared, ui};
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
            #[allow(unused_mut)]
            let mut scale = shared::Vec2::new(1., 1.);

            #[cfg(target_arch = "wasm32")]
            {
                scale = shared::Vec2::new(0.5 / shared.ui.scale, 0.5 / shared.ui.scale);
                scale.x *= 2.;
            }

            modal_ui.set_width(500. * scale.x);
            modal_ui.set_height(500. * scale.y);

            modal_ui.horizontal(|ui| {
                egui::Frame::new()
                    .fill(shared.config.ui_colors.dark_accent.into())
                    .inner_margin(egui::Margin::same(5))
                    .show(ui, |ui| {
                        ui.set_width(100. * scale.x);
                        ui.set_height(475. * scale.y);
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
                    ui.set_width(400. * scale.x);
                    ui.set_height(475. * scale.y);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| match shared
                        .ui
                        .settings_state
                    {
                        shared::SettingsState::General => general(ui, shared),
                        shared::SettingsState::Keyboard => keyboard(ui, shared),
                    });
                })
            });

            modal_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui::button("Apply", ui).clicked() {
                    crate::utils::save_config(&shared.config);
                    shared.ui.set_state(shared::UiState::SettingsModal, false);
                }
                if ui::button("Cancel", ui).clicked() {
                    crate::utils::import_config(shared);
                    shared.ui.set_state(shared::UiState::SettingsModal, false);
                }
            })
        });
}

fn general(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    macro_rules! drag_value {
        ($id:expr, $field:expr, $ui:expr) => {
            $ui.add(egui::DragValue::new(&mut $field).speed(0.1));
        };
    }

    macro_rules! color {
        ($title:expr, $color:expr, $bg_color:expr, $ui:expr) => {
            $ui.horizontal(|ui| {
                egui::Frame::show(
                    egui::Frame {
                        fill: $bg_color.into(),
                        ..Default::default()
                    },
                    ui,
                    |ui| {
                        ui.label($title);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            // hex input
                            let color32: egui::Color32 = $color.into();
                            let (edited, val, _) = ui::text_input(
                                $title.to_string() + "_hex",
                                shared,
                                ui,
                                color32.to_hex().to_string()[..7].to_string(),
                                Some(ui::TextInputOptions {
                                    size: shared::Vec2::new(60., 20.),
                                    ..Default::default()
                                }),
                            );
                            if edited {
                                $color = egui::Color32::from_hex(&(val + "ff"))
                                    .unwrap_or_default()
                                    .into();
                            }

                            drag_value!($title.to_string() + "_b", $color.b, ui);
                            drag_value!($title.to_string() + "_g", $color.g, ui);
                            drag_value!($title.to_string() + "_r", $color.r, ui);
                        });
                    },
                );
            });
        };
    }

    ui.horizontal(|ui| {
        ui.heading("Color");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui::button("Reset", ui).clicked() {
                shared.config.ui_colors = crate::ColorConfig::default();
            }
        });
    });

    #[rustfmt::skip]
    {
        color!("Main",         shared.config.ui_colors.main,         shared.config.ui_colors.dark_accent, ui);
        color!("Light Accent", shared.config.ui_colors.light_accent, shared.config.ui_colors.main,        ui);
        color!("Dark Accent",  shared.config.ui_colors.dark_accent,  shared.config.ui_colors.dark_accent, ui);
        color!("Text",         shared.config.ui_colors.text,         shared.config.ui_colors.main,        ui);
        color!("Frameline",    shared.config.ui_colors.frameline,    shared.config.ui_colors.dark_accent, ui);
        color!("Gradient",     shared.config.ui_colors.gradient,     shared.config.ui_colors.main,        ui);
    };
}

fn keyboard(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    ui.horizontal(|ui| {
        ui.heading("Keyboard");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui::button("Reset", ui).clicked() {
                shared.config.keys = crate::KeyboardConfig::default();
            }
        });
    });

    macro_rules! key {
        ($label:expr, $field:expr, $color:expr) => {
            key(
                $label,
                &mut $field,
                ui,
                &mut shared.ui.changing_key,
                &shared.input.last_pressed,
                $color,
                shared.config.ui_colors.text,
            );
        };
    }

    let keys = &mut shared.config.keys;
    let colors = &shared.config.ui_colors;

    #[rustfmt::skip]
    {
        key!("Next Animation Frame",     keys.next_anim_frame, colors.dark_accent);
        key!("Previous Animation Frame", keys.prev_anim_frame, colors.main);
        key!("Zoom Camera In",           keys.zoom_in_camera,  colors.dark_accent);
        key!("Zoom Camera Out",          keys.zoom_out_camera, colors.main);
        key!("Undo",                     keys.undo,            colors.dark_accent);
        key!("Redo",                     keys.redo,            colors.main);
        key!("Save",                     keys.save,            colors.dark_accent);
        key!("Open",                     keys.open,            colors.main);
        key!("Cancel",                   keys.cancel,          colors.dark_accent);
    };
}

fn key(
    name: &str,
    field: &mut egui::KeyboardShortcut,
    ui: &mut egui::Ui,
    changing_key: &mut String,
    last_pressed: &Option<egui::Key>,
    color: shared::Color,
    text_color: shared::Color,
) {
    macro_rules! dd_mod {
        ($ui:expr, $modifier:expr, $field:expr) => {
            $ui.selectable_value(
                &mut $field,
                $modifier,
                egui::RichText::new(modifier_name($modifier)).color(text_color),
            );
        };
    }

    ui.horizontal(|ui| {
        egui::Frame::show(
            egui::Frame {
                fill: color.into(),
                ..Default::default()
            },
            ui,
            |ui| {
                ui.label(name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button_str = if changing_key == name {
                        "..."
                    } else {
                        field.logical_key.name()
                    };

                    if ui
                        .add_sized(
                            [120., 20.],
                            egui::Button::new(egui::RichText::new(button_str).color(text_color)),
                        )
                        .clicked()
                    {
                        *changing_key = name.to_string();
                    }

                    egui::ComboBox::new(name.to_string() + "mod", "")
                        .selected_text(
                            egui::RichText::new(modifier_name(field.modifiers)).color(text_color),
                        )
                        .show_ui(ui, |ui| {
                            dd_mod!(ui, egui::Modifiers::NONE, field.modifiers);
                            dd_mod!(ui, egui::Modifiers::COMMAND, field.modifiers);
                            dd_mod!(ui, egui::Modifiers::ALT, field.modifiers);
                            dd_mod!(ui, egui::Modifiers::SHIFT, field.modifiers);
                        })
                        .response;

                    // use shift-equivalent keys if the modifier is shift
                    if field.modifiers == egui::Modifiers::SHIFT {
                        field.logical_key = match field.logical_key {
                            egui::Key::Equals => egui::Key::Plus,
                            egui::Key::Slash => egui::Key::Questionmark,
                            egui::Key::Semicolon => egui::Key::Colon,
                            _ => field.logical_key,
                        };
                    }
                });
            },
        );
    });

    if changing_key == name && *last_pressed != None {
        field.logical_key = last_pressed.unwrap();
        *changing_key = "".to_string();
    }
}

fn modifier_name(modifier: egui::Modifiers) -> String {
    match modifier {
        egui::Modifiers::COMMAND => "Ctrl/Cmd",
        egui::Modifiers::ALT => "Alt/Option",
        egui::Modifiers::SHIFT => "Shift",
        egui::Modifiers::NONE => "None",
        _ => "??",
    }
    .to_string()
}
