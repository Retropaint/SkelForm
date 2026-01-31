use crate::utils;
use egui::IntoAtoms;

use crate::{shared, ui::EguiUi, Display};

pub fn draw(
    shared_ui: &mut crate::Ui,
    config: &mut crate::Config,
    camera: &crate::Camera,
    events: &mut crate::EventState,
    ctx: &egui::Context,
) {
    let mut col: egui::Color32 = config.colors.main.into();
    if shared_ui.translucent_settings {
        col = col.gamma_multiply(0.5);
    }
    let modal = egui::Modal::new("test".into()).frame(egui::Frame {
        corner_radius: 0.into(),
        fill: col,
        inner_margin: egui::Margin::same(5),
        stroke: egui::Stroke::new(1., config.colors.light_accent),
        ..Default::default()
    });
    modal.show(ctx, |modal_ui| {
        let window = shared::Vec2::new(camera.window.x / 3., camera.window.y / 3.);
        modal_ui.set_width(window.x.min(500.));
        modal_ui.set_height(window.y.min(500.));

        modal_ui.horizontal(|ui| {
            let mut col: egui::Color32 = config.colors.dark_accent.into();
            if shared_ui.translucent_settings {
                col = col.gamma_multiply(0.5);
            }
            let frame = egui::Frame::new()
                .fill(col)
                .inner_margin(egui::Margin::same(5));
            frame.show(ui, |ui| {
                ui.set_width(window.x.min(100.));
                ui.set_height(window.y.min(475.));
                let width = ui.min_rect().width();
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    let mut is_hovered = false;

                    #[rustfmt::skip]
                    macro_rules! tab {
                        ($name:expr, $state:expr) => {
                            settings_button($name, $state, ui, shared_ui, &config, width, &mut is_hovered)
                        };
                    }

                    let str_ui_raw = "settings_modal.user_interface.heading";
                    let str_misc_raw = "settings_modal.miscellaneous.heading";
                    let str_ui = shared_ui.loc(str_ui_raw).clone();
                    let str_anim = shared_ui.loc("settings_modal.animation.heading").clone();
                    let str_rendering = shared_ui.loc("settings_modal.rendering.heading").clone();
                    let str_keyboard = shared_ui.loc("settings_modal.keyboard.heading").clone();
                    let str_misc = shared_ui.loc(str_misc_raw).clone();
                    tab!(str_ui, shared::SettingsState::Ui);
                    tab!(str_anim, shared::SettingsState::Animation);
                    tab!(str_rendering, shared::SettingsState::Rendering);
                    tab!(str_keyboard, shared::SettingsState::Keyboard);
                    tab!(str_misc, shared::SettingsState::Misc);

                    if shared_ui.settings_state != shared::SettingsState::Rendering {
                        shared_ui.translucent_settings = false;
                    }

                    if !is_hovered {
                        shared_ui.hovering_setting = None;
                    }
                });
            });
            egui::Frame::new().show(ui, |ui| {
                ui.set_width(window.x.min(400.));
                ui.set_height(window.y.min(475.));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let layout = egui::Layout::top_down(egui::Align::Min);
                    ui.with_layout(layout, |ui| match shared_ui.settings_state {
                        shared::SettingsState::Ui => user_interface(ui, shared_ui, config),
                        shared::SettingsState::Animation => animation(ui, shared_ui, config),
                        shared::SettingsState::Rendering => rendering(ui, shared_ui, config, camera),
                        shared::SettingsState::Keyboard => keyboard(ui, shared_ui, config),
                        shared::SettingsState::Misc => misc(ui, shared_ui, config),
                    });
                });
            })
        });

        modal_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.skf_button("Apply").clicked() {
                events.apply_settings();
                shared_ui.settings_modal = false;
                shared_ui.translucent_settings = false;
            }
            if ui.skf_button("Cancel").clicked() {
                events.reset_config();
                shared_ui.settings_modal = false;
                shared_ui.translucent_settings = false;
            }
        })
    });
}

fn settings_button(
    name: String,
    state: shared::SettingsState,
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    config: &crate::Config,
    width: f32,
    is_hovered: &mut bool,
) {
    let mut col = config.colors.dark_accent;
    if shared_ui.hovering_setting == Some(state.clone()) {
        col += shared::Color::new(20, 20, 20, 0);
    }
    if shared_ui.settings_state == state.clone() {
        col += shared::Color::new(20, 20, 20, 0);
    }
    let button = egui::Frame::new()
        .fill(col.into())
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_width(width);
                ui.set_height(21.);
                ui.add_space(5.);
                ui.label(name);
            });
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if button.contains_pointer() {
        shared_ui.hovering_setting = Some(state.clone());
        *is_hovered = true;
    }
    if button.clicked() {
        shared_ui.settings_state = state;
    }
}

