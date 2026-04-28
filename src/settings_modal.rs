use egui::IntoAtoms;

use crate::{shared, ui::EguiUi, Display};

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
                    let str_edit = shared_ui.loc("settings_modal.editing.heading").clone();
                    let str_rendering = shared_ui.loc("settings_modal.rendering.heading").clone();
                    let str_keyboard = shared_ui.loc("settings_modal.keyboard.heading").clone();
                    let str_misc = shared_ui.loc(str_misc_raw).clone();
                    tab!(str_ui, shared::SettingsState::Ui);
                    tab!(str_edit, shared::SettingsState::Editing);
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
                    shared_ui.updated_config = config.clone();
                    ui.with_layout(layout, |ui| match shared_ui.settings_state {
                        shared::SettingsState::Ui => user_interface(ui, shared_ui),
                        shared::SettingsState::Editing => editing(ui, shared_ui),
                        shared::SettingsState::Rendering => rendering(ui, shared_ui, camera),
                        shared::SettingsState::Keyboard => keyboard(ui, shared_ui),
                        shared::SettingsState::Misc => misc(ui, shared_ui),
                    });
                    events.update_config();
                });
            })
        });

        modal_ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.skf_button(shared_ui.loc("settings_modal.apply")).clicked() {
                events.apply_settings();
                shared_ui.settings_modal = false;
                shared_ui.translucent_settings = false;
            }
            if ui.skf_button(shared_ui.loc("settings_modal.cancel")).clicked() {
                events.reset_config();
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
) {
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
}

fn user_interface(ui: &mut egui::Ui, shared_ui: &mut crate::Ui) {
    let str_general = &shared_ui.loc("settings_modal.user_interface.general");
    ui.heading(str_general);
    ui.horizontal(|ui| {
        let str_ui_scale = &shared_ui.loc("settings_modal.user_interface.ui_scale");
        ui.label(str_ui_scale);
        let scale = shared_ui.updated_config.ui_scale;
        let (edited, value, _) = ui.float_input("ui_scale".to_string(), shared_ui, scale, 1., None);
        if edited {
            shared_ui.updated_config.ui_scale = value;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let str = shared_ui.loc("settings_modal.user_interface.ui_slider");
            if ui.skf_button(&str).clicked() {
                shared_ui.settings_modal = false;
                crate::toggleElement(true, "ui-slider".to_string())
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("settings_modal.user_interface.layout"));
        let combo_box = egui::ComboBox::new("layout", "")
            .selected_text(shared_ui.updated_config.layout.to_string());
        combo_box.show_ui(ui, |ui| {
            let str_split = shared_ui.loc("settings_modal.user_interface.layout_split");
            let str_left = shared_ui.loc("settings_modal.user_interface.layout_left");
            let str_right = shared_ui.loc("settings_modal.user_interface.layout_right");
            let config = &mut shared_ui.updated_config.layout;
            ui.selectable_value(config, shared::UiLayout::Split, str_split);
            ui.selectable_value(config, shared::UiLayout::Right, str_right);
            ui.selectable_value(config, shared::UiLayout::Left, str_left);
        });
    });

    ui.add_space(20.);

    let mut alt_col = true;

    macro_rules! color_row {
        ($title:expr, $color:expr) => {
            alt_col = !alt_col;
            let col = &shared_ui.updated_config.colors;
            let color = if alt_col { col.main } else { col.dark_accent };
            let str_color = shared_ui
                .loc(&format!("settings_modal.user_interface.colors.{}", $title))
                .clone();
            color_row(str_color, $color, color, ui, false);
        };
    }

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

    // iterable color config
    #[rustfmt::skip]
    {
        color_row!("main",               &mut shared_ui.updated_config.colors.main              );
        color_row!("light_accent",       &mut shared_ui.updated_config.colors.light_accent      );
        color_row!("dark_accent",        &mut shared_ui.updated_config.colors.dark_accent       );
        color_row!("text",               &mut shared_ui.updated_config.colors.text              );
        color_row!("frameline",          &mut shared_ui.updated_config.colors.frameline         );
        color_row!("gradient",           &mut shared_ui.updated_config.colors.gradient          );
        color_row!("link",               &mut shared_ui.updated_config.colors.link              );
        color_row!("warning_text",       &mut shared_ui.updated_config.colors.warning_text      );
        color_row!("inverse_kinematics", &mut shared_ui.updated_config.colors.inverse_kinematics);
        color_row!("meshdef",            &mut shared_ui.updated_config.colors.meshdef           );
        color_row!("texture",            &mut shared_ui.updated_config.colors.texture           );
        color_row!("ik_target",          &mut shared_ui.updated_config.colors.ik_target         );
    };
    ui.add_space(20.);
}

fn editing(ui: &mut egui::Ui, shared_ui: &mut crate::Ui) {
    ui.horizontal(|ui| {
        let str_heading = &shared_ui.loc("settings_modal.editing.heading");
        ui.heading(str_heading);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let str_default = &shared_ui.loc("settings_modal.default");
            if ui.skf_button(str_default).clicked() {
                let config = &mut shared_ui.updated_config;
                config.edit_while_playing = crate::Config::default().edit_while_playing;
                config.propagate_visibility = crate::Config::default().propagate_visibility;
                config.rot_snap_step = crate::Config::default().rot_snap_step;
                config.transform_rot_radius = crate::Config::default().transform_rot_radius;
                config.center_point_radius = crate::Config::default().center_point_radius;
                config.transform_scale_radius = crate::Config::default().transform_scale_radius;
            }
        });
    });

    ui.horizontal(|ui| {
        let str_edit = &shared_ui.loc("settings_modal.editing.edit_while_playing");
        let str_edit_desc = &shared_ui.loc("settings_modal.editing.edit_while_playing_desc");
        ui.label(str_edit).on_hover_text(str_edit_desc);
        ui.checkbox(
            &mut shared_ui.updated_config.edit_while_playing,
            "".into_atoms(),
        );
    });

    ui.horizontal(|ui| {
        let str_edit = &shared_ui.loc("settings_modal.editing.propagate_visibility");
        let str_edit_desc = &shared_ui.loc("settings_modal.editing.propagate_visibility_desc");
        ui.label(str_edit).on_hover_text(str_edit_desc);
        ui.checkbox(
            &mut shared_ui.updated_config.propagate_visibility,
            "".into_atoms(),
        );
    });

    shared_ui.updated_config.rot_snap_step = basic_input(
        "settings_modal.editing.rot_snap_step",
        shared_ui.updated_config.rot_snap_step,
        shared_ui,
        ui,
    );

    ui.add_space(10.);

    shared_ui.updated_config.center_point_radius = basic_input(
        "settings_modal.editing.center_point_radius",
        shared_ui.updated_config.center_point_radius,
        shared_ui,
        ui,
    );
    shared_ui.updated_config.transform_rot_radius = basic_input(
        "settings_modal.editing.transform_rot_radius",
        shared_ui.updated_config.transform_rot_radius,
        shared_ui,
        ui,
    );
    shared_ui.updated_config.transform_scale_radius = basic_input(
        "settings_modal.editing.transform_scale_radius",
        shared_ui.updated_config.transform_scale_radius,
        shared_ui,
        ui,
    );
}

