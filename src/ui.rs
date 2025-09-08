//! Core user interface (UI) logic.
use egui::{Color32, Context, Shadow, Stroke};

use crate::*;

const HELP_LIGHT_CANT: &str = "There is already an animation! Looks like you've figured it out.\n\nTo activate the help light, please start a new project.";

const FFMPEG_ERR: &str =
    "ffmpeg is not available.\n\nPlease ensure it is installed and in your $PATH.";

pub trait EguiUi {
    fn skf_button(&mut self, text: &str) -> egui::Response;
    fn gradient(&mut self, rect: egui::Rect, top: Color32, bottom: Color32);
    fn clickable_label(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response;
    fn text_input(
        &mut self,
        id: String,
        shared: &mut Shared,
        value: String,
        options: Option<TextInputOptions>,
    ) -> (bool, String, egui::Response);
    fn float_input(
        &mut self,
        id: String,
        shared: &mut Shared,
        value: f32,
        modifier: f32,
    ) -> (bool, f32, egui::Response);
    fn debug_rect(&mut self, rect: egui::Rect);
}

/// The `main` of this module.
pub fn draw(context: &Context, shared: &mut Shared, _window_factor: f32) {
    shared.input.last_pressed = None;

    context.input_mut(|i| {
        if shared.ui.rename_id == "" {
            kb_inputs(i, shared);
        }
        shared.input.last_pressed = i.keys_down.iter().last().copied();

        shared.input.left_clicked = i.pointer.primary_clicked();
        shared.input.right_clicked = i.pointer.secondary_clicked();
        shared.input.left_down = i.pointer.primary_down();
        shared.input.left_pressed = i.pointer.primary_pressed();

        if i.smooth_scroll_delta.y != 0. && !shared.input.on_ui {
            ui::set_zoom(
                shared.camera.zoom + (i.smooth_scroll_delta.y as f32),
                shared,
            );
        }

        shared.time = i.time as f32;
    });

    context.set_cursor_icon(shared.cursor_icon);
    shared.cursor_icon = egui::CursorIcon::Default;

    default_styling(context, shared);

    let scale_mod: f32;

    #[cfg(not(target_arch = "wasm32"))]
    {
        scale_mod = 1.;
    }

    #[cfg(target_arch = "wasm32")]
    {
        scale_mod = _window_factor;
    }

    context.set_zoom_factor(shared.ui.scale * scale_mod);

    // apply individual element styling once, then immediately go back to default
    macro_rules! style_once {
        ($func:expr) => {
            $func;
            default_styling(context, shared);
        };
    }

    if let Some(_pos) = context.pointer_latest_pos() {
        #[cfg(feature = "mobile")]
        #[cfg(feature = "debug")]
        context
            .debug_painter()
            .circle_filled(_pos, 2., egui::Color32::GREEN);
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

    if shared.ui.has_state(UiState::PolarModal) {
        modal::polar_modal(shared, context);
    }
    if shared.ui.has_state(UiState::Modal) {
        modal::modal(shared, context);
    }
    if shared.ui.has_state(UiState::ImageModal) {
        modal::image_modal(shared, context);
    }
    if shared.ui.has_state(UiState::SettingsModal) {
        settings_modal::draw(shared, context);
    }
    if shared.ui.has_state(UiState::FirstTimeModal) {
        modal::first_time_modal(shared, context);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if shared.ui.has_state(UiState::StartupWindow) {
        startup_window::startup_modal(shared, context);
    }
    style_once!(top_panel(context, shared));

    if shared.ui.anim.open {
        style_once!(keyframe_editor::draw(context, shared));
    } else {
        shared.ui.camera_bar_pos.y = context.screen_rect().bottom();
    }

    style_once!(armature_window::draw(context, shared));

    let min_default_size = 220.;
    let mut max_size = min_default_size;
    if shared.ui.selected_bone_idx != usize::MAX {
        max_size = 250.;
    } else if shared.ui.anim.selected_frame != -1 {
        max_size = 250.;
    }

    let mut enable_bone_panel = true;
    if let Some(_) = shared.selected_bone() {
        enable_bone_panel = !shared.ui.setting_ik_target;
    }

    let bone_panel_id = "Bone";
    draw_resizable_panel(
        bone_panel_id,
        egui::SidePanel::right(bone_panel_id)
            .resizable(true)
            .max_width(max_size)
            .min_width(min_default_size)
            .default_width(min_default_size)
            .show(context, |ui| {
                ui.add_enabled_ui(enable_bone_panel, |ui| {
                    ui.gradient(
                        ui.ctx().screen_rect(),
                        Color32::TRANSPARENT,
                        shared.config.colors.gradient.into(),
                    );

                    if shared.ui.selected_bone_idx != usize::MAX {
                        bone_panel::draw(ui, shared);
                    } else if shared.selected_animation() != None
                        && shared.ui.anim.selected_frame != -1
                    {
                        keyframe_panel::draw(ui, shared);
                    }

                    shared.ui.animate_mode_bar_pos.x = ui.min_rect().left();
                    shared.ui.camera_bar_pos.x = ui.min_rect().left();
                })
            }),
        &mut shared.input.on_ui,
        context,
    );

    if shared.ui.selected_bone_idx != usize::MAX {
        edit_mode_bar(context, shared);
    }

    if shared.armature.bones.len() > 0 {
        animate_bar(context, shared);
    }

    // check if mouse is on ui
    //
    // this check always returns false on mouse click, so it's only checked when the mouse isn't clicked
    if !shared.input.left_down {
        shared.input.on_ui = context.is_pointer_over_area();
    }

    // close all context menus if clicking outside of them
    if shared.input.left_clicked && !shared.ui.context_menu.keep {
        shared.ui.context_menu.close();
    }
}

pub fn kb_inputs(input: &mut egui::InputState, shared: &mut Shared) {
    mouse_button_as_key(input, egui::PointerButton::Primary, egui::Key::F31);
    mouse_button_as_key(input, egui::PointerButton::Secondary, egui::Key::F32);
    mouse_button_as_key(input, egui::PointerButton::Middle, egui::Key::F33);
    mouse_button_as_key(input, egui::PointerButton::Extra1, egui::Key::F34);
    mouse_button_as_key(input, egui::PointerButton::Extra2, egui::Key::F35);

    if input.consume_shortcut(&shared.config.keys.undo) {
        utils::undo_redo(true, shared);
    }
    if input.consume_shortcut(&shared.config.keys.undo) {
        utils::undo_redo(false, shared);
    }

    if input.consume_shortcut(&shared.config.keys.zoom_in_camera) {
        ui::set_zoom(shared.camera.zoom - 10., shared);
    }
    if input.consume_shortcut(&shared.config.keys.zoom_out_camera) {
        ui::set_zoom(shared.camera.zoom + 10., shared);
    }

    if input.consume_shortcut(&shared.config.keys.save) {
        #[cfg(target_arch = "wasm32")]
        utils::save_web(&mut shared.armature);

        #[cfg(not(target_arch = "wasm32"))]
        utils::open_save_dialog(shared.temp_path.save.clone());
        //if shared.save_path == "" {
        //    utils::open_save_dialog();
        //} else {
        //    utils::save(shared.save_path.clone(), shared);
        //}
    }

    if input.consume_shortcut(&shared.config.keys.open) {
        #[cfg(not(target_arch = "wasm32"))]
        utils::open_import_dialog(shared.temp_path.import.clone());
        #[cfg(target_arch = "wasm32")]
        toggleElement(true, "file-dialog".to_string());
    }

    if input.consume_shortcut(&shared.config.keys.cancel) {
        #[cfg(target_arch = "wasm32")]
        {
            toggleElement(false, "image-dialog".to_string());
            toggleElement(false, "file-dialog".to_string());
            toggleElement(false, "ui-slider".to_string());
        }

        shared.ui.set_state(UiState::ImageModal, false);
        shared.ui.set_state(UiState::Modal, false);
        shared.ui.set_state(UiState::PolarModal, false);
        shared.ui.set_state(UiState::ForcedModal, false);
        shared.ui.set_state(UiState::SettingsModal, false);

        shared.ui.setting_ik_target = false;
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
    egui::TopBottomPanel::top("top_bar")
        .frame(egui::Frame {
            fill: shared.config.colors.main.into(),
            stroke: Stroke::new(0., shared.config.colors.main),
            inner_margin: egui::Margin {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            },
            outer_margin: egui::Margin {
                left: 0,
                right: 0,
                top: 0,
                bottom: 0,
            },
            ..Default::default()
        })
        .show(egui_ctx, |ui| {
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

                let str_settings = shared.loc("top_bar.settings");
                if ui
                    .menu_button(title!(str_settings), |ui| ui.close())
                    .response
                    .clicked()
                {
                    shared.ui.set_state(UiState::SettingsModal, true);
                }

                ui.menu_button(title!(shared.loc("top_bar.help.heading")), |ui| {
                    ui.set_width(100.);
                    let str = if !shared.ui.tutorial_step_is(TutorialStep::None) {
                        shared.loc("top_bar.help.stop_help_light")
                    } else {
                        shared.loc("top_bar.help.help_light")
                    };
                    if top_bar_button(ui, str, None, &mut offset, shared).clicked() {
                        if !shared.ui.tutorial_step_is(TutorialStep::None) {
                            shared.ui.set_tutorial_step(TutorialStep::None);
                        } else if shared.ui.tutorial_step_is(TutorialStep::Finish) {
                            shared.ui.open_modal(HELP_LIGHT_CANT.to_string(), false);
                        } else {
                            shared.ui.start_tutorial(&shared.armature);
                            if shared.armature.animations.len() > 0 {
                                shared.ui.tutorial_step = TutorialStep::Finish;
                                shared.ui.open_modal(HELP_LIGHT_CANT.to_string(), false);
                            }
                        }
                        ui.close();
                    }
                    let str_user_docs = shared.loc("top_bar.help.user_docs");
                    if top_bar_button(ui, str_user_docs, None, &mut offset, shared).clicked() {
                        utils::open_docs(true, "");
                    }
                    let str_dev_docs = shared.loc("top_bar.help.dev_docs");
                    if top_bar_button(ui, str_dev_docs, None, &mut offset, shared).clicked() {
                        utils::open_docs(false, "");
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let str_binary = shared.loc("top_bar.help.binary_folder");
                        if top_bar_button(ui, str_binary, None, &mut offset, shared).clicked() {
                            match open::that(utils::bin_path()) {
                                Err(_) => {}
                                Ok(file) => file,
                            };
                        }
                    }

                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let str_config = shared.loc("top_bar.help.config_folder");
                        if top_bar_button(ui, str_config, None, &mut offset, shared).clicked() {
                            match open::that(config_path().parent().unwrap()) {
                                Err(_) => {}
                                Ok(file) => file,
                            };
                        }
                    }
                });
            });

            shared.ui.edit_bar_pos.y = ui.min_rect().bottom();
            shared.ui.animate_mode_bar_pos.y = ui.min_rect().bottom();
        });
}

impl EguiUi for egui::Ui {
    fn skf_button(&mut self, text: &str) -> egui::Response {
        self.add(
            egui::Button::new(egui::RichText::new(text)).corner_radius(egui::CornerRadius::ZERO),
        )
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
        let label = self
            .label(text)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .interact(egui::Sense::click());

        if label.contains_pointer() {
            return label.highlight();
        }

        label
    }

