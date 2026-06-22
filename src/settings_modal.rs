use egui::IntoAtoms;

use crate::{shared, ui::EguiUi, Config, Display};

pub fn draw(
    shared_ui: &mut crate::Ui,
    config: &crate::Config,
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
        let window = shared::Vec2::new(camera.window.x / 4., camera.window.y / 3.);
        modal_ui.set_width(window.x.min(375.));
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

                    macro_rules! tab {
                        ($name:expr, $state:expr) => {
                            settings_button(
                                $name,
                                $state,
                                ui,
                                shared_ui,
                                &config,
                                width,
                                &mut is_hovered,
                            )
                        };
                    }

                    let str_ui_raw = "settings_modal.user_interface.heading";
                    let str_misc_raw = "settings_modal.miscellaneous.heading";
                    let str_ui = shared_ui.loc(str_ui_raw).clone();
                    let str_edit = shared_ui.loc("settings_modal.editing.heading").clone();
                    let str_rendering = shared_ui.loc("settings_modal.rendering.heading").clone();
                    let str_keyboard = shared_ui.loc("settings_modal.keyboard.heading").clone();
                    let str_colors = shared_ui.loc("settings_modal.colors.heading").clone();
                    let str_misc = shared_ui.loc(str_misc_raw).clone();
                    tab!(str_ui, shared::SettingsState::Ui);
                    tab!(str_edit, shared::SettingsState::Editing);
                    tab!(str_rendering, shared::SettingsState::Rendering);
                    tab!(str_keyboard, shared::SettingsState::Keyboard);
                    tab!(str_colors, shared::SettingsState::Colors);
                    tab!(str_misc, shared::SettingsState::Misc);

                    if shared_ui.settings_state != shared::SettingsState::Rendering {
                        shared_ui.translucent_settings = false;
                    }

                    if !is_hovered {
                        shared_ui.hovering_setting = None;
                    }
                });
            });

            egui::ScrollArea::vertical().show(ui, |ui| {
                // add padding to the right, for the scrollbar
                let frame = egui::Frame::new().outer_margin(egui::Margin {
                    right: 13,
                    ..Default::default()
                });

                frame.show(ui, |ui| {
                    ui.set_width(window.x.min(375.));
                    ui.set_height(window.y.min(475.));
                    let layout = egui::Layout::top_down(egui::Align::Min);
                    shared_ui.updated_config = config.clone();

                    // show selected section
                    ui.with_layout(layout, |ui| match shared_ui.settings_state {
                        shared::SettingsState::Ui => user_interface(ui, shared_ui, config),
                        shared::SettingsState::Editing => editing(ui, shared_ui, config),
                        shared::SettingsState::Rendering => {
                            rendering(ui, shared_ui, camera, config)
                        }
                        shared::SettingsState::Keyboard => keyboard(ui, shared_ui),
                        shared::SettingsState::Colors => colors(ui, shared_ui),
                        shared::SettingsState::Misc => misc(ui, shared_ui, config),
                    });

                    events.update_config();
                });
            })
        });
        modal_ui.add_space(5.);
        modal_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // apply button
            let str = shared_ui.loc("settings_modal.cancel");
            if ui.skf_button(str).clicked() {
                events.reset_config();
                shared_ui.settings_modal = false;
                shared_ui.translucent_settings = false;
            }

            // cancel button
            let str = shared_ui.loc("settings_modal.apply");
            if ui.skf_button(str).clicked() {
                events.apply_settings();
                shared_ui.settings_modal = false;
                shared_ui.translucent_settings = false;
            }
        })
    });
}

