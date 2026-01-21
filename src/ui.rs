//! Core user interface (UI) logic.
use std::collections::HashMap;

use egui::{Color32, Context, Shadow, Stroke};

use crate::*;

#[rustfmt::skip]
pub trait EguiUi {
    fn skf_button(&mut self, text: &str) -> egui::Response;
    fn gradient(&mut self, rect: egui::Rect, top: Color32, bottom: Color32);
    fn clickable_label(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response;
    fn text_input(&mut self,id: String, shared_ui: &mut crate::Ui, value: String, options: Option<TextInputOptions>) -> (bool, String, egui::Response);
    fn float_input(&mut self,id: String,shared: &mut crate::Ui,value: f32,modifier: f32,options: Option<TextInputOptions>) -> (bool, f32, egui::Response);
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
pub fn draw(context: &Context, shared: &mut Shared) {
    shared.input.last_pressed = None;
    shared.ui.context_menu.keep = false;

    context.input_mut(|i| {
        if shared.ui.rename_id == "" {
            kb_inputs(i, shared);
        }
        shared.input.last_pressed = i.keys_down.iter().last().copied();

        shared.input.left_clicked = i.pointer.primary_clicked();
        shared.input.right_clicked = i.pointer.secondary_clicked();
        shared.input.left_down = i.pointer.primary_down();
        shared.input.left_pressed = i.pointer.primary_pressed();
        shared.input.right_down = i.pointer.secondary_down();
        if shared.input.left_down {
            shared.input.down_dur += 1;
        } else {
            shared.input.down_dur = -1;
        }
        if shared.ui.mobile {
            shared.input.left_clicked = i.pointer.any_pressed();
        }
        shared.input.mouse_prev = shared.input.mouse;
        if let Some(mouse) = i.pointer.latest_pos() {
            shared.input.mouse = mouse.into();
            shared.input.mouse *= shared.ui.scale;
        }

        // don't record prev mouse on first frame of touch as it
        // goes all over the place
        if i.any_touches() && i.pointer.primary_pressed() {
            shared.input.mouse_prev = shared.input.mouse;
        }

        if i.smooth_scroll_delta.y != 0. && !shared.camera.on_ui {
            shared.events.new(Events::CamZoomScroll);
            shared.input.scroll_delta = i.smooth_scroll_delta.y;
            match shared.config.layout {
                UiLayout::Right => shared.camera.pos.x -= i.smooth_scroll_delta.y * 0.5,
                UiLayout::Left => shared.camera.pos.x += i.smooth_scroll_delta.y * 0.5,
                _ => {}
            }
        }

        shared.time = i.time as f32;
    });

    context.set_cursor_icon(shared.ui.cursor_icon);
    shared.ui.cursor_icon = egui::CursorIcon::Default;

    default_styling(context, shared);

    // apply individual element styling once, then immediately go back to default
    macro_rules! style_once {
        ($func:expr) => {
            $func;
            default_styling(context, shared);
        };
    }

    if let Some(_pos) = context.pointer_latest_pos() {
        if shared.ui.mobile {
            #[cfg(feature = "debug")]
            context
                .debug_painter()
                .circle_filled(_pos, 2., egui::Color32::GREEN);
        }
    }

    let anim_icon_size = 18;
    if shared.ui.anim.icon_images.len() == 0 {
        let mut full_img =
            image::load_from_memory(include_bytes!("../assets/anim_icons.png")).unwrap();

        if full_img.width() > 0 {
            let mut x = 0;
            while full_img.width() > 0 && x < full_img.width() - 1 {
                let img = full_img.crop(x, 0, 18, 18).into_rgba8();
                x += anim_icon_size;
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [img.width() as usize, img.height() as usize],
                    img.as_flat_samples().as_slice(),
                );
                let tex = context.load_texture("anim_icons", color_image, Default::default());
                shared.ui.anim.icon_images.push(tex);
            }
        }
    }
    camera_bar(context, shared);

    if shared.ui.polar_modal {
        modal::polar_modal(
            context,
            &shared.config,
            &mut shared.ui,
            &mut shared.armature,
            &mut shared.selections,
            &mut shared.events,
        );
    }
    if shared.ui.modal {
        modal::modal(context, &mut shared.ui, &shared.config);
    }
    if shared.ui.styles_modal {
        styles_modal::draw(shared, context);
    }
    if shared.ui.settings_modal {
        settings_modal::draw(shared, context);
    }
    if shared.ui.startup_window {
        startup_window::startup_modal(shared, context);
    }
    if shared.ui.donating_modal {
        modal::donating_modal(shared, context);
    }
    if shared.ui.atlas_modal {
        atlas_modal::draw(shared, context);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if shared.ui.checking_update {
        modal::modal(context, &mut shared.ui, &shared.config);
        let url = "https://skelform.org/data/version";
        let request = ureq::get(url).header("Example-Header", "header value");
        let raw_ver = match request.call() {
            Ok(mut data) => data.body_mut().read_to_string().unwrap(),
            Err(_) => "err".to_string(),
        };

        if raw_ver == "err" {
            let str = shared.ui.loc("startup.error_update");
            shared.events.open_modal(str.to_string(), false);
        } else if raw_ver != "" {
            let ver_str = raw_ver.split(' ').collect::<Vec<_>>();
            let ver_idx = ver_str[0].parse::<i32>().unwrap();
            let ver_name = ver_str[1];
            if ver_idx > crate::VERSION_IDX {
                shared.ui.new_version = ver_name.to_string();
                let str = "New version available: ".to_owned()
                    + &ver_name
                    + "\nGo to version page and download manually?";
                shared.events.open_polar_modal(PolarId::NewUpdate, str);
            } else {
                let str = "No updates available. This is the latest version.".to_string();
                shared.events.open_modal(str, false);
            }
        }

        shared.ui.checking_update = false;
    }
    style_once!(top_panel(context, shared));

    if shared.ui.anim.open {
        style_once!(keyframe_editor::draw(context, shared));
    }

    style_once!(armature_window::draw(context, shared));

    let min_default_size = 210.;
    let mut max_size = min_default_size;
    if shared.selections.bone_idx != usize::MAX {
        max_size = 250.;
    } else if shared.selections.anim_frame != -1 {
        max_size = 250.;
    }

    let mut enable_bone_panel = true;
    if let Some(_) = shared.selected_bone() {
        enable_bone_panel = !shared.ui.setting_ik_target;
    }

    // get current properties of selected bone, including animations
    let mut selected_bone = Bone::default();
    if shared.selections.bone_idx != usize::MAX
        && shared.selections.bone_idx < shared.armature.bones.len()
    {
        selected_bone = shared.selected_bone().unwrap().clone();

        if shared.ui.anim.open && shared.selections.anim != usize::MAX {
            let frame = shared.selections.anim_frame;
            let animated_bones = shared.armature.animate(shared.selections.anim, frame, None);
            selected_bone = animated_bones[shared.selections.bone_idx].clone();
        }
    }

    let bone_panel_id = "Bone";
    let mut side_panel = egui::SidePanel::right(bone_panel_id)
        .resizable(true)
        .max_width(max_size)
        .min_width(min_default_size)
        .default_width(min_default_size);
    if shared.config.layout == UiLayout::Left {
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
                let gradient = shared.config.colors.gradient.into();
                ui.gradient(ui.ctx().content_rect(), Color32::TRANSPARENT, gradient);

                if shared.selections.bone_idx != usize::MAX {
                    bone_panel::draw(selected_bone.clone(), ui, shared);
                } else if shared.selected_animation() != None && shared.selections.anim_frame != -1
                {
                    keyframe_panel::draw(ui, shared);
                }
            });
            shared.ui.bone_panel_rect = Some(ui.min_rect());
        }),
        &mut shared.camera.on_ui,
        context,
    );

    // adjust bar positions
    let bone_panel = shared.ui.bone_panel_rect.unwrap();
    let top_panel = shared.ui.top_panel_rect.unwrap();
    let armature_panel = shared.ui.armature_panel_rect.unwrap();
    let keyframe_panel = shared.ui.keyframe_panel_rect;
    match shared.config.layout {
        UiLayout::Split => {
            shared.ui.anim_bar.pos.x = bone_panel.left() - shared.ui.anim_bar.scale.x - 21.;
            shared.ui.anim_bar.pos.y = top_panel.bottom() - 1.;

            shared.ui.edit_bar.pos = Vec2::new(armature_panel.right(), top_panel.bottom());

            shared.ui.camera_bar.pos.x =
                bone_panel.left() - shared.ui.camera_bar.scale.x - ((6. * 3.3) as f32).ceil();
            if keyframe_panel != None && shared.ui.anim.open {
                shared.ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared.ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared.ui.camera_bar.pos.y -= shared.ui.camera_bar.scale.y + 15.;
        }
        UiLayout::Right => {
            shared.ui.edit_bar.pos.x = bone_panel.left() - shared.ui.edit_bar.scale.x - 28.;
            shared.ui.edit_bar.pos.y = top_panel.bottom();

            shared.ui.anim_bar.pos = Vec2::new(0., top_panel.bottom());

            shared.ui.camera_bar.pos.x = bone_panel.left() - shared.ui.camera_bar.scale.x - 21.;
            if keyframe_panel != None && shared.ui.anim.open {
                shared.ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared.ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared.ui.camera_bar.pos.y -= shared.ui.camera_bar.scale.y + 15.;
        }
        UiLayout::Left => {
            shared.ui.edit_bar.pos.x = bone_panel.right();
            shared.ui.edit_bar.pos.y = top_panel.bottom();

            shared.ui.anim_bar.pos.x = context.content_rect().right() - shared.ui.anim_bar.scale.x;
            shared.ui.anim_bar.pos.y = top_panel.bottom();

            shared.ui.camera_bar.pos.x = bone_panel.right() + 7.;
            if keyframe_panel != None && shared.ui.anim.open {
                shared.ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared.ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared.ui.camera_bar.pos.y -= shared.ui.camera_bar.scale.y + 15.;
        }
    }

    if shared.selections.bone_idx != usize::MAX {
        edit_mode_bar(context, shared);
    }

    if shared.armature.bones.len() > 0 {
        animate_bar(context, shared);
    }

    // check if mouse is on ui
    //
    // this check always returns false on mouse click, so it's only checked when the mouse isn't clicked
    if !shared.input.left_down {
        shared.camera.on_ui = context.is_pointer_over_area();
    }

    // close all context menus if clicking outside of them
    if shared.input.left_clicked && !shared.ui.context_menu.keep {
        shared.ui.context_menu.close();
    }

    macro_rules! helper_text {
        ($text:expr, $offset:expr) => {
            let align = egui::Align2::CENTER_CENTER;
            let font = egui::FontId::default();
            let mouse_pos = shared.input.mouse / shared.ui.scale + $offset;
            let painter = context.debug_painter();

            let pos = (mouse_pos + Vec2::new(1., 1.)).into();
            painter.text(pos, align, $text, font.clone(), egui::Color32::BLACK);

            let point_col = shared.config.colors.center_point.into();
            painter.text(mouse_pos.into(), align, $text, font, point_col);
        };
    }

    if shared.selected_bone() == None {
        return;
    }

    if shared.edit_mode.is_rotating {
        let offset = Vec2::new(50., 0.);
        let rot = selected_bone.rot / 3.14 * 180.;
        let formatted = (rot * 100.).round() / 100.;
        helper_text!(formatted.to_string() + "°", offset);
    }
    if shared.edit_mode.is_scaling {
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

pub fn kb_inputs(input: &mut egui::InputState, shared: &mut Shared) {
    mouse_button_as_key(input, egui::PointerButton::Primary, egui::Key::F31);
    mouse_button_as_key(input, egui::PointerButton::Secondary, egui::Key::F32);
    mouse_button_as_key(input, egui::PointerButton::Middle, egui::Key::F33);
    mouse_button_as_key(input, egui::PointerButton::Extra1, egui::Key::F34);
    mouse_button_as_key(input, egui::PointerButton::Extra2, egui::Key::F35);

    shared.input.holding_mod = input.modifiers.command;
    shared.input.holding_shift = input.modifiers.shift;

    if input.consume_shortcut(&shared.config.keys.undo) {
        shared.events.new(Events::Undo);
    }
    if input.consume_shortcut(&shared.config.keys.redo) {
        shared.events.new(Events::Redo);
    }

    if input.consume_shortcut(&shared.config.keys.zoom_in_camera) {
        shared.events.new(Events::CamZoomIn);
    }
    if input.consume_shortcut(&shared.config.keys.zoom_out_camera) {
        shared.events.new(Events::CamZoomOut);
    }

    if input.consume_shortcut(&shared.config.keys.save) {
        #[cfg(target_arch = "wasm32")]
        utils::save_web(shared);

        #[cfg(not(target_arch = "wasm32"))]
        utils::open_save_dialog(&shared.file_name, &shared.saving);
        //if shared.save_path == "" {
        //    utils::open_save_dialog();
        //} else {
        //    utils::save(shared.save_path.clone(), shared);
        //}
    }

    if input.consume_shortcut(&shared.config.keys.open) {
        #[cfg(not(target_arch = "wasm32"))]
        utils::open_import_dialog(&shared.file_name, &shared.import_contents);
        #[cfg(target_arch = "wasm32")]
        crate::clickFileInput(false);
    }

    // copy shortcut
    if input.consume_shortcut(&shared.config.keys.copy) {
        shared.copy_buffer = CopyBuffer::default();

        // copy bone(s)
        let idx = shared.selections.bone_idx;
        if idx != usize::MAX {
            copy_bone(shared, idx);
        }
    }

    // paste shortcut
    if input.consume_shortcut(&shared.config.keys.paste) {
        if shared.copy_buffer.keyframes.len() > 0 {
        } else if shared.copy_buffer.bones.len() > 0 {
            shared.undo_states.new_undo_bones(&shared.armature.bones);
            paste_bone(shared, shared.selections.bone_idx);
        }
    }

    if input.consume_shortcut(&shared.config.keys.cancel) {
        cancel_shortcut(shared);
    }
}

pub fn cancel_shortcut(shared: &mut Shared) {
    let ui = &mut shared.ui;
    let no_modals =
        !ui.styles_modal && !ui.modal && !ui.polar_modal && !ui.forced_modal && !ui.settings_modal;

    ui.styles_modal = false;
    ui.modal = false;
    ui.polar_modal = false;
    ui.forced_modal = false;
    ui.settings_modal = false;
    ui.atlas_modal = false;

    // if a context menu is open, cancel that instead
    if ui.context_menu.id != "" {
        shared.ui.context_menu.id = "".to_string();
        return;
    }

    if no_modals && !ui.setting_ik_target {
        shared.events.new(Events::UnselectAll);
    }

    #[cfg(target_arch = "wasm32")]
    {
        toggleElement(false, "image-dialog".to_string());
        toggleElement(false, "file-dialog".to_string());
    }

    shared.ui.setting_ik_target = false;
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

    input.keys_down.insert(fake_key);
    input.events.push(egui::Event::Key {
        key: fake_key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
}

fn top_panel(egui_ctx: &Context, shared: &mut Shared) {
    let panel = egui::TopBottomPanel::top("top_bar").frame(egui::Frame {
        fill: shared.config.colors.main.into(),
        stroke: Stroke::new(0., shared.config.colors.main),
        inner_margin: egui::Margin::default(),
        outer_margin: egui::Margin::default(),
        ..Default::default()
    });
    panel.show(egui_ctx, |ui| {
        ui.set_max_height(20.);
        let mut offset = 0.;
        egui::MenuBar::new().ui(ui, |ui| {
            menu_file_button(ui, shared);
            menu_edit_button(ui, shared);
            menu_view_button(ui, shared);

            macro_rules! title {
                ($title:expr) => {
                    egui::RichText::new($title).color(shared.config.colors.text)
                };
            }

            let str_settings = title!(&shared.ui.loc("top_bar.settings"));
            let button = ui.menu_button(str_settings, |ui| ui.close());
            if button.response.clicked() {
                shared.ui.settings_modal = true;
            }

            ui.menu_button(title!(&shared.ui.loc("top_bar.help.heading")), |ui| {
                ui.set_width(90.);
                //let str_user_docs = &shared.ui.loc("top_bar.help.user_docs");
                let str_user_docs = &shared.ui.loc("top_bar.help.user_docs");
                if top_bar_button(ui, str_user_docs, None, &mut offset, shared).clicked() {
                    utils::open_docs(true, "");
                }
                let str_dev_docs = &shared.ui.loc("top_bar.help.dev_docs");
                if top_bar_button(ui, str_dev_docs, None, &mut offset, shared).clicked() {
                    utils::open_docs(false, "");
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let str_binary = &shared.ui.loc("top_bar.help.binary_folder");
                    if top_bar_button(ui, str_binary, None, &mut offset, shared).clicked() {
                        match open::that(utils::bin_path()) {
                            Err(_) => {}
                            Ok(file) => file,
                        };
                    }
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let str_config = &shared.ui.loc("top_bar.help.config_folder");
                    if top_bar_button(ui, str_config, None, &mut offset, shared).clicked() {
                        match open::that(config_path().parent().unwrap()) {
                            Err(_) => {}
                            Ok(file) => file,
                        };
                    }
                }
            });
        });

        shared.ui.top_panel_rect = Some(ui.min_rect());
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
        let click = egui::Sense::click();
        let label = self.label(text).on_hover_cursor(hand).interact(click);

        if label.contains_pointer() {
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
) -> Option<egui::TextureHandle> {
    let thumb_img;
    if let Ok(data) = image::load_from_memory(&bytes) {
        thumb_img = data;
    } else {
        return None;
    }
    let color_image: egui::ColorImage;
    if has_alpha {
        color_image = egui::ColorImage::from_rgba_unmultiplied(
            [thumb_img.width() as usize, thumb_img.height() as usize],
            &thumb_img.clone().into_rgba8(),
        );
    } else {
        color_image = egui::ColorImage::from_rgb(
            [thumb_img.width() as usize, thumb_img.height() as usize],
            &thumb_img.clone().into_rgb8(),
        );
    }

    let ui_tex = ctx.load_texture("anim_icons", color_image, Default::default());

    Some(ui_tex)
}

fn menu_file_button(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut offset = 0.;
    let title = egui::RichText::new(&shared.ui.loc("top_bar.file.heading"))
        .color(shared.config.colors.text);
    ui.menu_button(title, |ui| {
        ui.set_width(125.);

        macro_rules! top_bar_button {
            ($name:expr, $kb:expr) => {
                top_bar_button(ui, $name, $kb, &mut offset, shared)
            };
        }

        let str_open = &shared.ui.loc("top_bar.file.open");
        if top_bar_button!(str_open, Some(&shared.config.keys.open)).clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_import_dialog(&shared.file_name, &shared.import_contents);
            #[cfg(target_arch = "wasm32")]
            crate::clickFileInput(false);
            ui.close();
        }
        let str_save = &shared.ui.loc("top_bar.file.save");
        if top_bar_button!(str_save, Some(&shared.config.keys.save)).clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_save_dialog(&shared.file_name, &shared.saving);
            #[cfg(target_arch = "wasm32")]
            utils::save_web(&shared);
            ui.close();
        }
        let str_startup = &shared.ui.loc("top_bar.file.startup");
        if top_bar_button!(str_startup, None).clicked() {
            shared.ui.startup_window = true;
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
                let headline = shared.ui.loc("startup.error_ffmpeg");
                shared.events.open_modal(headline.to_string(), false);
                return;
            }

            // complain if there's no proper animation to export
            let str = "No animation available.".to_string();
            if shared.selections.anim == usize::MAX {
                let anims = &shared.armature.animations;
                if anims.len() == 0 || anims[0].keyframes.len() == 0 {
                    shared.events.open_modal(str, false);
                    return;
                } else {
                    shared.selections.anim = 0;
                }
            } else if shared.last_keyframe() == None {
                shared.events.open_modal(str, false);
                return;
            }

            shared.recording = true;
            shared.ui.anim.open = true;
            shared.done_recording = true;
            shared.events.select_anim_frame(0);
            shared.ui.anim.loops = 1;
            ui.close();
        }
    });
}

fn menu_view_button(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut offset = 0.;

    let str_view = &shared.ui.loc("top_bar.view.heading");
    let title = egui::RichText::new(str_view).color(shared.config.colors.text);
    ui.menu_button(title, |ui| {
        macro_rules! tpb {
            ($name:expr, $kb:expr) => {
                top_bar_button(ui, $name, $kb, &mut offset, shared)
            };
        }

        ui.set_width(125.);
        let str_zoom_in = &shared.ui.loc("top_bar.view.zoom_in");
        if tpb!(str_zoom_in, Some(&shared.config.keys.zoom_in_camera)).clicked() {
            shared.events.new(Events::CamZoomIn);
        }
        let str_zoom_out = &shared.ui.loc("top_bar.view.zoom_out");
        if tpb!(str_zoom_out, Some(&shared.config.keys.zoom_out_camera)).clicked() {
            shared.events.new(Events::CamZoomOut);
        }
    });
}

fn menu_edit_button(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut offset = 0.;
    let str_edit = &shared.ui.loc("top_bar.edit.heading");
    let title = egui::RichText::new(str_edit).color(shared.config.colors.text);
    ui.menu_button(title, |ui| {
        ui.set_width(90.);
        let key_undo = Some(&shared.config.keys.undo);
        let str_undo = &shared.ui.loc("top_bar.edit.undo");
        if top_bar_button(ui, str_undo, key_undo, &mut offset, shared).clicked() {
            shared.events.new(Events::Undo);
            ui.close();
        }
        let str_redo = &shared.ui.loc("top_bar.edit.redo");
        let key_redo = Some(&shared.config.keys.redo);
        if top_bar_button(ui, str_redo, key_redo, &mut offset, shared).clicked() {
            shared.events.new(Events::Redo);
            ui.close();
        }
    });
}

fn edit_mode_bar(egui_ctx: &Context, shared: &mut Shared) {
    let mut ik_disabled = true;
    let mut is_end = false;
    if let Some(bone) = shared.selected_bone() {
        ik_disabled = bone.ik_disabled || shared.armature.bone_eff(bone.id) == JointEffector::None;
        is_end = shared.armature.bone_eff(bone.id) == JointEffector::End;
    }

    // edit mode window
    let window = egui::Window::new("Mode")
        .resizable(false)
        .title_bar(false)
        .max_width(100.)
        .movable(false)
        .current_pos(egui::Pos2::new(
            shared.ui.edit_bar.pos.x + 7.5,
            shared.ui.edit_bar.pos.y - 1.,
        ));
    window.show(egui_ctx, |ui| {
        ui.horizontal(|ui| {
            macro_rules! edit_mode_button {
                ($label:expr, $edit_mode:expr, $event:expr, $check:expr) => {
                    ui.add_enabled_ui($check, |ui| {
                        if selection_button($label, shared.edit_mode.current == $edit_mode, ui)
                            .clicked()
                        {
                            shared.events.new($event)
                        };
                    })
                };
            }
            let ik_disabled = !shared.ui.showing_mesh && ik_disabled;
            let rot = ik_disabled || is_end;
            edit_mode_button!(
                &shared.ui.loc("move"),
                EditModes::Move,
                Events::EditModeMove,
                ik_disabled
            );
            edit_mode_button!(
                &shared.ui.loc("rotate"),
                EditModes::Rotate,
                Events::EditModeRotate,
                rot
            );
            edit_mode_button!(
                &shared.ui.loc("scale"),
                EditModes::Scale,
                Events::EditModeScale,
                ik_disabled
            );
        });
        shared.ui.edit_bar.scale = ui.min_rect().size().into();
    });
}

fn animate_bar(egui_ctx: &Context, shared: &mut Shared) {
    let window = egui::Window::new("Animating")
        .resizable(false)
        .title_bar(false)
        .max_width(100.)
        .movable(false)
        .current_pos(egui::Pos2::new(
            shared.ui.anim_bar.pos.x,
            shared.ui.anim_bar.pos.y,
        ));
    window.show(egui_ctx, |ui| {
        ui.horizontal(|ui| {
            let str_armature = &shared.ui.loc("armature_panel.heading");
            if selection_button(str_armature, !shared.ui.anim.open, ui).clicked() {
                shared.ui.anim.open = false;
                for anim in &mut shared.armature.animations {
                    anim.elapsed = None;
                }
            }
            let str_animation = &shared.ui.loc("keyframe_editor.heading");
            if selection_button(str_animation, shared.ui.anim.open, ui).clicked() {
                shared.ui.anim.open = true;
            }
            shared.ui.anim_bar.scale = ui.min_rect().size().into();
        });
        shared.ui.anim_bar.scale = ui.min_rect().size().into();
    });
}

fn camera_bar(egui_ctx: &Context, shared: &mut Shared) {
    let margin = 6.;
    let window = egui::Window::new("Camera")
        .resizable(false)
        .title_bar(false)
        .max_width(60.)
        .max_height(25.)
        .movable(false)
        .frame(egui::Frame {
            fill: shared.config.colors.gradient.into(),
            inner_margin: margin.into(),
            stroke: Stroke {
                width: 1.,
                color: shared.config.colors.dark_accent.into(),
            },
            ..Default::default()
        })
        .current_pos(egui::Pos2::new(
            shared.ui.camera_bar.pos.x,
            shared.ui.camera_bar.pos.y,
        ));
    window.show(egui_ctx, |ui| {
        macro_rules! input {
            ($float:expr, $id:expr, $label:expr, $tip:expr) => {
                ui.horizontal(|ui| {
                    ui.label($label).on_hover_text(&shared.ui.loc($tip));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        (_, $float, _) = ui.float_input(
                            $id.to_string(),
                            &mut shared.ui,
                            $float.round(),
                            1.,
                            None,
                        );
                    })
                })
            };
        }

        input!(shared.camera.pos.x, "cam_pos_x", "X", "cam_x");
        input!(shared.camera.pos.y, "cam_pos_y", "Y", "cam_y");
        input!(shared.camera.zoom, "cam_zoom", "🔍", "cam_zoom");

        shared.ui.camera_bar.scale = ui.min_rect().size().into();
    });
}

/// Default styling to apply across all UI.
pub fn default_styling(context: &Context, shared: &Shared) {
    let mut visuals = egui::Visuals::dark();
    let colors = &shared.config.colors;

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
    shared: &Shared,
) -> egui::Response {
    let height = 20.;

    let rect = egui::Rect::from_min_size(
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset),
        egui::Vec2::new(ui.min_rect().width(), height),
    );
    let response: egui::Response = ui.allocate_rect(rect, egui::Sense::click());
    let painter = ui.painter_at(ui.min_rect());

    let col = if response.hovered() {
        shared.config.colors.light_accent.into()
    } else {
        egui::Color32::TRANSPARENT
    };
    painter.rect_filled(rect, egui::CornerRadius::ZERO, col);

    let font = egui::FontId::new(13., egui::FontFamily::Proportional);

    // text
    let pos =
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset) + egui::vec2(5., 2.);
    let text_col = shared.config.colors.text.into();
    painter.text(pos, egui::Align2::LEFT_TOP, text, font.clone(), text_col);

    let key_str = if key != None {
        key.unwrap().display()
    } else {
        "".to_string()
    };

    // kb key text
    if !shared.ui.mobile {
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
    on_ui: &mut bool,
    context: &egui::Context,
) {
    if let Some(resize) = context.read_response(egui::Id::new(id).with("__resize")) {
        if resize.hovered() || panel.response.hovered() {
            *on_ui = true;
        }
    }
}

pub fn copy_bone(shared: &mut Shared, idx: usize) {
    let arm_bones = &shared.armature.bones;
    let mut bones = vec![];
    armature_window::get_all_children(&arm_bones, &mut bones, &arm_bones[idx]);
    bones.insert(0, shared.armature.bones[idx].clone());
    shared.copy_buffer.bones = bones;
}

pub fn paste_bone(shared: &mut Shared, idx: usize) {
    shared.undo_states.new_undo_bones(&shared.armature.bones);

    // determine which id to give the new bone(s), based on the highest current id
    let ids: Vec<i32> = shared.armature.bones.iter().map(|bone| bone.id).collect();
    let mut highest_id = 0;
    for id in ids {
        highest_id = id.max(highest_id);
    }
    highest_id += 1;

    let mut insert_idx = usize::MAX;
    let mut id_refs: HashMap<i32, i32> = HashMap::new();

    for b in 0..shared.copy_buffer.bones.len() {
        let bone = &mut shared.copy_buffer.bones[b];

        highest_id += 1;
        let new_id = highest_id;

        id_refs.insert(bone.id, new_id);
        bone.id = highest_id;

        if bone.parent_id != -1 && id_refs.get(&bone.parent_id) != None {
            bone.parent_id = *id_refs.get(&bone.parent_id).unwrap();
        } else if idx != usize::MAX {
            insert_idx = idx + 1;
            bone.parent_id = shared.armature.bones[idx].id;
        } else {
            bone.parent_id = -1;
        }
    }
    if insert_idx == usize::MAX {
        shared.armature.bones.append(&mut shared.copy_buffer.bones);
    } else {
        for bone in &shared.copy_buffer.bones {
            shared.armature.bones.insert(insert_idx, bone.clone());
            insert_idx += 1;
        }
    }
}
