//! Core user interface (UI) logic.
use egui::{Color32, Context, Shadow, Stroke};
use modal::modal_x;

use crate::*;

#[rustfmt::skip]
pub trait EguiUi {
    fn skf_button(&mut self, text: &str) -> egui::Response;
    fn gradient(&mut self, rect: egui::Rect, top: Color32, bottom: Color32);
    fn clickable_label(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response;
    fn text_input(&mut self,id: String, shared_ui: &mut crate::Ui, value: String, options: Option<TextInputOptions>) -> (bool, String, egui::Response);
    fn float_input(&mut self,id: String,shared_ui: &mut crate::Ui,value: f32,modifier: f32,options: Option<TextInputOptions>) -> (bool, f32, egui::Response);
    fn debug_rect(&mut self, rect: egui::Rect);
    fn context_rename(&mut self, shared_ui: &mut crate::Ui, config: &Config, id: String);
    fn context_delete(&mut self, shared_ui: &mut crate::Ui, config: &Config, events: &mut EventState, loc_code: &str, polar_id: PolarId);
    fn context_button(&mut self, text: impl Into<egui::WidgetText>, config: &Config) -> egui::Response;
}

// all context menus must be opened through this.
// `content` is a closure with a `&mut egui::Ui` parameter.
#[macro_export]
macro_rules! context_menu {
    ($button:expr, $ui:expr, $id:expr, $content:expr) => {
        if $button.secondary_clicked() {
            $ui.context_menu.show(&$id)
        }
        if $ui.context_menu.is(&$id) {
            $button.show_tooltip_ui(|ui| {
                $content(ui);
                if ui.ui_contains_pointer() {
                    $ui.context_menu.keep = true;
                }
            });
        }
    };
}

/// The `main` of this module.
pub fn draw(
    context: &Context,
    shared_ui: &mut crate::Ui,
    input: &InputStates,
    selections: &mut SelectionState,
    config: &mut Config,
    events: &mut EventState,
    edit_mode: &mut EditMode,
    camera: &Camera,
    armature: &mut Armature,
    copy_buffer: &mut CopyBuffer,
) {
    shared_ui.context_menu.keep = false;

    let sel = selections.clone();

    context.set_cursor_icon(shared_ui.cursor_icon);
    shared_ui.cursor_icon = egui::CursorIcon::Default;

    default_styling(context, &config);

    // apply individual element styling once, then immediately go back to default
    macro_rules! style_once {
        ($func:expr) => {
            $func;
            default_styling(context, &config);
        };
    }

    if let Some(_pos) = context.pointer_latest_pos() {
        if shared_ui.mobile {
            #[cfg(feature = "debug")]
            context
                .debug_painter()
                .circle_filled(_pos, 2., egui::Color32::GREEN);
        }
    }

    let anim_icon_size = 18;
    if shared_ui.anim.icon_images.len() == 0 {
        let full_img = image::load_from_memory(include_bytes!("../assets/anim_icons.png")).unwrap();

        let mut x = 0;
        while full_img.width() > 0 && x < full_img.width() - 1 {
            let img = full_img.crop_imm(x, 0, 18, 18).into_rgba8();
            x += anim_icon_size;
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [img.width() as usize, img.height() as usize],
                img.as_flat_samples().as_slice(),
            );
            let name = "anim_icon_".to_owned() + &x.to_string();
            let tex = context.load_texture(name, color_image, Default::default());
            shared_ui.anim.icon_images.push(tex);
        }
    }
    if !shared_ui.startup_window {
        camera_bar(context, config, shared_ui, camera, events);
    }

