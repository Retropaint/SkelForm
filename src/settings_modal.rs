use crate::{shared, ui, ui::EguiUi, Display};

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
            let window = shared::Vec2::new(shared.window.x / 3., shared.window.y / 3.);
            modal_ui.set_width(window.x.min(500.));
            modal_ui.set_height(window.y.min(500.));

            modal_ui.horizontal(|ui| {
                egui::Frame::new()
                    .fill(shared.config.ui_colors.dark_accent.into())
                    .inner_margin(egui::Margin::same(5))
                    .show(ui, |ui| {
                        ui.set_width(window.x.min(100.));
                        ui.set_height(window.y.min(475.));
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            macro_rules! tab {
                                ($name:expr, $state:expr) => {
                                    let is_state = shared.ui.settings_state == $state;
                                    if ui::selection_button($name, is_state, ui).clicked() {
                                        shared.ui.settings_state = $state;
                                    }
                                };
                            }

                            tab!("User Interface", shared::SettingsState::Ui);
                            tab!("Rendering", shared::SettingsState::Rendering);
                            tab!("Keyboard", shared::SettingsState::Keyboard);
                        });
                    });
                egui::Frame::new().show(ui, |ui| {
                    ui.set_width(window.x.min(400.));
                    ui.set_height(window.y.min(475.));
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| match shared
                        .ui
                        .settings_state
                    {
                        shared::SettingsState::Ui => user_interface(ui, shared),
                        shared::SettingsState::Rendering => rendering(ui, shared),
                        shared::SettingsState::Keyboard => keyboard(ui, shared),
                    });
                })
            });

            modal_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.skf_button("Apply").clicked() {
                    shared.ui.scale = shared.config.ui_scale;
                    shared.gridline_gap = shared.config.gridline_gap;
                    crate::utils::save_config(&shared.config);
                    shared.ui.set_state(shared::UiState::SettingsModal, false);
                }
                if ui.skf_button("Cancel").clicked() {
                    crate::utils::import_config(shared);
                    shared.ui.set_state(shared::UiState::SettingsModal, false);
                }
            })
        });
}

fn user_interface(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    ui.heading("General");
    ui.horizontal(|ui| {
        ui.label("UI Scale:");
        let (edited, value, _) =
            ui.float_input("ui_scale".to_string(), shared, shared.config.ui_scale, 1.);
        if edited {
            shared.config.ui_scale = value;
        }
    });

    ui.add_space(20.);

    colors(ui, shared);
}

fn rendering(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.skf_button("Default").clicked() {
                shared.config.ui_colors.background = crate::Config::default().ui_colors.background;
                shared.config.ui_colors.gridline = crate::Config::default().ui_colors.gridline;
                shared.config.gridline_gap = crate::Config::default().gridline_gap;
            }
        });
    });

    ui.horizontal(|ui| {
        ui.label("Gridline gap (pixels):");
        let (edited, value, _) = ui.float_input(
            "grid_gap".to_string(),
            shared,
            shared.config.gridline_gap as f32,
            1.,
        );
        if edited {
            shared.config.gridline_gap = value as i32;
        }
    });

    macro_rules! color_row {
        ($title:expr, $color:expr, $bg_color:expr) => {
            let mut col = $color.clone();
            color_row($title, &mut col, $bg_color, ui, shared);
            $color = col;
        };
    }

    color_row!(
        "Background",
        shared.config.ui_colors.background,
        shared.config.ui_colors.main
    );
    color_row!(
        "Gridline",
        shared.config.ui_colors.gridline,
        shared.config.ui_colors.main
    );
}

fn colors(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    macro_rules! color_row {
        ($title:expr, $color:expr, $bg_color:expr) => {
            let mut col = $color.clone();
            color_row($title, &mut col, $bg_color, ui, shared);
            $color = col;
        };
    }

    ui.horizontal(|ui| {
        ui.heading("Color");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.skf_button("Default").clicked() {
                shared.config.ui_colors = crate::ColorConfig::default();
            }
        });
    });

    macro_rules! col {
        () => {
            &mut shared.config.ui_colors
        };
    }

    // iterable color config
    #[rustfmt::skip]
    {
        color_row!("Main",         col!().main,         col!().dark_accent);
        color_row!("Light Accent", col!().light_accent, col!().main       );
        color_row!("Dark Accent",  col!().dark_accent,  col!().dark_accent);
        color_row!("Text",         col!().text,         col!().main       );
        color_row!("Frameline",    col!().frameline,    col!().dark_accent);
        color_row!("Gradient",     col!().gradient,     col!().main       );
    };
}

fn color_row(
    title: &str,
    color: &mut shared::Color,
    bg: shared::Color,
    ui: &mut egui::Ui,
    shared: &mut shared::Shared,
) {
    macro_rules! drag_value {
        ($id:expr, $field:expr, $ui:expr) => {
            $ui.add(egui::DragValue::new(&mut $field).speed(0.1));
        };
    }
    ui.horizontal(|ui| {
        egui::Frame::show(
            egui::Frame {
                fill: bg.into(),
                ..Default::default()
            },
            ui,
            |ui| {
                ui.label(title);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // hex input
                    let color32: egui::Color32 = (*color).into();
                    let (edited, val, _) = ui.text_input(
                        "test".to_string() + "_hex",
                        shared,
                        color32.to_hex().to_string()[..7].to_string(),
                        Some(ui::TextInputOptions {
                            size: shared::Vec2::new(60., 20.),
                            ..Default::default()
                        }),
                    );
                    if edited {
                        if let Ok(data) = egui::Color32::from_hex(&(val + "ff")) {
                            color.r = data.r();
                            color.g = data.g();
                            color.b = data.b();
                        }
                    }

                    drag_value!("test".to_string() + "_b", color.b, ui);
                    drag_value!("test".to_string() + "_g", color.g, ui);
                    drag_value!("test".to_string() + "_r", color.r, ui);
                });
            },
        );
    });
}

fn keyboard(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    ui.horizontal(|ui| {
        ui.heading("Keyboard");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.skf_button("Default").clicked() {
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

    // iterable key config
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
                        "...".to_string()
                    } else {
                        field.logical_key.display()
                    };

                    let button_rich_text = egui::RichText::new(button_str).color(text_color);
                    if ui
                        .add_sized([80., 20.], egui::Button::new(button_rich_text))
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
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
                            dd_mod!(ui, egui::Modifiers::CTRL, field.modifiers);
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
        egui::Modifiers::CTRL => "Ctrl/Control",
        egui::Modifiers::ALT => "Alt/Option",
        egui::Modifiers::SHIFT => "Shift",
        egui::Modifiers::NONE => "None",
        _ => "??",
    }
    .to_string()
}