fn user_interface(ui: &mut egui::Ui, shared_ui: &mut crate::Ui, config: &mut crate::Config) {
    let str_general = &shared_ui.loc("settings_modal.user_interface.general");
    ui.heading(str_general);
    ui.horizontal(|ui| {
        let str_ui_scale = &shared_ui.loc("settings_modal.user_interface.ui_scale");
        ui.label(str_ui_scale);
        let scale = config.ui_scale;
        let (edited, value, _) = ui.float_input("ui_scale".to_string(), shared_ui, scale, 1., None);
        if edited {
            config.ui_scale = value;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let str = shared_ui.loc("settings_modal.user_interface.ui_slider");
            if ui.skf_button(&str).clicked() {
                crate::toggleElement(true, "ui-slider".to_string())
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label("Layout:");
        let combo_box = egui::ComboBox::new("layout", "").selected_text(config.layout.to_string());
        combo_box.show_ui(ui, |ui| {
            ui.selectable_value(&mut config.layout, shared::UiLayout::Split, "Split");
            ui.selectable_value(&mut config.layout, shared::UiLayout::Right, "Right");
            ui.selectable_value(&mut config.layout, shared::UiLayout::Left, "Left");
        });
    });

    ui.add_space(20.);
    colors(ui, config, shared_ui);
    ui.add_space(20.);
}

fn animation(ui: &mut egui::Ui, shared_ui: &mut crate::Ui, config: &mut crate::Config) {
    let str_heading = &shared_ui.loc("settings_modal.animation.heading");
    ui.heading(str_heading);
    ui.horizontal(|ui| {
        let str_edit = &shared_ui.loc("settings_modal.animation.edit_while_playing");
        let str_edit_desc = &shared_ui.loc("settings_modal.animation.edit_while_playing_desc");
        ui.label(str_edit).on_hover_text(str_edit_desc);
        ui.checkbox(&mut config.edit_while_playing, "".into_atoms());
    });
}

fn rendering(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    config: &mut crate::Config,
    camera: &crate::Camera,
) {
    ui.horizontal(|ui| {
        let str_heading = &shared_ui.loc("settings_modal.rendering.heading");
        ui.heading(str_heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // translucent toggle
            let label = ui
                .heading("👁")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(shared_ui.loc("settings_modal.rendering.translucent_desc"));

            if label.clicked() {
                shared_ui.translucent_settings = !shared_ui.translucent_settings;
            }
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                config.colors.background = crate::Config::default().colors.background;
                config.colors.gridline = crate::Config::default().colors.gridline;
                config.colors.center_point = crate::Config::default().colors.center_point;
                config.gridline_gap = crate::Config::default().gridline_gap;
            }
        });
    });

    ui.horizontal(|ui| {
        let str_heading = &shared_ui.loc("settings_modal.rendering.pixel_mag");
        ui.label(str_heading);
        let id = "pixelmag".to_string();
        let (edited, value, _) =
            ui.float_input(id, shared_ui, config.pixel_magnification as f32, 1., None);
        if edited {
            config.pixel_magnification = (value as i32).max(1);
        }
        let window = camera.window / config.pixel_magnification as f32;
        ui.label("= ".to_owned() + &window.x.to_string() + ", " + &window.y.to_string());
    });

    ui.horizontal(|ui| {
        let str_gridline_gap = &shared_ui.loc("settings_modal.rendering.gridline_gap");
        ui.label(str_gridline_gap);
        let gap = config.gridline_gap as f32;
        let (edited, value, _) = ui.float_input("grid_gap".to_string(), shared_ui, gap, 1., None);
        if edited {
            config.gridline_gap = value as i32;
        }
    });

    ui.horizontal(|ui| {
        let str_gridline_gap = &shared_ui.loc("settings_modal.rendering.gridline_front");
        ui.label(str_gridline_gap);
        ui.checkbox(&mut config.gridline_front, "".into_atoms());
    });

    macro_rules! color_row {
        ($title:expr, $color:expr, $bg_color:expr) => {
            let str_color = shared_ui
                .loc(&("settings_modal.rendering.".to_owned() + $title))
                .clone();
            let mut col = $color.clone();
            color_row(str_color, &mut col, $bg_color, ui);
            $color = col;
        };
    }

    let dark_accent = &config.colors.dark_accent;
    let main = &config.colors.main;
    color_row!("background", config.colors.background, *dark_accent);
    color_row!("gridline", config.colors.gridline, *main);
    color_row!("center_point", config.colors.center_point, *dark_accent);
}

