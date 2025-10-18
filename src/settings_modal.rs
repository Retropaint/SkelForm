#[cfg(not(target_arch = "wasm32"))]
use crate::utils;
use egui::IntoAtoms;

use crate::{
    shared,
    ui::{self, EguiUi},
    Display,
};

pub const DIRECT_BONE: &str = "When clicking a bone's texture, the first untextured parent of the bone will be selected. Checkmark this to always select the textured bone directly.";

pub fn draw(shared: &mut shared::Shared, ctx: &egui::Context) {
    egui::Modal::new("test".into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: shared.config.colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., shared.config.colors.light_accent),
            ..Default::default()
        })
        .show(ctx, |modal_ui| {
            let window = shared::Vec2::new(shared.window.x / 3., shared.window.y / 3.);
            modal_ui.set_width(window.x.min(500.));
            modal_ui.set_height(window.y.min(500.));

            modal_ui.horizontal(|ui| {
                egui::Frame::new()
                    .fill(shared.config.colors.dark_accent.into())
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

                            let str_ui =
                                shared.loc("settings_modal.user_interface.heading").clone();
                            let str_rendering =
                                shared.loc("settings_modal.rendering.heading").clone();
                            let str_keyboard =
                                shared.loc("settings_modal.keyboard.heading").clone();
                            let str_misc =
                                shared.loc("settings_modal.miscellaneous.heading").clone();
                            tab!(&str_ui, shared::SettingsState::Ui);
                            tab!(&str_rendering, shared::SettingsState::Rendering);
                            tab!(&str_keyboard, shared::SettingsState::Keyboard);
                            tab!(&str_misc, shared::SettingsState::Misc);
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
                        shared::SettingsState::Misc => misc(ui, shared),
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
    let str_general = shared.loc("settings_modal.user_interface.general");
    ui.heading(str_general);
    ui.horizontal(|ui| {
        let str_ui_scale = shared.loc("settings_modal.user_interface.ui_scale");
        ui.label(str_ui_scale);
        let (edited, value, _) = ui.float_input(
            "ui_scale".to_string(),
            shared,
            shared.config.ui_scale,
            1.,
            None,
        );
        if edited {
            shared.config.ui_scale = value;
        }
    });

    ui.add_space(20.);

    colors(ui, shared);
}

fn rendering(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    ui.horizontal(|ui| {
        let str_heading = shared.loc("settings_modal.rendering.heading");
        ui.heading(str_heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = shared.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                shared.config.colors.background = crate::Config::default().colors.background;
                shared.config.colors.gridline = crate::Config::default().colors.gridline;
                shared.config.colors.center_point = crate::Config::default().colors.center_point;
                shared.config.gridline_gap = crate::Config::default().gridline_gap;
            }
        });
    });

    ui.horizontal(|ui| {
        let str_gridline_gap = shared.loc("settings_modal.rendering.gridline_gap");
        ui.label(str_gridline_gap);
        let (edited, value, _) = ui.float_input(
            "grid_gap".to_string(),
            shared,
            shared.config.gridline_gap as f32,
            1.,
            None,
        );
        if edited {
            shared.config.gridline_gap = value as i32;
        }
    });

    ui.horizontal(|ui| {
        let str_gridline_gap = shared.loc("settings_modal.rendering.gridline_front");
        ui.label(str_gridline_gap);
        ui.checkbox(&mut shared.config.gridline_front, "".into_atoms());
    });

    macro_rules! color_row {
        ($title:expr, $color:expr, $bg_color:expr) => {
            let str_color = shared
                .loc(&("settings_modal.rendering.".to_owned() + $title))
                .clone();
            let mut col = $color.clone();
            color_row(str_color, &mut col, $bg_color, ui);
            $color = col;
        };
    }

    color_row!(
        "background",
        shared.config.colors.background,
        shared.config.colors.dark_accent
    );
    color_row!(
        "gridline",
        shared.config.colors.gridline,
        shared.config.colors.main
    );
    color_row!(
        "center_point",
        shared.config.colors.center_point,
        shared.config.colors.dark_accent
    );
}

fn misc(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        let str_autosave_freq = shared.loc("settings_modal.miscellaneous.autosave_frequency");
        ui.label(str_autosave_freq);
        let (edited, value, _) = ui.float_input(
            "autosave_freq".to_string(),
            shared,
            shared.config.autosave_frequency as f32,
            1.,
            None,
        );
        if edited && value > 0. {
            shared.config.autosave_frequency = value as i32;
        }
    });
    ui.horizontal(|ui| {
        let str_exact_bone = shared.loc("settings_modal.miscellaneous.select_exact_bone");
        let str_exact_bone_desc = shared.loc("settings_modal.miscellaneous.select_exact_bone_desc");
        ui.label(&(str_exact_bone.to_owned() + crate::ICON_INFO + ":"))
            .on_hover_cursor(egui::CursorIcon::Default)
            .on_hover_text(str_exact_bone_desc);
        ui.checkbox(&mut shared.config.exact_bone_select, "".into_atoms());
    });
    ui.horizontal(|ui| {
        let str_keep_tex_idx = shared.loc("settings_modal.miscellaneous.keep_bone_tex_idx");
        let str_keep_tex_idx_desc =
            shared.loc("settings_modal.miscellaneous.keep_bone_tex_idx_desc");
        ui.label(&(str_keep_tex_idx.to_owned() + crate::ICON_INFO + ":"))
            .on_hover_cursor(egui::CursorIcon::Default)
            .on_hover_text(str_keep_tex_idx_desc);
        ui.checkbox(&mut shared.config.keep_tex_idx_on_move, "".into_atoms());
    });

    ui.add_space(20.);

    let str_startup = shared.loc("top_bar.file.startup");
    ui.heading(str_startup);
    ui.horizontal(|ui| {
        let str_skip_startup = shared.loc("settings_modal.miscellaneous.skip_startup_window");
        ui.label(str_skip_startup);
        ui.checkbox(&mut shared.config.skip_startup, "".into_atoms());
    });
    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        if shared.recent_file_paths.len() == 0 {
            ui.disable();
        }
        let str_clear_recents = shared.loc("settings_modal.miscellaneous.clear_recent_files");
        if ui.skf_button(str_clear_recents).clicked() {
            shared.recent_file_paths = vec![];
            utils::save_to_recent_files(&vec![]);
        }
    });

    ui.add_space(20.);

    let str_startup = shared.loc("settings_modal.miscellaneous.beta.heading");
    ui.heading(str_startup);
    let text =
        egui::RichText::new(shared.loc("settings_modal.miscellaneous.beta.warning")).italics();
    ui.label(text);
    ui.add_space(5.);
    ui.horizontal(|ui| {
        let str_skip_startup = shared.loc("settings_modal.miscellaneous.beta.mesh_deformation");
        ui.label(str_skip_startup);
        ui.checkbox(&mut shared.config.meshdef, "".into_atoms());
    });
}