pub fn settings_button(
    name: String,
    state: shared::SettingsState,
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    config: &crate::Config,
    width: f32,
    is_hovered: &mut bool,
) -> egui::Response {
    let mut col = config.colors.dark_accent;
    if shared_ui.hovering_setting == Some(state.clone()) {
        col += shared::Color::new(20, 20, 20, 0);
    }
    if shared_ui.settings_state == state.clone() {
        col += shared::Color::new(20, 20, 20, 0);
    }

    let rect = egui::Rect::from_min_size(
        egui::Pos2::new(ui.cursor().left(), ui.cursor().top()),
        egui::Vec2::new(width, 21.),
    );

    let id = egui::Id::new(format!("setting_{}", name));
    let button = ui
        .interact(rect, id, egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    egui::Frame::new().fill(col.into()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.set_width(width);
            ui.set_height(21.);
            let mut text_pos = ui.min_rect().left_center();
            text_pos.x += 5.;
            let align = egui::Align2::LEFT_CENTER;
            let font = egui::FontId::new(13., egui::FontFamily::Proportional);
            #[rustfmt::skip]
            ui.painter().text(text_pos, align, name.clone(), font, config.colors.text.into());
        });
    });

    if button.contains_pointer() || button.has_focus() {
        shared_ui.hovering_setting = Some(state.clone());
        *is_hovered = true;
    }
    if button.clicked() {
        shared_ui.settings_state = state;
    }
    button
}

fn user_interface(ui: &mut egui::Ui, shared_ui: &mut crate::Ui, config: &crate::Config) {
    let str_general = &shared_ui.loc("settings_modal.user_interface.heading");
    ui.horizontal(|ui| {
        ui.heading(str_general);

        // default button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                let config = &mut shared_ui.updated_config;
                config.layout = shared::UiLayout::Split;
                config.ui_scale = 1.;
            }
        });
    });

    alt_hor(ui, config, true, |ui| {
        // UI scale
        let str_ui_scale = &shared_ui.loc("settings_modal.user_interface.ui_scale");
        ui.label(str_ui_scale);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let scale = shared_ui.updated_config.ui_scale;
            let (edited, value, _) =
                ui.float_input("ui_scale".to_string(), shared_ui, scale, 1., None);
            if edited {
                shared_ui.updated_config.ui_scale = value;
            }

            // UI slider (web only)
            #[cfg(target_arch = "wasm32")]
            {
                let str = shared_ui.loc("settings_modal.user_interface.ui_slider");
                if ui.skf_button(&str).clicked() {
                    shared_ui.settings_modal = false;
                    crate::toggleElement(true, "ui-slider".to_string())
                }
            }
        });
    });

    // Layout dropdown (Split, Left, Right)
    let str_split = shared_ui.loc("settings_modal.user_interface.layout_split");
    let str_left = shared_ui.loc("settings_modal.user_interface.layout_left");
    let str_right = shared_ui.loc("settings_modal.user_interface.layout_right");
    let used_str = match shared_ui.updated_config.layout {
        shared::UiLayout::Split => str_split.clone(),
        shared::UiLayout::Right => str_right.clone(),
        shared::UiLayout::Left => str_left.clone(),
    };

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("settings_modal.user_interface.layout"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let combo_box = egui::ComboBox::new("layout", "").selected_text(used_str);
            combo_box.show_ui(ui, |ui| {
                let config = &mut shared_ui.updated_config.layout;
                ui.selectable_value(config, shared::UiLayout::Split, str_split);
                ui.selectable_value(config, shared::UiLayout::Right, str_right);
                ui.selectable_value(config, shared::UiLayout::Left, str_left);
            });
        });
    });

    ui.add_space(20.);
}