fn misc(ui: &mut egui::Ui, shared_ui: &mut crate::Ui, config: &mut crate::Config) {
    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        let str_autosave_freq = &shared_ui.loc("settings_modal.miscellaneous.autosave_frequency");
        ui.label(str_autosave_freq);
        let id = "autosave_freq".to_string();
        let auto_freq = config.autosave_frequency as f32;
        let (edited, value, _) = ui.float_input(id, shared_ui, auto_freq, 1., None);
        if edited && value > 0. {
            config.autosave_frequency = value as i32;
        }
    });
    ui.horizontal(|ui| {
        let str_exact_bone = &shared_ui.loc("settings_modal.miscellaneous.select_exact_bone");
        let str_exact_bone_desc =
            &shared_ui.loc("settings_modal.miscellaneous.select_exact_bone_desc");
        ui.label(str_exact_bone.to_owned())
            .on_hover_cursor(egui::CursorIcon::Default)
            .on_hover_text(str_exact_bone_desc);
        ui.checkbox(&mut config.exact_bone_select, "".into_atoms());
    });
    ui.horizontal(|ui| {
        let str_keep_tex_str = &shared_ui.loc("settings_modal.miscellaneous.keep_tex_str");
        let str_keep_tex_str_desc =
            &shared_ui.loc("settings_modal.miscellaneous.keep_tex_str_desc");
        ui.label(&(str_keep_tex_str.to_owned()))
            .on_hover_cursor(egui::CursorIcon::Default)
            .on_hover_text(str_keep_tex_str_desc);
        ui.checkbox(&mut config.keep_tex_str, "".into_atoms());
    });

    ui.add_space(20.);

    let str_startup = &shared_ui.loc("top_bar.file.startup");
    ui.heading(str_startup);
    ui.horizontal(|ui| {
        let str_skip_startup = &shared_ui.loc("settings_modal.miscellaneous.skip_startup_window");
        ui.label(str_skip_startup);
        ui.checkbox(&mut config.skip_startup, "".into_atoms());
    });
    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        if shared_ui.recent_file_paths.len() == 0 {
            ui.disable();
        }
        let str_clear_recents = &shared_ui.loc("settings_modal.miscellaneous.clear_recent_files");
        if ui.skf_button(str_clear_recents).clicked() {
            shared_ui.recent_file_paths = vec![];
            utils::save_to_recent_files(&vec![]);
        }
    });

    ui.add_space(20.);

    let str_startup = &shared_ui.loc("settings_modal.miscellaneous.beta.heading");
    ui.heading(str_startup);
    //let text = shared.ui.loc("settings_modal.miscellaneous.beta.warning");
    let mut text = shared_ui.loc("settings_modal.miscellaneous.beta.nothing");
    text = text.replace("$version", env!("CARGO_PKG_VERSION"));
    let mut cache = egui_commonmark::CommonMarkCache::default();
    let str = utils::markdown(text, shared_ui.local_doc_url.to_string());
    egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
    ui.add_space(20.);

    if ui.button("Intentionally Crash").clicked() {
        panic!();
    }
}

fn colors(ui: &mut egui::Ui, config: &mut crate::Config, shared_ui: &crate::Ui) {
    macro_rules! color_row {
        ($title:expr, $color:expr, $bg_color:expr) => {
            let str_color = shared_ui
                .loc(&("settings_modal.user_interface.colors.".to_owned() + $title))
                .clone();
            color_row(str_color, $color, $bg_color, ui);
        };
    }

    ui.horizontal(|ui| {
        let str_colors = &shared_ui.loc("settings_modal.user_interface.colors_heading");
        ui.heading(str_colors);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                config.colors = crate::ColorConfig::default();
            }
        });
    });

    let alt_bg = config.colors.main.clone();
    let main_bg = config.colors.dark_accent.clone();

    // iterable color config
    #[rustfmt::skip]
    {
        color_row!("main",         &mut config.colors.main,         main_bg);
        color_row!("light_accent", &mut config.colors.light_accent, alt_bg);
        color_row!("dark_accent",  &mut config.colors.dark_accent,  main_bg);
        color_row!("text",         &mut config.colors.text,         alt_bg);
        color_row!("frameline",    &mut config.colors.frameline,    main_bg);
        color_row!("gradient",     &mut config.colors.gradient,     alt_bg);
        color_row!("link",         &mut config.colors.link,         main_bg);
        color_row!("warning_text", &mut config.colors.warning_text, alt_bg);
        color_row!("inverse_kinematics", &mut config.colors.inverse_kinematics, main_bg);
        color_row!("meshdef", &mut config.colors.meshdef, alt_bg);
        color_row!("texture", &mut config.colors.texture, main_bg);
        color_row!("ik_target", &mut config.colors.ik_target, alt_bg);
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

fn keyboard(ui: &mut egui::Ui, shared_ui: &mut crate::Ui, config: &mut crate::Config) {
    ui.horizontal(|ui| {
        let str_heading = &shared_ui.loc("settings_modal.keyboard.heading");
        ui.heading(str_heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                config.keys = crate::KeyboardConfig::default();
            }
        });
    });

    macro_rules! key {
        ($label:expr, $field:expr, $color:expr) => {
            key(
                $label,
                &mut $field,
                ui,
                &mut shared_ui.changing_key,
                &shared_ui.last_pressed,
                $color,
                config.colors.text,
            );
        };
    }

    macro_rules! loc {
        ($label:expr) => {
            shared_ui
                .loc(&("settings_modal.keyboard.".to_owned() + $label))
                .clone()
        };
    }

    macro_rules! keys {
        () => {
            &mut config.keys
        };
    }

    let colors = &config.colors;

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
            let text = egui::RichText::new(modifier_name($modifier)).color(text_color);
            $ui.selectable_value(&mut $field, $modifier, text);
        };
    }

    ui.horizontal(|ui| {
        egui::Frame::new().fill(color.into()).show(ui, |ui| {
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
        });
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