    if shared_ui.polar_modal {
        modal::polar_modal(context, &config, shared_ui, events);
    }
    if shared_ui.modal {
        modal::modal(context, shared_ui, &config);
    }
    if shared_ui.styles_modal {
        styles_modal::draw(
            context, shared_ui, config, camera, armature, selections, events,
        );
    }
    if shared_ui.settings_modal {
        settings_modal::draw(shared_ui, config, camera, events, context);
    }
    if shared_ui.startup_window {
        startup_window::startup_modal(context, shared_ui, events, &config);
    }
    if shared_ui.donating_modal {
        modal::donating_modal(context, shared_ui, &config);
    }
    if shared_ui.atlas_modal {
        atlas_modal::draw(
            context, config, selections, armature, shared_ui, input, events,
        );
    }
    if shared_ui.export_modal {
        export_modal::draw(
            context, shared_ui, &edit_mode, config, events, armature, camera, selections,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    if shared_ui.checking_update {
        modal::modal(context, shared_ui, &config);
        let url = "https://skelform.org/data/version";
        let request = ureq::get(url).header("Example-Header", "header value");
        let raw_ver = match request.call() {
            Ok(mut data) => data.body_mut().read_to_string().unwrap(),
            Err(_) => "err".to_string(),
        };

        if raw_ver == "err" {
            events.open_modal("startup.error_update", false);
        } else if raw_ver != "" {
            let ver_str = raw_ver.split(' ').collect::<Vec<_>>();
            let ver_idx = ver_str[0].parse::<i32>().unwrap();
            let ver_name = ver_str[1];
            if ver_idx > crate::VERSION_IDX {
                shared_ui.new_version = ver_name.to_string();
                let str = "New version available: ".to_owned()
                    + &ver_name
                    + "\nGo to version page and download manually?";
                events.open_polar_modal(PolarId::NewUpdate, str);
            } else {
                events.open_modal("No updates available. This is the latest version.", false);
            }
        }

        shared_ui.checking_update = false;
    }
    style_once!(top_panel(
        context, config, shared_ui, events, selections, armature, camera, edit_mode
    ));

    if edit_mode.anim_open {
        style_once!(keyframe_editor::draw(
            context,
            shared_ui,
            input,
            armature,
            config,
            selections,
            events,
            copy_buffer
        ));
    }

    style_once!(armature_window::draw(
        context, events, config, armature, selections, edit_mode, shared_ui
    ));

    let min_default_size = 210.;
    let mut max_size = min_default_size;
    if selections.bone_idx != usize::MAX {
        max_size = 250.;
    } else if selections.anim_frame != -1 {
        max_size = 250.;
    }

    let mut enable_bone_panel = true;
    if let Some(_) = armature.sel_bone(&sel) {
        enable_bone_panel = !edit_mode.setting_ik_target;
    }

    // get current properties of selected bone, including animations
    let mut selected_bone = Bone::default();
    if selections.bone_idx != usize::MAX && selections.bone_idx < armature.bones.len() {
        selected_bone = armature.sel_bone(&sel).unwrap().clone();

        if edit_mode.anim_open && selections.anim != usize::MAX {
            let frame = selections.anim_frame;
            let animated_bones = armature.animate(selections.anim, frame, None);
            selected_bone = animated_bones[selections.bone_idx].clone();
        }
    }

    let bone_panel_id = "Bone";
    let mut side_panel = egui::SidePanel::right(bone_panel_id)
        .resizable(true)
        .max_width(max_size)
        .min_width(min_default_size)
        .default_width(min_default_size);
    if config.layout == UiLayout::Left {
        side_panel = egui::SidePanel::left(bone_panel_id)
            .resizable(true)
            .max_width(max_size)
            .min_width(min_default_size)
            .default_width(min_default_size);
    }
    draw_resizable_panel(
        bone_panel_id,
        side_panel.show(context, |ui| {
            ui.add_enabled_ui(enable_bone_panel, |ui| {
                let gradient = config.colors.gradient.into();
                ui.gradient(ui.ctx().content_rect(), Color32::TRANSPARENT, gradient);

                if selections.bone_idx != usize::MAX {
                    bone_panel::draw(
                        selected_bone.clone(),
                        ui,
                        selections,
                        shared_ui,
                        armature,
                        config,
                        events,
                        &input,
                        edit_mode,
                    );
                } else if armature.sel_anim(&selections) != None && selections.anim_frame != -1 {
                    keyframe_panel::draw(ui, &selections, &armature, events);
                }
            });
            shared_ui.bone_panel_rect = Some(ui.min_rect());
        }),
        events,
        context,
    );

    // adjust bar positions
    let bone_panel = shared_ui.bone_panel_rect.unwrap();
    let top_panel = shared_ui.top_panel_rect.unwrap();
    let armature_panel = shared_ui.armature_panel_rect.unwrap();
    let keyframe_panel = shared_ui.keyframe_panel_rect;
    match config.layout {
        UiLayout::Split => {
            shared_ui.anim_bar.pos.x = bone_panel.left() - shared_ui.anim_bar.scale.x - 21.;
            shared_ui.anim_bar.pos.y = top_panel.bottom() - 1.;

            shared_ui.edit_bar.pos = Vec2::new(armature_panel.right(), top_panel.bottom());

            shared_ui.camera_bar.pos.x =
                bone_panel.left() - shared_ui.camera_bar.scale.x - ((6. * 3.3) as f32).ceil();
            if keyframe_panel != None && edit_mode.anim_open {
                shared_ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared_ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared_ui.camera_bar.pos.y -= shared_ui.camera_bar.scale.y + 15.;
        }
        UiLayout::Right => {
            shared_ui.edit_bar.pos.x = bone_panel.left() - shared_ui.edit_bar.scale.x - 28.;
            shared_ui.edit_bar.pos.y = top_panel.bottom();

            shared_ui.anim_bar.pos = Vec2::new(0., top_panel.bottom());

            shared_ui.camera_bar.pos.x = bone_panel.left() - shared_ui.camera_bar.scale.x - 21.;
            if keyframe_panel != None && edit_mode.anim_open {
                shared_ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared_ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared_ui.camera_bar.pos.y -= shared_ui.camera_bar.scale.y + 15.;
        }
        UiLayout::Left => {
            shared_ui.edit_bar.pos.x = bone_panel.right();
            shared_ui.edit_bar.pos.y = top_panel.bottom();

            shared_ui.anim_bar.pos.x = context.content_rect().right() - shared_ui.anim_bar.scale.x;
            shared_ui.anim_bar.pos.y = top_panel.bottom();

            shared_ui.camera_bar.pos.x = bone_panel.right() + 7.;
            if keyframe_panel != None && edit_mode.anim_open {
                shared_ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared_ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared_ui.camera_bar.pos.y -= shared_ui.camera_bar.scale.y + 15.;
        }
    }

    if selections.bone_idx != usize::MAX {
        edit_mode_bar(context, armature, selections, edit_mode, events, shared_ui);
    }

    if armature.bones.len() > 0 {
        animate_bar(context, shared_ui, edit_mode, events);
    }

    // check if mouse is on ui
    //
    // this check always returns false on mouse click, so it's only checked when the mouse isn't clicked
    if !input.left_down {
        events.toggle_pointer_on_ui(context.is_pointer_over_area());
    }

    // close all context menus if clicking outside of them
    if input.left_clicked && !shared_ui.context_menu.keep {
        shared_ui.context_menu.close();
    }

    macro_rules! helper_text {
        ($text:expr, $offset:expr) => {
            let align = egui::Align2::CENTER_CENTER;
            let font = egui::FontId::default();
            let mouse_pos = input.mouse / shared_ui.scale + $offset;
            let painter = context.debug_painter();

            let pos = (mouse_pos + Vec2::new(1., 1.)).into();
            painter.text(pos, align, $text, font.clone(), egui::Color32::BLACK);

            let point_col = config.colors.center_point.into();
            painter.text(mouse_pos.into(), align, $text, font, point_col);
        };
    }

    if armature.sel_bone(&sel) == None {
        return;
    }

    if edit_mode.is_rotating {
        let offset = Vec2::new(50., 0.);
        let rot = selected_bone.rot / 3.14 * 180.;
        let formatted = (rot * 100.).round() / 100.;
        helper_text!(formatted.to_string() + "°", offset);
    }
    if edit_mode.is_scaling {
        let offset = Vec2::new(50., 0.);
        let formatted = (selected_bone.scale.x * 100.).round() / 100.;
        let mut padding = "";
        if formatted.to_string() == "1" {
            padding = ".00";
        }
        let helper_str = "⏵ w: ".to_owned() + &formatted.to_string() + padding;
        helper_text!(helper_str.to_string(), offset);

        let offset = Vec2::new(-1., -38.);
        let formatted = (selected_bone.scale.y * 100.).round() / 100.;
        let mut padding = "";
        if formatted.to_string() == "1" {
            padding = ".00";
        }
        let helper_str = "h: ".to_owned() + &formatted.to_string() + padding + "\n     ⏶";
        helper_text!(helper_str.to_string(), offset);
    }
}

pub fn process_inputs(
    context: &Context,
    input: &mut InputStates,
    shared_ui: &mut crate::Ui,
    config: &Config,
    selections: &SelectionState,
    edit_mode: &mut EditMode,
    events: &mut EventState,
    camera: &Camera,
    armature: &Armature,
) {
    shared_ui.last_pressed = None;

    context.input_mut(|i| {
        input.holding_mod = i.modifiers.command;
        input.holding_shift = i.modifiers.shift;
        if shared_ui.rename_id == "" {
            kb_inputs(
                i, shared_ui, events, config, selections, edit_mode, armature, camera,
            );
        }
        shared_ui.last_pressed = i.keys_down.iter().last().copied();

        input.left_clicked = i.pointer.primary_clicked();
        input.right_clicked = i.pointer.secondary_clicked();
        input.left_down = i.pointer.primary_down();
        input.left_pressed = i.pointer.primary_pressed();
        input.right_down = i.pointer.secondary_down();
        if input.left_pressed {
            input.mouse_init = Some(input.mouse);
        }
        if i.pointer.primary_released() {
            input.mouse_init = None;
        }
        if input.left_down {
            input.down_dur += 1;
        } else {
            input.down_dur = -1;
        }
        if shared_ui.mobile {
            input.left_clicked = i.pointer.any_pressed();
        }
        input.mouse_prev = input.mouse;
        if let Some(mouse) = i.pointer.latest_pos() {
            input.mouse = mouse.into();
            input.mouse *= shared_ui.scale;
        }

        // don't record prev mouse on first frame of touch as it
        // goes all over the place
        if i.any_touches() && i.pointer.primary_pressed() {
            input.mouse_prev = input.mouse;
        }

        if i.smooth_scroll_delta.y != 0. && !camera.on_ui {
            input.scroll_delta = i.smooth_scroll_delta.y;
            events.cam_zoom_scroll();
        }

        edit_mode.time = i.time as f32;
    });
}

pub fn kb_inputs(
    input: &mut egui::InputState,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
    config: &Config,
    selections: &SelectionState,
    edit_mode: &EditMode,
    #[allow(unused_variables)] armature: &Armature,
    #[allow(unused_variables)] camera: &Camera,
) {
    if shared_ui.startup_window {
        return;
    }
    mouse_button_as_key(input, egui::PointerButton::Primary, egui::Key::F31);
    mouse_button_as_key(input, egui::PointerButton::Secondary, egui::Key::F32);
    mouse_button_as_key(input, egui::PointerButton::Middle, egui::Key::F33);
    mouse_button_as_key(input, egui::PointerButton::Extra1, egui::Key::F34);
    mouse_button_as_key(input, egui::PointerButton::Extra2, egui::Key::F35);

    if input.consume_shortcut(&config.keys.undo) {
        events.undo();
    }
    if input.consume_shortcut(&config.keys.redo) {
        events.redo();
    }

    if input.consume_shortcut(&config.keys.zoom_in_camera) {
        events.cam_zoom_in();
    }
    if input.consume_shortcut(&config.keys.zoom_out_camera) {
        events.cam_zoom_out();
    }

    if input.consume_shortcut(&config.keys.save) {
        #[cfg(target_arch = "wasm32")]
        utils::save_web(armature, camera, edit_mode, false);
        #[cfg(not(target_arch = "wasm32"))]
        utils::save_native(shared_ui);
    }

    if input.consume_shortcut(&config.keys.export) {
        shared_ui.export_modal = true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    if input.consume_shortcut(&config.keys.save_as) {
        shared_ui.save_path = None;
        utils::save_native(shared_ui);
    }

    if input.consume_shortcut(&config.keys.open) {
        #[cfg(not(target_arch = "wasm32"))]
        utils::open_import_dialog(&shared_ui.file_path, &shared_ui.file_type);
        #[cfg(target_arch = "wasm32")]
        crate::clickFileInput(false);
    }

    // copy shortcut
    if input.consume_shortcut(&config.keys.copy) {
        // copy bone(s)
        let idx = selections.bone_idx;
        if idx != usize::MAX {
            events.copy_bone(idx);
        }
    }

    // paste shortcut
    if input.consume_shortcut(&config.keys.paste) {
        events.paste_bone(selections.bone_idx);
    }

    if input.consume_shortcut(&config.keys.cancel) {
        let no_modals = !shared_ui.styles_modal
            && !shared_ui.modal
            && !shared_ui.polar_modal
            && !shared_ui.forced_modal
            && !shared_ui.settings_modal
            && !shared_ui.export_modal
            && !shared_ui.warnings_open;

        shared_ui.styles_modal = false;
        shared_ui.modal = false;
        shared_ui.polar_modal = false;
        shared_ui.forced_modal = false;
        shared_ui.settings_modal = false;
        shared_ui.atlas_modal = false;
        shared_ui.export_modal = false;
        shared_ui.warnings_open = false;

        // if a context menu is open, cancel that instead
        if shared_ui.context_menu.id != "" {
            shared_ui.context_menu.id = "".to_string();
            return;
        }

        if no_modals && !edit_mode.setting_ik_target {
            events.unselect_all();
        }

        #[cfg(target_arch = "wasm32")]
        {
            toggleElement(false, "image-dialog".to_string());
            toggleElement(false, "file-dialog".to_string());
        }

        events.toggle_setting_ik_target(0);
    }
}

pub fn mouse_button_as_key(
    input: &mut egui::InputState,
    button: egui::PointerButton,
    fake_key: egui::Key,
) {
    if !input.pointer.button_down(button) {
        input.keys_down.remove(&fake_key);
        return;
    }

    if input.pointer.button_pressed(button) {
        input.keys_down.insert(fake_key);
        input.events.push(egui::Event::Key {
            key: fake_key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    }

    if input.pointer.button_released(button) {
        input.keys_down.remove(&fake_key);
        input.events.push(egui::Event::Key {
            key: fake_key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
    }
}

fn top_panel(
    egui_ctx: &Context,
    config: &Config,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
    selections: &SelectionState,
    armature: &Armature,
    camera: &Camera,
    edit_mode: &EditMode,
) {
    let panel = egui::TopBottomPanel::top("top_bar").frame(egui::Frame {
        fill: config.colors.main.into(),
        stroke: Stroke::new(0., config.colors.main),
        inner_margin: egui::Margin::default(),
        outer_margin: egui::Margin::default(),
        ..Default::default()
    });
    panel.show(egui_ctx, |ui| {
        ui.set_max_height(20.);
        let mut offset = 0.;
        if shared_ui.startup_window {
            shared_ui.top_panel_rect = Some(ui.min_rect());
            return;
        }
        egui::MenuBar::new().ui(ui, |ui| {
            #[rustfmt::skip]
            menu_file_button(ui, &config, shared_ui, events, &selections, &armature, edit_mode, &camera);
            menu_edit_button(ui, &config, &shared_ui, events);
            menu_view_button(ui, &config, &shared_ui, events);

            macro_rules! title {
                ($title:expr) => {
                    egui::RichText::new($title).color(config.colors.text)
                };
            }

            let str_settings = title!(&shared_ui.loc("top_bar.settings"));
            let button = ui.menu_button(str_settings, |ui| ui.close());
            if button.response.clicked() {
                shared_ui.settings_modal = true;
            }

            let s_ui = &shared_ui;

            ui.menu_button(title!(&shared_ui.loc("top_bar.help.heading")), |ui| {
                ui.set_width(90.);
                //let str_user_docs = &shared.ui.loc("top_bar.help.user_docs");
                let str_user_docs = &shared_ui.loc("top_bar.help.user_docs");
                if top_bar_button(ui, str_user_docs, None, &mut offset, config, s_ui).clicked() {
                    utils::open_docs(true, "");
                }
                let str_dev_docs = &shared_ui.loc("top_bar.help.dev_docs");
                if top_bar_button(ui, str_dev_docs, None, &mut offset, config, s_ui).clicked() {
                    utils::open_docs(false, "");
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let str_binary = &shared_ui.loc("top_bar.help.binary_folder");
                    if top_bar_button(ui, str_binary, None, &mut offset, config, s_ui).clicked() {
                        match open::that(utils::bin_path()) {
                            Err(_) => {}
                            Ok(file) => file,
                        };
                    }
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let str_config = &shared_ui.loc("top_bar.help.config_folder");
                    if top_bar_button(ui, str_config, None, &mut offset, config, s_ui).clicked() {
                        match open::that(config_path().parent().unwrap()) {
                            Err(_) => {}
                            Ok(file) => file,
                        };
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if shared_ui.warnings.len() == 0 {
                        return;
                    }
                    ui.add_space(10.);
                    let count = egui::RichText::new(shared_ui.warnings.len().to_string() + " ⚠")
                        .color(config.colors.warning_text);
                    let pointing_hand = egui::CursorIcon::PointingHand;
                    let header = ui
                        .add(egui::Button::selectable(false, count))
                        .on_hover_cursor(pointing_hand);
                    if header.clicked() {
                        shared_ui.warnings_open = !shared_ui.warnings_open;
                    }
                    if !shared_ui.warnings_open {
                        return;
                    }
                    let bg = egui::LayerId::background();
                    let popup = egui::Popup::new("warnings".into(), ui.ctx().clone(), &header, bg);
                    popup.show(|ui| {
                        ui.set_width(350.);
                        ui.heading(shared_ui.loc("warnings.heading"))
                            .on_hover_text(shared_ui.loc("warnings.desc"));
                        modal_x(ui, [0., 0.].into(), || {
                            shared_ui.warnings_open = false;
                        });
                        ui.add_space(5.);
                        for w in 0..shared_ui.warnings.len() {
                            let warning = shared_ui.warnings[w].clone();

                            egui::Frame::new().show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.set_height(21.);
                                    ui.add_space(5.);
                                    warnings::warning_line(
                                        ui, &warning, shared_ui, &armature, config, events,
                                    );
                                });
                            });

                            if w != shared_ui.warnings.len() - 1 {
                                ui.separator();
                            }
                        }
                    });
                })
            })
        });

        shared_ui.top_panel_rect = Some(ui.min_rect());
    });
}

impl EguiUi for egui::Ui {
    fn skf_button(&mut self, text: &str) -> egui::Response {
        let text = egui::RichText::new(text);
        self.add(egui::Button::new(text).corner_radius(egui::CornerRadius::ZERO))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn gradient(&mut self, rect: egui::Rect, top: Color32, bottom: Color32) {
        let mut mesh = egui::Mesh::default();

        mesh.colored_vertex(rect.left_top(), top);
        mesh.colored_vertex(rect.right_top(), top);
        mesh.colored_vertex(rect.left_bottom(), bottom);
        mesh.colored_vertex(rect.right_bottom(), bottom);

        mesh.add_triangle(0, 2, 3);
        mesh.add_triangle(0, 3, 1);

        self.painter().add(egui::Shape::mesh(mesh));
    }

    fn clickable_label(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response {
        let hand = egui::CursorIcon::PointingHand;
        let label = self
            .add(egui::Button::selectable(false, text))
            .on_hover_cursor(hand);

        if label.contains_pointer() || label.has_focus() {
            return label.highlight();
        }

        label
    }

    fn context_button(
        &mut self,
        text: impl Into<egui::WidgetText>,
        config: &Config,
    ) -> egui::Response {
        let button = self
            .allocate_ui([0., 0.].into(), |ui| {
                ui.set_width(70.);
                ui.set_height(20.);
                let width = ui.available_width();
                let mut col = config.colors.main;
                if ui.ui_contains_pointer() {
                    col = config.colors.light_accent;
                }
                egui::Frame::new().fill(col.into()).show(ui, |ui| {
                    ui.style_mut().interaction.selectable_labels = false;
                    ui.set_width(width);
                    ui.set_height(20.);
                    ui.horizontal(|ui| {
                        ui.add_space(5.);
                        ui.label(text)
                    });
                });
            })
            .response
            .interact(egui::Sense::click());

        button
    }

    fn text_input(
        &mut self,
        id: String,
        shared_ui: &mut crate::Ui,
        mut value: String,
        mut options: Option<TextInputOptions>,
    ) -> (bool, String, egui::Response) {
        let input: egui::Response;

        if options == None {
            options = Some(TextInputOptions::default());
        }

        if options.as_ref().unwrap().size == Vec2::ZERO {
            options.as_mut().unwrap().size = Vec2::new(self.available_width(), 20.);
        }

        if options.as_ref().unwrap().focus && !shared_ui.input_focused {
            shared_ui.input_focused = true;
            shared_ui.edit_value = Some(value.clone());

            if shared_ui.mobile {
                open_mobile_input(shared_ui.edit_value.clone().unwrap());
            }
        }

        if shared_ui.rename_id != id {
            input = self.add_sized(
                options.as_ref().unwrap().size,
                egui::TextEdit::singleline(&mut value)
                    .hint_text(options.as_ref().unwrap().placeholder.clone()),
            );
            // extract value as a string and store it with edit_value
            if input.has_focus() {
                shared_ui.edit_value = Some(value.clone());
                shared_ui.rename_id = id.to_string();
                if shared_ui.mobile {
                    open_mobile_input(shared_ui.edit_value.clone().unwrap());
                }
            }
        } else {
            let singleline = egui::TextEdit::singleline(shared_ui.edit_value.as_mut().unwrap())
                .hint_text(options.as_ref().unwrap().placeholder.clone());
            input = self.add_sized(options.as_ref().unwrap().size, singleline);

            let mut entered = false;

            // if input modal is closed, consider the value entered
            if shared_ui.mobile {
                #[cfg(target_arch = "wasm32")]
                {
                    shared_ui.edit_value = Some(getEditInput());
                    if !isModalActive("edit-input-modal".to_string()) {
                        entered = true;
                    }
                }
            }

            if self.input(|i| i.key_pressed(egui::Key::Escape)) {
                shared_ui.input_focused = false;
                shared_ui.rename_id = "".to_string();
                return (false, value, input);
            }

            if self.input(|i| i.key_pressed(egui::Key::Enter)) || input.lost_focus() {
                entered = true;
            }

            let mut final_value = shared_ui.edit_value.as_ref().unwrap();
            if final_value == "" {
                final_value = &options.as_ref().unwrap().default;
            }

            if entered {
                shared_ui.input_focused = false;
                shared_ui.rename_id = "".to_string();
                return (true, final_value.clone(), input);
            }

            if input.lost_focus() {
                shared_ui.rename_id = "".to_string();
            }
        }

        if options.as_ref().unwrap().focus {
            input.request_focus();
        }

        (false, value, input)
    }

    // helper for editable float inputs
    fn float_input(
        &mut self,
        id: String,
        shared_ui: &mut crate::Ui,
        value: f32,
        modifier: f32,
        mut options: Option<TextInputOptions>,
    ) -> (bool, f32, egui::Response) {
        if options == None {
            options = Some(TextInputOptions {
                size: Vec2::new(40., 20.),
                ..Default::default()
            })
        }

        let (edited, _, input) =
            self.text_input(id, shared_ui, (value * modifier).to_string(), options);

        if edited {
            shared_ui.rename_id = "".to_string();
            if shared_ui.edit_value.as_mut().unwrap() == "" {
                shared_ui.edit_value = Some("0".to_string());
            }
            match shared_ui.edit_value.as_mut().unwrap().parse::<f32>() {
                Ok(output) => return (true, output / modifier, input),
                Err(_) => return (false, value, input),
            }
        }

        (false, value, input)
    }

    fn debug_rect(&mut self, rect: egui::Rect) {
        self.painter().rect_stroke(
            rect,
            egui::CornerRadius::ZERO,
            egui::Stroke::new(1., egui::Color32::RED),
            egui::StrokeKind::Outside,
        );
    }

    fn context_rename(&mut self, shared_ui: &mut crate::Ui, config: &Config, id: String) {
        if self
            .context_button(shared_ui.loc("rename"), config)
            .clicked()
        {
            shared_ui.rename_id = id;
            shared_ui.context_menu.close();
        };
    }

    fn context_delete(
        &mut self,
        shared_ui: &mut crate::Ui,
        config: &Config,
        events: &mut EventState,
        loc_code: &str,
        polar_id: PolarId,
    ) {
        if self
            .context_button(shared_ui.loc("delete"), config)
            .clicked()
        {
            let str_del = &shared_ui.loc(&("polar.".to_owned() + &loc_code)).clone();
            events.open_polar_modal(polar_id, str_del.to_string());

            // only hide the menu, as anim id is still needed for modal
            shared_ui.context_menu.hide = true;
        }
    }
}

pub fn create_ui_texture(
    bytes: Vec<u8>,
    has_alpha: bool,
    ctx: &Context,
    name: &str,
) -> Option<egui::TextureHandle> {
    let thumb_img;
    if let Ok(data) = image::load_from_memory(&bytes) {
        thumb_img = data;
    } else {
        return None;
    }

    let (size, pixels) = if has_alpha {
        let rgba = thumb_img.into_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        (size, rgba.into_raw())
    } else {
        let rgb = thumb_img.into_rgb8();
        let size = [rgb.width() as usize, rgb.height() as usize];
        (size, rgb.into_raw())
    };

    let color_image = if has_alpha {
        egui::ColorImage::from_rgba_unmultiplied(size, &pixels)
    } else {
        egui::ColorImage::from_rgb(size, &pixels)
    };

    let ui_tex = ctx.load_texture(name, color_image, Default::default());

    Some(ui_tex)
}

fn menu_file_button(
    ui: &mut egui::Ui,
    config: &Config,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
    selections: &SelectionState,
    armature: &Armature,
    #[allow(unused_variables)] edit_mode: &EditMode,
    #[allow(unused_variables)] camera: &Camera,
) {
    let mut offset = 0.;
    let title =
        egui::RichText::new(&shared_ui.loc("top_bar.file.heading")).color(config.colors.text);
    ui.menu_button(title, |ui| {
        ui.set_width(125.);

        macro_rules! top_bar_button {
            ($name:expr, $kb:expr) => {
                top_bar_button(ui, $name, $kb, &mut offset, &config, &shared_ui)
            };
        }

        let str_open = &shared_ui.loc("top_bar.file.open");
        if top_bar_button!(str_open, Some(&config.keys.open)).clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_import_dialog(&shared_ui.file_path, &shared_ui.file_type);
            #[cfg(target_arch = "wasm32")]
            crate::clickFileInput(false);
            ui.close();
        }
        let str_save = &shared_ui.loc("top_bar.file.save");
        if top_bar_button!(str_save, Some(&config.keys.save)).clicked() {
            #[cfg(target_arch = "wasm32")]
            utils::save_web(armature, camera, edit_mode, false);
            #[cfg(not(target_arch = "wasm32"))]
            utils::save_native(shared_ui);
            ui.close();
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let str_save_as = &shared_ui.loc("top_bar.file.save_as");
            if top_bar_button!(str_save_as, Some(&config.keys.save_as)).clicked() {
                shared_ui.save_path = None;
                utils::save_native(shared_ui);
                ui.close();
            }
        }
        let str_export = &shared_ui.loc("top_bar.file.export");
        if top_bar_button!(str_export, Some(&config.keys.export)).clicked() {
            shared_ui.export_modal = true;
            ui.close();
        }
        let str_startup = &shared_ui.loc("top_bar.file.startup");
        if top_bar_button!(str_startup, None).clicked() {
            shared_ui.startup_window = true;
            ui.close();
        }
        // disabled: export video is unstable and not really necessary (focus on sprite export instead!)
        if false && top_bar_button!("Export Video", None).clicked() {
            // check if ffmpeg exists and complain if it doesn't
            let mut ffmpeg = false;
            match std::process::Command::new("ffmpeg")
                .arg("-version")
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        ffmpeg = true;
                    } else {
                        println!("ffmpeg command ran but returned an error:");
                    }
                }
                Err(e) => {
                    println!("Failed to run ffmpeg: {}", e);
                    println!("Make sure ffmpeg is installed and in your $PATH.");
                }
            }
            if !ffmpeg {
                events.open_modal("startup.error_ffmpeg", false);
                return;
            }

            // complain if there's no proper animation to export
            let sel = selections.clone();
            if selections.anim == usize::MAX {
                let anims = &armature.animations;
                if anims.len() == 0 || anims[0].keyframes.len() == 0 {
                    events.open_modal("No animation available.", false);
                    return;
                } else {
                    //selections.anim = 0;
                }
            } else if armature.sel_anim(&sel).unwrap().keyframes.last() == None {
                events.open_modal("No animation available.", false);
                return;
            }

            //edit_mode.recording = true;
            //edit_mode.anim_open = true;
            //edit_mode.done_recording = true;
            events.select_anim_frame(0, false);
            shared_ui.anim.loops = 1;
            ui.close();
        }
    });
}

fn menu_view_button(
    ui: &mut egui::Ui,
    config: &Config,
    shared_ui: &crate::Ui,
    events: &mut EventState,
) {
    let mut offset = 0.;

    let str_view = &shared_ui.loc("top_bar.view.heading");
    let title = egui::RichText::new(str_view).color(config.colors.text);
    ui.menu_button(title, |ui| {
        macro_rules! tpb {
            ($name:expr, $kb:expr) => {
                top_bar_button(ui, $name, $kb, &mut offset, &config, &shared_ui)
            };
        }

        ui.set_width(125.);
        let str_zoom_in = &shared_ui.loc("top_bar.view.zoom_in");
        if tpb!(str_zoom_in, Some(&config.keys.zoom_in_camera)).clicked() {
            events.cam_zoom_in();
        }
        let str_zoom_out = &shared_ui.loc("top_bar.view.zoom_out");
        if tpb!(str_zoom_out, Some(&config.keys.zoom_out_camera)).clicked() {
            events.cam_zoom_out();
        }
    });
}

fn menu_edit_button(
    ui: &mut egui::Ui,
    config: &Config,
    shared_ui: &crate::Ui,
    events: &mut EventState,
) {
    let mut offset = 0.;
    let str_edit = &shared_ui.loc("top_bar.edit.heading");
    let title = egui::RichText::new(str_edit).color(config.colors.text);
    ui.menu_button(title, |ui| {
        ui.set_width(90.);
        let key_undo = Some(&config.keys.undo);
        let str_undo = &shared_ui.loc("top_bar.edit.undo");
        if top_bar_button(ui, str_undo, key_undo, &mut offset, &config, &shared_ui).clicked() {
            events.undo();
            ui.close();
        }
        let str_redo = &shared_ui.loc("top_bar.edit.redo");
        let key_redo = Some(&config.keys.redo);
        if top_bar_button(ui, str_redo, key_redo, &mut offset, &config, &shared_ui).clicked() {
            events.redo();
            ui.close();
        }
    });
}

fn edit_mode_bar(
    egui_ctx: &egui::Context,
    armature: &Armature,
    selections: &SelectionState,
    edit_mode: &EditMode,
    events: &mut EventState,
    shared_ui: &mut crate::Ui,
) {
    let mut ik_disabled = true;
    let mut is_end = false;
    let sel = selections.clone();
    if let Some(bone) = armature.sel_bone(&sel) {
        ik_disabled = bone.ik_disabled || armature.bone_eff(bone.id) == JointEffector::None;
        is_end = armature.bone_eff(bone.id) == JointEffector::End;
    }

    // edit mode window
    let window = egui::Window::new("Mode")
        .resizable(false)
        .title_bar(false)
        .max_width(100.)
        .movable(false)
        .current_pos(egui::Pos2::new(
            shared_ui.edit_bar.pos.x + 7.5,
            shared_ui.edit_bar.pos.y - 1.,
        ));
    window.show(egui_ctx, |ui| {
        ui.horizontal(|ui| {
            macro_rules! edit_mode_button {
                ($label:expr, $edit_mode:expr, $event:ident, $check:expr) => {
                    ui.add_enabled_ui($check, |ui| {
                        if selection_button($label, edit_mode.current == $edit_mode, ui).clicked() {
                            events.$event()
                        };
                    })
                };
            }
            let ik_disabled = !edit_mode.showing_mesh && ik_disabled;
            let rot = ik_disabled || is_end;
            edit_mode_button!(
                &shared_ui.loc("move"),
                EditModes::Move,
                edit_mode_move,
                ik_disabled
            );
            edit_mode_button!(
                &shared_ui.loc("rotate"),
                EditModes::Rotate,
                edit_mode_rotate,
                rot
            );
            edit_mode_button!(
                &shared_ui.loc("scale"),
                EditModes::Scale,
                edit_mode_scale,
                ik_disabled
            );
        });
        shared_ui.edit_bar.scale = ui.min_rect().size().into();
    });
}

fn animate_bar(
    egui_ctx: &Context,
    shared_ui: &mut crate::Ui,
    edit_mode: &EditMode,
    events: &mut EventState,
) {
    let window = egui::Window::new("Animating")
        .resizable(false)
        .title_bar(false)
        .max_width(100.)
        .movable(false)
        .current_pos(egui::Pos2::new(
            shared_ui.anim_bar.pos.x,
            shared_ui.anim_bar.pos.y,
        ));
    window.show(egui_ctx, |ui| {
        ui.horizontal(|ui| {
            let str_armature = &shared_ui.loc("armature_panel.heading");
            if selection_button(str_armature, !edit_mode.anim_open, ui).clicked() {
                events.toggle_anim_panel_open(0);
            }
            let str_animation = &shared_ui.loc("keyframe_editor.heading");
            if selection_button(str_animation, edit_mode.anim_open, ui).clicked() {
                events.toggle_anim_panel_open(1);
            }
            shared_ui.anim_bar.scale = ui.min_rect().size().into();
        });
        shared_ui.anim_bar.scale = ui.min_rect().size().into();
    });
}

fn camera_bar(
    egui_ctx: &Context,
    config: &Config,
    shared_ui: &mut crate::Ui,
    camera: &Camera,
    events: &mut EventState,
) {
    let margin = 6.;
    let window = egui::Window::new("Camera")
        .resizable(false)
        .title_bar(false)
        .max_width(60.)
        .max_height(25.)
        .movable(false)
        .frame(egui::Frame {
            fill: config.colors.gradient.into(),
            inner_margin: margin.into(),
            stroke: Stroke {
                width: 1.,
                color: config.colors.dark_accent.into(),
            },
            ..Default::default()
        })
        .current_pos(egui::Pos2::new(
            shared_ui.camera_bar.pos.x,
            shared_ui.camera_bar.pos.y,
        ));
    window.show(egui_ctx, |ui| {
        macro_rules! input {
            ($float:expr, $id:expr, $label:expr, $tip:expr) => {
                ui.horizontal(|ui| {
                    ui.label($label).on_hover_text(&shared_ui.loc($tip));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let id = $id.to_string();
                        let (edited, value, _) =
                            ui.float_input(id, shared_ui, $float.round(), 1., None);
                        if edited {
                            let cam = &camera;
                            if $id.to_string() == "cam_pos_x" {
                                events.edit_camera(value, cam.pos.y, cam.zoom);
                            } else if $id.to_string() == "cam_pos_y" {
                                events.edit_camera(cam.pos.x, value, cam.zoom);
                            } else if $id.to_string() == "cam_zoom" {
                                events.edit_camera(cam.pos.x, cam.pos.y, value);
                            }
                        }
                    })
                })
            };
        }

        input!(camera.pos.x, "cam_pos_x", "X", "cam_x");
        input!(camera.pos.y, "cam_pos_y", "Y", "cam_y");
        input!(camera.zoom, "cam_zoom", "🔍", "cam_zoom");

        shared_ui.camera_bar.scale = ui.min_rect().size().into();
    });
}

/// Default styling to apply across all UI.
pub fn default_styling(context: &Context, config: &Config) {
    let mut visuals = egui::Visuals::dark();
    let colors = &config.colors;

    visuals.menu_corner_radius = egui::CornerRadius::ZERO;

    // remove rounded corners on windows
    visuals.window_corner_radius = egui::CornerRadius::ZERO;

    visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.inactive.weak_bg_fill = colors.light_accent.into();
    visuals.widgets.inactive.bg_fill = colors.dark_accent.into();

    visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.hovered.weak_bg_fill = colors.light_accent.into();
    visuals.widgets.hovered.bg_fill = colors.dark_accent.into();

    visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.active.weak_bg_fill = colors.light_accent.into();
    visuals.widgets.active.bg_fill = colors.dark_accent.into();

    visuals.widgets.open.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.open.weak_bg_fill = colors.dark_accent.into();
    visuals.widgets.open.bg_fill = colors.dark_accent.into();
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1., colors.dark_accent);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1., colors.dark_accent);