fn colors(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    macro_rules! color_row {
        ($title:expr, $color:expr, $bg_color:expr) => {
            let str_color = shared
                .loc(&("settings_modal.user_interface.colors.".to_owned() + $title))
                .clone();
            color_row(str_color, $color, $bg_color, ui);
        };
    }

    ui.horizontal(|ui| {
        let str_colors = shared.loc("settings_modal.user_interface.colors_heading");
        ui.heading(str_colors);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = shared.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                shared.config.colors = crate::ColorConfig::default();
            }
        });
    });

    let alt_bg = shared.config.colors.main.clone();
    let main_bg = shared.config.colors.dark_accent.clone();

    // iterable color config
    #[rustfmt::skip]
    {
        color_row!("main",         &mut shared.config.colors.main,         main_bg);
        color_row!("light_accent", &mut shared.config.colors.light_accent, alt_bg);
        color_row!("dark_accent",  &mut shared.config.colors.dark_accent,  main_bg);
        color_row!("text",         &mut shared.config.colors.text,         alt_bg);
        color_row!("frameline",    &mut shared.config.colors.frameline,    main_bg);
        color_row!("gradient",     &mut shared.config.colors.gradient,     alt_bg);
        color_row!("link",         &mut shared.config.colors.link,         main_bg);
    };
}

fn color_row(title: String, color: &mut shared::Color, bg: shared::Color, ui: &mut egui::Ui) {
    let frame = egui::Frame {
        fill: bg.into(),
        ..Default::default()
    };
    ui.horizontal(|ui| {
        egui::Frame::show(frame, ui, |ui| {
            ui.label(title);

            // color picker
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut col: [f32; 3] = [
                    color.r as f32 / 255.,
                    color.g as f32 / 255.,
                    color.b as f32 / 255.,
                ];
                ui.color_edit_button_rgb(&mut col);
                *color = shared::Color {
                    r: (col[0] * 255.) as u8,
                    g: (col[1] * 255.) as u8,
                    b: (col[2] * 255.) as u8,
                    a: 255,
                };
            });
        });
    });
}

fn keyboard(ui: &mut egui::Ui, shared: &mut shared::Shared) {
    ui.horizontal(|ui| {
        let str_heading = shared.loc("settings_modal.keyboard.heading");
        ui.heading(str_heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = shared.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
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
                shared.config.colors.text,
            );
        };
    }

    macro_rules! loc {
        ($label:expr) => {
            shared
                .loc(&("settings_modal.keyboard.".to_owned() + $label))
                .clone()
        };
    }

    macro_rules! keys {
        () => {
            &mut shared.config.keys
        };
    }

    let colors = &shared.config.colors;

    // iterable key config
    #[rustfmt::skip]
    {
        key!(loc!("next_anim_frame"), keys!().next_anim_frame, colors.dark_accent);
        key!(loc!("prev_anim_frame"), keys!().prev_anim_frame, colors.main);
        key!(loc!("zoom_camera_in"),  keys!().zoom_in_camera,  colors.dark_accent);
        key!(loc!("zoom_camera_out"), keys!().zoom_out_camera, colors.main);
        key!(loc!("undo"),            keys!().undo,            colors.dark_accent);
        key!(loc!("redo"),            keys!().redo,            colors.main);
        key!(loc!("save"),            keys!().save,            colors.dark_accent);
        key!(loc!("open"),            keys!().open,            colors.main);
        key!(loc!("cancel"),          keys!().cancel,          colors.dark_accent);
        key!(loc!("copy"),            keys!().copy,            colors.main);
        key!(loc!("paste"),           keys!().paste,           colors.dark_accent);
    };
}

fn key(
    name: String,
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
                ui.label(name.clone());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let button_str = if *changing_key == name {
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

    if *changing_key == name && *last_pressed != None {
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