    fn text_input(
        &mut self,
        id: String,
        shared: &mut Shared,
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

        if options.as_ref().unwrap().focus && !shared.ui.input_focused {
            #[cfg(feature = "mobile")]
            open_mobile_input(shared.ui.edit_value.clone().unwrap());

            shared.ui.input_focused = true;
            shared.ui.edit_value = Some(value.clone());
        }

        if shared.ui.rename_id != id {
            input = self.add_sized(
                options.as_ref().unwrap().size,
                egui::TextEdit::singleline(&mut value)
                    .hint_text(options.as_ref().unwrap().placeholder.clone()),
            );
            // extract value as a string and store it with edit_value
            if input.has_focus() {
                shared.ui.edit_value = Some(value.clone());
                shared.ui.rename_id = id.to_string();
                #[cfg(feature = "mobile")]
                open_mobile_input(shared.ui.edit_value.clone().unwrap());
            }
        } else {
            input = self.add_sized(
                options.as_ref().unwrap().size,
                egui::TextEdit::singleline(shared.ui.edit_value.as_mut().unwrap())
                    .hint_text(options.as_ref().unwrap().placeholder.clone()),
            );

            let mut entered = false;

            // if input modal is closed, consider the value entered
            #[cfg(feature = "mobile")]
            {
                shared.ui.edit_value = Some(getEditInput());
                if !isModalActive("edit-input-modal".to_string()) {
                    entered = true;
                }
            }

            if self.input(|i| i.key_pressed(egui::Key::Enter)) || input.lost_focus() {
                entered = true;
            }

            let mut final_value = shared.ui.edit_value.as_ref().unwrap();
            if final_value == "" {
                final_value = &options.as_ref().unwrap().default;
            }

            if entered {
                shared.ui.input_focused = false;
                shared.ui.rename_id = "".to_string();
                return (true, final_value.clone(), input);
            }

            if input.lost_focus() {
                shared.ui.rename_id = "".to_string();
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
        shared: &mut Shared,
        value: f32,
        modifier: f32,
    ) -> (bool, f32, egui::Response) {
        let (edited, _, input) = self.text_input(
            id,
            shared,
            (value * modifier).to_string(),
            Some(TextInputOptions {
                size: Vec2::new(40., 20.),
                ..Default::default()
            }),
        );

        if edited {
            shared.ui.rename_id = "".to_string();
            if shared.ui.edit_value.as_mut().unwrap() == "" {
                shared.ui.edit_value = Some("0".to_string());
            }
            match shared.ui.edit_value.as_mut().unwrap().parse::<f32>() {
                Ok(output) => {
                    return (true, output / modifier, input);
                }
                Err(_) => {
                    return (false, value, input);
                }
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
    let title =
        egui::RichText::new(shared.loc("top_bar.file.heading")).color(shared.config.colors.text);
    ui.menu_button(title, |ui| {
        ui.set_width(125.);

        macro_rules! top_bar_button {
            ($name:expr, $kb:expr) => {
                top_bar_button(ui, $name, $kb, &mut offset, shared)
            };
        }

        let str_open = shared.loc("top_bar.file.open");
        if top_bar_button!(str_open, Some(&shared.config.keys.open)).clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_import_dialog(shared.temp_path.import.clone());
            #[cfg(target_arch = "wasm32")]
            toggleElement(true, "file-dialog".to_string());
            ui.close();
        }
        let str_save = shared.loc("top_bar.file.save");
        if top_bar_button!(str_save, Some(&shared.config.keys.save)).clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_save_dialog(shared.temp_path.save.clone());
            #[cfg(target_arch = "wasm32")]
            utils::save_web(&shared.armature);
            ui.close();
        }
        let str_startup = shared.loc("top_bar.file.startup");
        if top_bar_button!(str_startup, None).clicked() {
            shared.ui.set_state(UiState::StartupWindow, true);
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
                let headline = FFMPEG_ERR;
                shared.ui.open_modal(headline.to_string(), false);
                return;
            }

            // complain if there's no proper animation to export
            if shared.ui.anim.selected == usize::MAX {
                if shared.armature.animations.len() == 0
                    || shared.armature.animations[0].keyframes.len() == 0
                {
                    shared
                        .ui
                        .open_modal("No animation available.".to_string(), false);
                    return;
                } else {
                    shared.ui.anim.selected = 0;
                }
            } else if shared.last_keyframe() == None {
                shared
                    .ui
                    .open_modal("No animation available.".to_string(), false);
                return;
            }

            shared.recording = true;
            shared.ui.anim.open = true;
            shared.done_recording = true;
            shared.ui.select_anim_frame(0);
            shared.ui.anim.loops = 1;
            ui.close();
        }
    });
}

fn menu_view_button(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut offset = 0.;

    let str_view = shared.loc("top_bar.view.heading");
    let title = egui::RichText::new(str_view).color(shared.config.colors.text);
    ui.menu_button(title, |ui| {
        macro_rules! tpb {
            ($name:expr, $kb:expr) => {
                top_bar_button(ui, $name, $kb, &mut offset, shared)
            };
        }

        ui.set_width(125.);
        let str_zoom_in = shared.loc("top_bar.view.zoom_in");
        if tpb!(str_zoom_in, Some(&shared.config.keys.zoom_in_camera)).clicked() {
            set_zoom(shared.camera.zoom - 10., shared);
        }
        let str_zoom_out = shared.loc("top_bar.view.zoom_out");
        if tpb!(str_zoom_out, Some(&shared.config.keys.zoom_out_camera)).clicked() {
            set_zoom(shared.camera.zoom + 10., shared);
        }
    });
}

fn menu_edit_button(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut offset = 0.;
    let str_edit = shared.loc("top_bar.edit.heading");
    let title = egui::RichText::new(str_edit).color(shared.config.colors.text);
    ui.menu_button(title, |ui| {
        ui.set_width(90.);
        let key_undo = Some(&shared.config.keys.undo);
        let str_undo = shared.loc("top_bar.edit.undo");
        if top_bar_button(ui, str_undo, key_undo, &mut offset, shared).clicked() {
            utils::undo_redo(true, shared);
            ui.close();
        }
        let str_redo = shared.loc("top_bar.edit.redo");
        let key_redo = Some(&shared.config.keys.redo);
        if top_bar_button(ui, str_redo, key_redo, &mut offset, shared).clicked() {
            utils::undo_redo(false, shared);
            ui.close();
        }
    });
}

fn edit_mode_bar(egui_ctx: &Context, shared: &mut Shared) {
    // edit mode window
    egui::Window::new("Mode")
        .resizable(false)
        .title_bar(false)
        .max_width(100.)
        .movable(false)
        .current_pos(egui::Pos2::new(
            shared.ui.edit_bar_pos.x + 7.5,
            shared.ui.edit_bar_pos.y - 1.,
        ))
        .show(egui_ctx, |ui| {
            ui.add_enabled_ui(!shared.ui.editing_mesh, |ui| {
                ui.horizontal(|ui| {
                    macro_rules! edit_mode_button {
                        ($label:expr, $edit_mode:expr) => {
                            if selection_button($label, shared.edit_mode == $edit_mode, ui)
                                .clicked()
                            {
                                shared.edit_mode = $edit_mode;
                            };
                        };
                    }
                    edit_mode_button!(shared.loc("move"), EditMode::Move);
                    edit_mode_button!(shared.loc("rotate"), EditMode::Rotate);
                    edit_mode_button!(shared.loc("scale"), EditMode::Scale);
                });
            })
        });
}

fn animate_bar(egui_ctx: &Context, shared: &mut Shared) {
    egui::Window::new("Animating")
        .resizable(false)
        .title_bar(false)
        .max_width(100.)
        .movable(false)
        .current_pos(egui::Pos2::new(
            shared.ui.animate_mode_bar_pos.x - shared.ui.animate_mode_bar_scale.x - 21.,
            shared.ui.animate_mode_bar_pos.y - 1.,
        ))
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                let str_armature = shared.loc("armature_panel.heading");
                if selection_button(str_armature, !shared.ui.anim.open, ui).clicked() {
                    shared.ui.anim.open = false;
                }
                let str_animation = shared.loc("keyframe_editor.heading");
                let button = selection_button(str_animation, shared.ui.anim.open, ui);
                draw_tutorial_rect(TutorialStep::OpenAnim, button.rect, shared, ui);
                if button.clicked() {
                    shared.ui.anim.open = true;
                    shared
                        .ui
                        .start_next_tutorial_step(TutorialStep::CreateAnim, &shared.armature);
                }
                shared.ui.animate_mode_bar_scale = ui.min_rect().size().into();
            });
        });
}