fn editing(ui: &mut egui::Ui, shared_ui: &mut crate::Ui, config: &crate::Config) {
    ui.horizontal(|ui| {
        let str_heading = &shared_ui.loc("settings_modal.editing.heading");
        ui.heading(str_heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                let config = &mut shared_ui.updated_config;
                config.edit_while_playing = crate::Config::default().edit_while_playing;
                config.rot_snap_step = crate::Config::default().rot_snap_step;
                config.transform_rot_radius = crate::Config::default().transform_rot_radius;
                config.center_point_radius = crate::Config::default().center_point_radius;
                config.transform_scale_radius = crate::Config::default().transform_scale_radius;
            }
        });
    });

    basic_checkbox(
        ui,
        &shared_ui.loc("settings_modal.editing.edit_while_playing"),
        &shared_ui.loc("settings_modal.editing.edit_while_playing_desc"),
        &mut shared_ui.updated_config.edit_while_playing,
        config,
        true,
    );

    shared_ui.updated_config.rot_snap_step = basic_input(
        "settings_modal.editing.rot_snap_step",
        shared_ui.updated_config.rot_snap_step,
        shared_ui,
        ui,
        config,
        false,
    );

    ui.add_space(10.);

    shared_ui.updated_config.center_point_radius = basic_input(
        "settings_modal.editing.center_point_radius",
        shared_ui.updated_config.center_point_radius,
        shared_ui,
        ui,
        config,
        true,
    );
    shared_ui.updated_config.transform_rot_radius = basic_input(
        "settings_modal.editing.transform_rot_radius",
        shared_ui.updated_config.transform_rot_radius,
        shared_ui,
        ui,
        config,
        false,
    );
    shared_ui.updated_config.transform_scale_radius = basic_input(
        "settings_modal.editing.transform_scale_radius",
        shared_ui.updated_config.transform_scale_radius,
        shared_ui,
        ui,
        config,
        true,
    );
}

fn rendering(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    camera: &crate::Camera,
    config: &crate::Config,
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
                let config = &mut shared_ui.updated_config;
                config.gridline_gap = crate::Config::default().gridline_gap;
                config.pixel_magnification = crate::Config::default().pixel_magnification;
            }
        });
    });

    let gap = shared_ui.updated_config.gridline_gap;
    let gap_str = "settings_modal.rendering.gridline_gap";
    shared_ui.updated_config.gridline_gap =
        basic_input(gap_str, gap as f32, shared_ui, ui, config, true) as i32;

    alt_hor(ui, config, false, |ui| {
        let str_heading = &shared_ui.loc("settings_modal.rendering.pixel_mag");
        ui.label(str_heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let id = "pixelmag".to_string();
            let mag = shared_ui.updated_config.pixel_magnification as f32;
            let (edited, value, _) = ui.float_input(id, shared_ui, mag, 1., None);
            if edited {
                shared_ui.updated_config.pixel_magnification = (value as i32).max(1);
            }
            let window = camera.window / shared_ui.updated_config.pixel_magnification as f32;
            ui.label(format!("{}, {}", window.x, window.y));
        });
    });

    basic_checkbox(
        ui,
        &shared_ui.loc("settings_modal.rendering.gridline_front"),
        "",
        &mut shared_ui.updated_config.gridline_front,
        config,
        true,
    );

    ui.add_space(7.);
}