fn rendering(ui: &mut egui::Ui, shared_ui: &mut crate::Ui, camera: &crate::Camera) {
    #[rustfmt::skip]
    macro_rules! colors { () => { shared_ui.updated_config.colors } }

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
                colors!().background = crate::Config::default().colors.background;
                colors!().gridline = crate::Config::default().colors.gridline;
                colors!().center_point = crate::Config::default().colors.center_point;
                colors!().inactive_center_point =
                    crate::Config::default().colors.inactive_center_point;
                colors!().transform_rings = crate::Config::default().colors.transform_rings;
                let config = &mut shared_ui.updated_config;
                config.gridline_gap = crate::Config::default().gridline_gap;
                config.pixel_magnification = crate::Config::default().pixel_magnification;
            }
        });
    });

    let gap = shared_ui.updated_config.gridline_gap;
    let gap_str = "settings_modal.rendering.gridline_gap";
    shared_ui.updated_config.gridline_gap = basic_input(gap_str, gap as f32, shared_ui, ui) as i32;

    ui.horizontal(|ui| {
        let str_gridline_gap = &shared_ui.loc("settings_modal.rendering.gridline_front");
        ui.label(str_gridline_gap);
        ui.checkbox(
            &mut shared_ui.updated_config.gridline_front,
            "".into_atoms(),
        );
    });

    ui.horizontal(|ui| {
        let str_heading = &shared_ui.loc("settings_modal.rendering.pixel_mag");
        ui.label(str_heading);
        let id = "pixelmag".to_string();
        let mag = shared_ui.updated_config.pixel_magnification as f32;
        let (edited, value, _) = ui.float_input(id, shared_ui, mag, 1., None);
        if edited {
            shared_ui.updated_config.pixel_magnification = (value as i32).max(1);
        }
        let window = camera.window / shared_ui.updated_config.pixel_magnification as f32;
        ui.label(format!(
            "= {}, {}",
            window.x.to_string(),
            window.y.to_string()
        ));
    });

    ui.add_space(7.);

    let mut alt_col = true;
    macro_rules! color_row {
        ($title:expr, $color:expr, $alpha:expr) => {
            alt_col = !alt_col;
            let col = &shared_ui.updated_config.colors;
            let bg_color = if alt_col { col.main } else { col.dark_accent };
            let str_color = shared_ui
                .loc(&format!("settings_modal.rendering.{}", $title))
                .clone();
            let mut col = $color.clone();
            color_row(str_color, &mut col, bg_color, ui, $alpha);
            $color = col;
        };
    }

    ui.add_space(7.);

    color_row!("background", colors!().background, false);
    color_row!("gridline", colors!().gridline, false);
    color_row!("center_point", colors!().center_point, true);
    color_row!(
        "inactive_center_point",
        colors!().inactive_center_point,
        true
    );
    color_row!("transform_rings", colors!().transform_rings, true);
}