fn camera_bar(egui_ctx: &Context, shared: &mut Shared) {
    let margin = 6.;
    egui::Window::new("Camera")
        .resizable(false)
        .title_bar(false)
        .max_width(100.)
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
            shared.ui.camera_bar_pos.x - shared.ui.camera_bar_scale.x - (margin * 3.3).ceil(),
            shared.ui.camera_bar_pos.y - shared.ui.camera_bar_scale.y - 15.,
        ))
        .show(egui_ctx, |ui| {
            macro_rules! input {
                ($element:expr, $float:expr, $id:expr, $ui:expr, $label:expr) => {
                    if $label != "" {
                        $ui.label($label);
                    }
                    (_, $float, _) = $ui.float_input($id.to_string(), shared, $float, 1.);
                };
            }

            ui.horizontal(|ui| {
                ui.label("Camera:");
                input!(shared.camera.pos, shared.camera.pos.x, "cam_pos_y", ui, "X");
                input!(shared.camera.pos, shared.camera.pos.y, "cam_pos_x", ui, "Y");
            });

            ui.horizontal(|ui| {
                ui.label("Zoom:");
                input!(shared.camera.zoom, shared.camera.zoom, "cam_zoom", ui, "");
            });

            shared.ui.camera_bar_scale = ui.min_rect().size().into();
        })
        .unwrap()
        .response;
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

    visuals.override_text_color = Some(colors.text.into());
    let mut col = colors.text;
    col -= Color::new(100, 100, 100, 0);
    visuals.weak_text_color = Some(col.into());

    context.set_visuals(visuals);
}