fn misc(ui: &mut egui::Ui, shared_ui: &mut crate::Ui, config: &crate::Config) {
    ui.horizontal(|ui| {
        let str_heading = &shared_ui.loc("settings_modal.miscellaneous.heading");
        ui.heading(str_heading);

        // default button
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                let config = &mut shared_ui.updated_config;
                config.autosave_frequency = Config::default().autosave_frequency;
                config.exact_bone_select = Config::default().exact_bone_select;
                config.keep_tex_str = Config::default().keep_tex_str;
                config.skip_startup = Config::default().skip_startup;
            }
        });
    });

    #[cfg(not(target_arch = "wasm32"))]
    alt_hor(ui, config, true, |ui| {
        let str_autosave_freq = &shared_ui.loc("settings_modal.miscellaneous.autosave_frequency");
        ui.label(str_autosave_freq);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let id = "autosave_freq".to_string();
            let auto_freq = shared_ui.updated_config.autosave_frequency as f32;
            let (edited, value, _) = ui.float_input(id, shared_ui, auto_freq, 1., None);
            if edited && value > 0. {
                shared_ui.updated_config.autosave_frequency = value as i32;
            }
        });
    });
    basic_checkbox(
        ui,
        &shared_ui.loc("settings_modal.miscellaneous.select_exact_bone"),
        &shared_ui.loc("settings_modal.miscellaneous.select_exact_bone_desc"),
        &mut shared_ui.updated_config.exact_bone_select,
        config,
        false,
    );
    basic_checkbox(
        ui,
        &shared_ui.loc("settings_modal.miscellaneous.keep_tex_str"),
        &shared_ui.loc("settings_modal.miscellaneous.keep_tex_str_desc"),
        &mut shared_ui.updated_config.keep_tex_str,
        config,
        true,
    );
    basic_checkbox(
        ui,
        &shared_ui.loc("settings_modal.miscellaneous.use_fallback"),
        &shared_ui.loc("settings_modal.miscellaneous.use_fallback_desc"),
        &mut shared_ui.use_fallback,
        config,
        false,
    );

    ui.add_space(20.);

    let str_startup = &shared_ui.loc("top_bar.file.startup");
    ui.heading(str_startup);

    basic_checkbox(
        ui,
        &shared_ui.loc("settings_modal.miscellaneous.skip_startup_window"),
        "",
        &mut shared_ui.use_fallback,
        config,
        true,
    );

    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        if shared_ui.recent_file_paths.len() == 0 {
            ui.disable();
        }
        let str_clear_recents = &shared_ui.loc("settings_modal.miscellaneous.clear_recent_files");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.skf_button(str_clear_recents).clicked() {
                shared_ui.recent_file_paths = vec![];
                crate::utils::save_to_recent_files(&vec![]);
            }
        });
    });

    ui.add_space(20.);

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.horizontal(|ui| {
            if ui.skf_button("Intentionally Crash").clicked() {
                panic!();
            }
        })
    });
}

fn color_row(
    title: String,
    color: &mut shared::Color,
    bg: shared::Color,
    ui: &mut egui::Ui,
    alpha: bool,
) {
    let frame = egui::Frame {
        fill: bg.into(),
        ..Default::default()
    };
    ui.horizontal(|ui| {
        egui::Frame::show(frame, ui, |ui| {
            ui.label(title);

            // color picker
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if alpha {
                    let mut col: [u8; 4] = [color.r, color.g, color.b, color.a];
                    ui.color_edit_button_srgba_unmultiplied(&mut col);
                    *color = shared::Color::new(col[0], col[1], col[2], col[3]);
                } else {
                    let mut col: [u8; 3] = [color.r, color.g, color.b];
                    ui.color_edit_button_srgb(&mut col);
                    *color = shared::Color::new(col[0], col[1], col[2], 255);
                }
            });
        });
    });
}