    visuals.window_shadow = Shadow::NONE;
    visuals.window_fill = colors.main.into();
    visuals.panel_fill = colors.main.into();
    visuals.window_stroke = egui::Stroke::new(1., colors.dark_accent);

    visuals.widgets.noninteractive.bg_fill = colors.text.into();
    visuals.widgets.noninteractive.weak_bg_fill = colors.text.into();
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1., colors.text);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1., colors.dark_accent);

    visuals.selection.bg_fill = colors.light_accent.into();
    visuals.selection.stroke = egui::Stroke::new(1., colors.text);

    //visuals.override_text_color = Some(colors.text.into());
    //let mut col = colors.text;
    //col -= Color::new(100, 100, 100, 0);
    //visuals.weak_text_color = Some(col.into());

    visuals.hyperlink_color = colors.link.into();

    context.set_visuals(visuals);
}

pub fn selection_button(text: &str, selected: bool, ui: &mut egui::Ui) -> egui::Response {
    let mut cursor = egui::CursorIcon::PointingHand;
    let mut bg_col = ui.visuals().widgets.active.weak_bg_fill;

    if selected {
        cursor = egui::CursorIcon::Default;
        bg_col = bg_col + egui::Color32::from_rgb(20, 20, 20);
    }

    let button = egui::Button::new(egui::RichText::new(text))
        .fill(bg_col)
        .corner_radius(egui::CornerRadius::ZERO);

    ui.add(button).on_hover_cursor(cursor)
}