fn misc(ui: &mut egui::Ui, shared_ui: &mut crate::Ui) {
    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        let str_autosave_freq = &shared_ui.loc("settings_modal.miscellaneous.autosave_frequency");
        ui.label(str_autosave_freq);
        let id = "autosave_freq".to_string();
        let auto_freq = shared_ui.updated_config.autosave_frequency as f32;
        let (edited, value, _) = ui.float_input(id, shared_ui, auto_freq, 1., None);
        if edited && value > 0. {
            shared_ui.updated_config.autosave_frequency = value as i32;
        }
    });
    ui.horizontal(|ui| {
        let str_exact_bone = &shared_ui.loc("settings_modal.miscellaneous.select_exact_bone");
        let str_exact_bone_desc =
            &shared_ui.loc("settings_modal.miscellaneous.select_exact_bone_desc");
        ui.label(str_exact_bone).on_hover_text(str_exact_bone_desc);
        ui.checkbox(
            &mut shared_ui.updated_config.exact_bone_select,
            "".into_atoms(),
        );
    });
    ui.horizontal(|ui| {
        let str_keep_tex_str = &shared_ui.loc("settings_modal.miscellaneous.keep_tex_str");
        let str_keep_tex_str_desc =
            &shared_ui.loc("settings_modal.miscellaneous.keep_tex_str_desc");
        ui.label(str_keep_tex_str)
            .on_hover_text(str_keep_tex_str_desc);
        ui.checkbox(&mut shared_ui.updated_config.keep_tex_str, "".into_atoms());
    });
    ui.horizontal(|ui| {
        let str_fallback = &shared_ui.loc("settings_modal.miscellaneous.use_fallback");
        let str_fallback_desc = &shared_ui.loc("settings_modal.miscellaneous.use_fallback_desc");
        ui.label(str_fallback).on_hover_text(str_fallback_desc);
        ui.checkbox(&mut shared_ui.use_fallback, "".into_atoms());
    });

    ui.add_space(20.);

    let str_startup = &shared_ui.loc("top_bar.file.startup");
    ui.heading(str_startup);
    ui.horizontal(|ui| {
        let str_skip_startup = &shared_ui.loc("settings_modal.miscellaneous.skip_startup_window");
        ui.label(str_skip_startup);
        ui.checkbox(&mut shared_ui.updated_config.skip_startup, "".into_atoms());
    });
    #[cfg(not(target_arch = "wasm32"))]
    ui.horizontal(|ui| {
        if shared_ui.recent_file_paths.len() == 0 {
            ui.disable();
        }
        let str_clear_recents = &shared_ui.loc("settings_modal.miscellaneous.clear_recent_files");
        if ui.skf_button(str_clear_recents).clicked() {
            shared_ui.recent_file_paths = vec![];
            crate::utils::save_to_recent_files(&vec![]);
        }
    });

    ui.add_space(20.);

    //let str_startup = &shared_ui.loc("settings_modal.miscellaneous.beta.heading");
    //ui.heading(str_startup);
    ////let text = shared.ui.loc("settings_modal.miscellaneous.beta.warning");
    //let mut text = shared_ui.loc("settings_modal.miscellaneous.beta.nothing");
    //text = text.replace("$version", env!("CARGO_PKG_VERSION"));
    //let mut cache = egui_commonmark::CommonMarkCache::default();
    //let str = utils::markdown(text, shared_ui.local_doc_url.to_string());
    //egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
    //ui.add_space(20.);

    if ui.button("Intentionally Crash").clicked() {
        panic!();
    }
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
        ui.heading("General");
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
        ui.heading("Editing");
        key!("transform_move",       keys.transform_move,       true);
        key!("transform_rotate",     keys.transform_rotate,     true);
        key!("transform_scale",      keys.transform_scale,      true);
        key!("edit_modifier",        keys.edit_modifier,        false);
        key!("edit_snap",            keys.edit_snap,            false);
        key!("next_bone",            keys.next_bone,            true);
        key!("prev_bone",            keys.prev_bone,            true);
        key!("toggle_bone_fold",     keys.toggle_bone_fold,     true);
        key!("toggle_edit_vertices", keys.toggle_edit_vertices, true);
        ui.add_space(10.);
        ui.heading("Keyframe Editor");
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

fn basic_input(label: &str, field: f32, shared_ui: &mut crate::Ui, ui: &mut egui::Ui) -> f32 {
    let mut result = field;
    ui.horizontal(|ui| {
        let str = &shared_ui.loc(label);
        ui.label(str);
        let (edited, value, _) = ui.float_input(label.to_string(), shared_ui, field, 1., None);
        if edited {
            result = value
        }
    });
    result
}