fn keyboard(ui: &mut egui::Ui, shared_ui: &mut crate::Ui) {
    ui.horizontal(|ui| {
        let str_heading = &shared_ui.loc("settings_modal.keyboard.heading");
        ui.heading(str_heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                shared_ui.updated_config.keys = crate::KeyboardConfig::default();
            }
        });
    });

    let mut alt_col = true;

    macro_rules! loc {
        ($label:expr) => {
            shared_ui
                .loc(&format!("settings_modal.keyboard.{}", $label))
                .clone()
        };
    }

    macro_rules! key {
        ($label:expr, $field:expr, $has_key:expr) => {
            alt_col = !alt_col;
            let color = if alt_col {
                shared_ui.updated_config.colors.main
            } else {
                shared_ui.updated_config.colors.dark_accent
            };
            let text_col = shared_ui.updated_config.colors.text;
            let label_desc = loc!(&format!("{}_desc", $label));
            #[rustfmt::skip]
            key(loc!($label), label_desc, &mut $field, ui, &mut shared_ui.changing_key, &shared_ui.last_pressed, color, text_col, $has_key);
        };
    }

    let mut keys = shared_ui.updated_config.keys.clone();
    // iterable key config
    #[rustfmt::skip]
    {
        ui.heading(shared_ui.loc("settings_modal.keyboard.sections.general"));
        key!("zoom_camera_in",  keys.zoom_in_camera,  true);
        key!("zoom_camera_out", keys.zoom_out_camera, true);
        key!("undo",            keys.undo,            true);
        key!("redo",            keys.redo,            true);
        key!("save",            keys.save,            true);
        key!("save_as",         keys.save_as,         true);
        key!("export",          keys.export,          true);
        key!("open",            keys.open,            true);
        key!("cancel",          keys.cancel,          true);
        key!("delete",          keys.delete,          true);
        key!("polar_yes",       keys.polar_yes,       true);
        key!("copy",            keys.copy,            true);
        key!("paste",           keys.paste,           true);
        ui.add_space(10.);
        ui.heading(shared_ui.loc("settings_modal.keyboard.sections.editing"));
        alt_col = true;
        key!("transform_move",       keys.transform_move,       true);
        key!("transform_rotate",     keys.transform_rotate,     true);
        key!("transform_scale",      keys.transform_scale,      true);
        key!("edit_modifier",        keys.edit_modifier,        false);
        key!("edit_snap",            keys.edit_snap,            false);
        key!("edit_alt",             keys.edit_alt,             false);
        key!("next_bone",            keys.next_bone,            true);
        key!("prev_bone",            keys.prev_bone,            true);
        key!("toggle_bone_fold",     keys.toggle_bone_fold,     true);
        key!("toggle_edit_vertices", keys.toggle_edit_vertices, true);
        ui.add_space(10.);
        ui.heading(shared_ui.loc("settings_modal.keyboard.sections.keyframe_editor"));
        alt_col = true;
        key!("next_anim_frame",    keys.next_anim_frame,    true);
        key!("prev_anim_frame",    keys.prev_anim_frame,    true);
        key!("next_keyframe",      keys.next_keyframe,      true);
        key!("prev_keyframe",      keys.prev_keyframe,      true);
        key!("toggle_animation",   keys.toggle_animation,   true);
        key!("play_animation",     keys.play_animation,     true);
        key!("timeline_zoom_mode", keys.timeline_zoom_mode, false);
    };
    ui.add_space(10.);
    shared_ui.updated_config.keys = keys.clone();
}

fn colors(ui: &mut egui::Ui, shared_ui: &mut crate::Ui) {
    // heading & default button
    ui.horizontal(|ui| {
        let str_colors = &shared_ui.loc("settings_modal.user_interface.colors_heading");
        ui.heading(str_colors);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                shared_ui.updated_config.colors = crate::ColorConfig::default();
            }
        });
    });

    let mut alt_col = true;
    macro_rules! color_row {
        ($title:expr, $color:expr, $alpha:expr) => {
            alt_col = !alt_col;
            let col = &shared_ui.updated_config.colors;
            let color = if alt_col { col.main } else { col.dark_accent };
            let str_color = shared_ui
                .loc(&format!("settings_modal.colors.{}", $title))
                .clone();
            color_row(str_color, $color, color, ui, $alpha);
        };
    }

    macro_rules! colors {
        () => {
            shared_ui.updated_config.colors
        };
    }

    // iterable color buttons
    #[rustfmt::skip]
    {
        ui.heading(shared_ui.loc("settings_modal.user_interface.heading"));
        color_row!("main",               &mut colors!().main,               false);
        color_row!("light_accent",       &mut colors!().light_accent,       false);
        color_row!("dark_accent",        &mut colors!().dark_accent,        false);
        color_row!("text",               &mut colors!().text,               false);
        color_row!("frameline",          &mut colors!().frameline,          false);
        color_row!("gradient",           &mut colors!().gradient,           false);
        color_row!("link",               &mut colors!().link,               false);
        color_row!("warning_text",       &mut colors!().warning_text,       false);
        color_row!("inverse_kinematics", &mut colors!().inverse_kinematics, false);
        color_row!("meshdef",            &mut colors!().meshdef,            false);
        color_row!("texture",            &mut colors!().texture,            false);
        color_row!("ik_target",          &mut colors!().ik_target,          false);
        ui.add_space(10.);
        ui.heading(shared_ui.loc("settings_modal.rendering.heading"));
        alt_col = true;
        color_row!("background",            &mut colors!().background,            false);
        color_row!("gridline",              &mut colors!().gridline,              false);
        color_row!("center_point",          &mut colors!().center_point,          true);
        color_row!("inactive_center_point", &mut colors!().inactive_center_point, true);
        color_row!("transform_rings",       &mut colors!().transform_rings,       true);

    };
}