pub fn job_text(str: &str, color: Option<Color32>, job: &mut egui::text::LayoutJob) {
    let mut format = egui::TextFormat::default();
    if color != None {
        format.color = color.unwrap();
    }
    job.append(&str.to_string(), 0.0, format)
}

pub fn top_bar_button(
    ui: &mut egui::Ui,
    text: &str,
    key: Option<&egui::KeyboardShortcut>,
    offset: &mut f32,
    config: &Config,
    shared_ui: &crate::Ui,
) -> egui::Response {
    let height = 20.;

    let rect = egui::Rect::from_min_size(
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset),
        egui::Vec2::new(ui.min_rect().width(), height),
    );
    let response: egui::Response = ui.allocate_rect(rect, egui::Sense::click());
    let painter = ui.painter_at(ui.min_rect());

    let col = if response.hovered() {
        config.colors.light_accent.into()
    } else {
        egui::Color32::TRANSPARENT
    };
    painter.rect_filled(rect, egui::CornerRadius::ZERO, col);

    let font = egui::FontId::new(13., egui::FontFamily::Proportional);

    // text
    let pos =
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset) + egui::vec2(5., 2.);
    let text_col = config.colors.text.into();
    painter.text(pos, egui::Align2::LEFT_TOP, text, font.clone(), text_col);

    let key_str = if key != None {
        key.unwrap().display()
    } else {
        "".to_string()
    };

    // kb key text
    if !shared_ui.mobile {
        let pos = egui::Pos2::new(ui.min_rect().right(), ui.min_rect().top() + *offset)
            + egui::vec2(-5., 2.5);
        let align = egui::Align2::RIGHT_TOP;
        painter.text(pos, align, key_str, font.clone(), egui::Color32::DARK_GRAY);
    }

    // set next button's Y to below this one
    *offset += height + 2.;

    response
}