pub fn set_zoom(zoom: f32, shared: &mut Shared) {
    shared.camera.zoom = zoom;
    if shared.camera.zoom < 0.1 {
        shared.camera.zoom = 0.1;
    }
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
    #[allow(unused_variables)]
    #[allow(unused_mut)]
    let mut width = 100.;

    #[cfg(feature = "mobile")]
    {
        width *= 0.8;
    }

    let rect = egui::Rect::from_min_size(
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset),
        egui::Vec2::new(ui.min_rect().width(), height),
    );
    let response: egui::Response = ui.allocate_rect(rect, egui::Sense::click());
    let painter = ui.painter_at(ui.min_rect());
    if response.hovered() {
        painter.rect_filled(
            rect,
            egui::CornerRadius::ZERO,
            shared.config.colors.light_accent,
        );
    } else {
        painter.rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::TRANSPARENT);
    }

    let font = egui::FontId::new(13., egui::FontFamily::Proportional);

    // text
    painter.text(
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset) + egui::vec2(5., 2.),
        egui::Align2::LEFT_TOP,
        text,
        font.clone(),
        shared.config.colors.text.into(),
    );

    let key_str = if key != None {
        key.unwrap().display()
    } else {
        "".to_string()
    };

    // kb key text
    #[cfg(not(feature = "mobile"))]
    painter.text(
        egui::Pos2::new(ui.min_rect().right(), ui.min_rect().top() + *offset)
            + egui::vec2(-5., 2.5),
        egui::Align2::RIGHT_TOP,
        key_str,
        font.clone(),
        egui::Color32::DARK_GRAY,
    );

    // set next button's Y to below this one
    *offset += height + 2.;

    response
}

pub fn visualize_bone_point(context: &Context, shared: &Shared) {
    egui::Area::new("background_area".into())
        .order(egui::Order::Foreground) // Very back
        .show(context, |ui| {
            for bone in &shared.armature.bones {
                ui.painter().circle_filled(
                    utils::world_to_screen_space(bone.pos, shared.window, shared.camera.zoom, true)
                        .into(),
                    10.,
                    egui::Color32::GREEN,
                );
            }
        });
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

pub fn draw_tutorial_rect(
    step: TutorialStep,
    rect: egui::Rect,
    shared: &mut Shared,
    ui: &mut egui::Ui,
) {
    if shared.ui.tutorial_step_is(step) {
        ui::draw_fading_rect(ui, rect, Color32::GOLD, 60., 1.);
    }
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

#[cfg(feature = "mobile")]
fn open_mobile_input(value: String) {
    setEditInput(value);
    toggleElement(true, "edit-input-modal".to_string());
    focusEditInput();
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