fn key(
    name: String,
    tooltip: String,
    field: &mut egui::KeyboardShortcut,
    ui: &mut egui::Ui,
    changing_key: &mut String,
    last_pressed: &Option<egui::Key>,
    color: shared::Color,
    text_color: shared::Color,
    show_key: bool,
) {
    macro_rules! dd_mod {
        ($ui:expr, $modifier:expr, $field:expr) => {
            let text = egui::RichText::new(modifier_name($modifier)).color(text_color);
            $ui.selectable_value(&mut $field, $modifier, text);
        };
    }

    ui.horizontal(|ui| {
        egui::Frame::new().fill(color.into()).show(ui, |ui| {
            if tooltip != "" {
                ui.label(name.clone()).on_hover_text(tooltip);
            } else {
                ui.label(name.clone());
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // logical key
                if show_key {
                    let button_str = if *changing_key == name {
                        "...".to_string()
                    } else {
                        field.logical_key.display()
                    };
                    let button_rich_text = egui::RichText::new(button_str).color(text_color);
                    let button = ui
                        .add_sized([80., 20.], egui::Button::new(button_rich_text))
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if button.clicked() {
                        *changing_key = name.to_string();
                    }
                } else {
                    // add padding, to align modifier dropdown to the rest in the list
                    ui.add_space(88.);
                }

                // modifier dropdown
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

fn basic_input(
    label: &str,
    field: f32,
    shared_ui: &mut crate::Ui,
    ui: &mut egui::Ui,
    config: &crate::Config,
    alt: bool,
) -> f32 {
    let mut result = field;
    alt_hor(ui, config, alt, |ui| {
        let str = &shared_ui.loc(label);
        ui.label(str);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (edited, value, _) = ui.float_input(label.to_string(), shared_ui, field, 1., None);
            if edited {
                result = value
            }
        });
    });
    result
}

pub fn basic_checkbox(
    ui: &mut egui::Ui,
    label: &str,
    desc: &str,
    field: &mut bool,
    config: &crate::Config,
    alt: bool,
) {
    alt_hor(ui, config, alt, |ui| {
        if desc == "" {
            ui.label(label);
        } else {
            ui.label(label).on_hover_text(desc);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.scope(|ui| {
                ui.style_mut().visuals.widgets.inactive.bg_fill = if alt {
                    config.colors.main
                } else {
                    config.colors.dark_accent
                }
                .into();
                if desc == "" {
                    ui.checkbox(field, "".into_atoms());
                } else {
                    ui.checkbox(field, "".into_atoms()).on_hover_text(desc);
                }
            })
        });
    });
}

pub fn alt_hor<T: FnOnce(&mut egui::Ui)>(
    ui: &mut egui::Ui,
    config: &crate::Config,
    alt: bool,
    content: T,
) {
    let frame = egui::Frame {
        fill: if alt {
            config.colors.dark_accent.into()
        } else {
            config.colors.main.into()
        },
        ..Default::default()
    };
    ui.horizontal(|ui| {
        frame.show(ui, |ui| {
            content(ui);
        });
    });
}