pub fn draw_fading_rect(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    color: Color32,
    max_alpha: f32,
    time: f64,
) {
    let time = ui.ctx().input(|i| i.time / time);
    let fade = ((time * 3.14).sin() * 0.5 + 0.5) as f32;

    let fade_color =
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), (fade * max_alpha) as u8);
    ui.painter().rect_filled(rect, 0., fade_color);
}

#[derive(PartialEq)]
pub struct TextInputOptions {
    pub size: Vec2,
    pub focus: bool,
    pub placeholder: String,
    pub default: String,
}

impl Default for TextInputOptions {
    fn default() -> Self {
        TextInputOptions {
            size: Vec2::new(0., 0.),
            focus: false,
            placeholder: "".to_string(),
            default: "".to_string(),
        }
    }
}

fn open_mobile_input(_value: String) {
    #[cfg(target_arch = "wasm32")]
    {
        crate::setEditInput(_value);
        crate::toggleElement(true, "edit-input-modal".to_string());
        crate::focusEditInput();
    }
}

// Wrapper for resizable panels.
// Handles toggling on_ui if resizing the panel itself.
pub fn draw_resizable_panel<T>(
    id: &str,
    panel: egui::InnerResponse<T>,
    events: &mut EventState,
    context: &egui::Context,
) {
    if let Some(resize) = context.read_response(egui::Id::new(id).with("__resize")) {
        if resize.hovered() || panel.response.hovered() {
            events.toggle_pointer_on_ui(true);
        }
    }
}
