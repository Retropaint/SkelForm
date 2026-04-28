//! Core user interface (UI) logic.
use egui::{Color32, Context, Shadow, Stroke};
use modal::modal_x;

use crate::*;

#[rustfmt::skip]
pub trait EguiUi {
    fn skf_button(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response;
    fn sized_skf_button(&mut self, size: impl Into<egui::Vec2>, text: impl Into<egui::WidgetText>) -> egui::Response;
    fn gradient(&mut self, rect: egui::Rect, top: Color32, bottom: Color32);
    fn clickable_label(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response;
    fn text_input(&mut self,id: String, shared_ui: &mut crate::Ui, value: String, options: Option<TextInputOptions>) -> (bool, String, egui::Response);
    fn float_input(&mut self, id: String, shared_ui: &mut crate::Ui, value: f32, modifier: f32, options: Option<TextInputOptions>) -> (bool, f32, egui::Response);
    fn debug_rect(&mut self, rect: egui::Rect);
    fn context_rename(&mut self, shared_ui: &mut crate::Ui, config: &Config, id: String);
    fn context_delete(&mut self, shared_ui: &mut crate::Ui, config: &Config, events: &mut EventState, loc_code: &str, polar_id: PolarId);
    fn context_button(&mut self, text: &str, config: &Config) -> egui::Response;
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
    copy_buffer: &CopyBuffer,
) {
    shared_ui.context_menu.keep = false;

    let sel = selections.clone();

    context.set_cursor_icon(shared_ui.cursor_icon);
    shared_ui.cursor_icon = egui::CursorIcon::Default;

    default_styling(context, &config);

    // context menu
    let is_different = shared_ui.context_menu.id != shared_ui.context_menu.last_id;
    let is_closed = shared_ui.context_menu.id == "";
    if is_different || is_closed {
        // (-1, -1) pos is used to reset menu width
        shared_ui.context_menu.pos = Vec2::new(-1., -1.);
        shared_ui.context_menu.last_id = shared_ui.context_menu.id.clone();
    } else if !shared_ui.context_menu.hide {
        let pos = shared_ui.context_menu.pos;
        egui::Area::new("context_menu".into())
            .fixed_pos(Vec2::new(pos.x, pos.y))
            .order(egui::Order::Foreground)
            .show(context, |ui| {
                let id = shared_ui.context_menu.id.clone();
                let frame = egui::Frame::popup(ui.style())
                    .show(ui, |ui| {
                        if shared_ui.context_menu.pos == Vec2::new(-1., -1.) {
                            // menu width won't re-adjust if it's bigger than its content,
                            // so reset it
                            ui.set_width(0.);

                            // get last mouse pos, to stick menu on
                            context.input_mut(|i| {
                                let pointer = i.pointer.latest_pos();
                                if pointer != None {
                                    shared_ui.context_menu.pos = pointer.unwrap().into();
                                }
                            });

                            // don't draw menu yet, since it'll be at (-1, -1) in this frame
                            return;
                        }
                        let s = &selections;
                        let cb = &copy_buffer;
                        context_menu_content(config, shared_ui, events, ui, id, &armature, &s, &cb);
                    })
                    .response;

                // close if clicked out of it
                if ui.input(|i| i.pointer.any_click()) && !frame.contains_pointer() {
                    shared_ui.context_menu.close();
                }
            });
    }

    // apply individual element styling once, then immediately go back to default
    macro_rules! style_once {
        ($func:expr) => {
            $func;
            default_styling(context, &config);
        };
    }

    if let Some(_pos) = context.pointer_latest_pos() {
        //if shared_ui.mobile {
        //    context
        //        .debug_painter()
        //        .circle_filled(_pos, 2., egui::Color32::GREEN);
        //}
    }

    let ik_bytes = include_bytes!("../assets/lucysir_ik.png");
    load_png(&mut shared_ui.ik_img, ik_bytes, "lucysir_ik", context);
    let lock_bytes = include_bytes!("../assets/lock.png");
    load_png(&mut shared_ui.lock_img, lock_bytes, "lock", context);

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
            let name = format!("anim_icon_{}", x.to_string());
            let tex = context.load_texture(name, color_image, Default::default());
            shared_ui.anim.icon_images.push(tex);
        }
    }
    if !shared_ui.startup_window {
        camera_bar(context, config, shared_ui, camera, events);
    }

    if !shared_ui.startup_window {
        render_bar(context, config, shared_ui, events, armature);
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
        export_modal::draw(context, shared_ui, &edit_mode, config, events, armature);
    }
    if shared_ui.lang_import_modal {
        modal::lang_import_modal(context, shared_ui, &config, events);
    }
    if shared_ui.feedback_modal {
        modal::feedback_modal(context, shared_ui, &config, events);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if shared_ui.checking_update {
        modal::modal(context, shared_ui, &config);
        let url = "https://skelform.org/download_links.json";
        let request = ureq::get(url).header("Example-Header", "header value");
        let dl_links: serde_json::Value = match request.call() {
            Ok(mut data) => {
                serde_json::from_str(&data.body_mut().read_to_string().unwrap()).unwrap()
            }
            Err(_) => serde_json::Value::default(),
        };

        if dl_links.get("version") == None {
            events.open_modal("startup.error_update", false);
        } else if dl_links != "" {
            let ver_str = dl_links["version"].as_str().unwrap();
            let this_ver_str = format!("v{}", env!("CARGO_PKG_VERSION"));
            if ver_str.trim() != this_ver_str.trim() {
                let loc = "startup.update_available";
                let str = shared_ui.loc(loc).replace("$ver", ver_str);
                events.open_polar_modal(PolarId::NewUpdate, str);
            } else {
                events.open_modal("startup.no_updates", false);
            }
        }

        shared_ui.checking_update = false;
    }
    let buffer = &copy_buffer;
    style_once!(top_panel(
        context, config, shared_ui, events, selections, armature, camera, edit_mode, buffer
    ));

    if edit_mode.anim_open {
        #[rustfmt::skip]
        style_once!(keyframe_editor::draw(context, shared_ui, input, armature, config, selections, events, &edit_mode, camera));
    }

    style_once!(armature_window::draw(
        context, events, config, armature, selections, edit_mode, shared_ui, camera
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
        enable_bone_panel = !edit_mode.setting_ik_target && !edit_mode.setting_bind_bone;
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
        .resizable(false)
        .default_width(max_size);
    if config.layout == UiLayout::Left {
        side_panel = egui::SidePanel::left(bone_panel_id)
            .resizable(true)
            .default_width(max_size);
    }
    draw_resizable_panel(
        bone_panel_id,
        side_panel.show(context, |ui| {
            ui.set_width(min_default_size);
            ui.add_enabled_ui(enable_bone_panel, |ui| {
                let gradient = config.colors.gradient.into();
                ui.gradient(ui.ctx().content_rect(), Color32::TRANSPARENT, gradient);

                let scroll_area = egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
                scroll_area.show(ui, |ui| {
                    let sel = &selections;
                    if selections.bone_idx != usize::MAX {
                        #[rustfmt::skip]
                        bone_panel::draw(selected_bone.clone(),ui,selections,shared_ui,armature,config,events,&input,edit_mode,);
                    } else if armature.sel_anim(&sel) != None && sel.anim_frame != -1 {
                        keyframe_panel::draw(ui, &selections, &armature, events, shared_ui, config);
                    } else if armature.bones.len() > 1 {
                        ui.heading("Armature Shortcuts");
                        ui.add_space(10.);
                        #[rustfmt::skip] {
                            keyboard_shortcut(ui, shared_ui.loc("settings_modal.keyboard.next_bone"), config.keys.next_bone);
                            keyboard_shortcut(ui, shared_ui.loc("settings_modal.keyboard.prev_bone"), config.keys.prev_bone);
                            keyboard_shortcut(ui, shared_ui.loc("settings_modal.keyboard.toggle_bone_fold"), config.keys.toggle_bone_fold);
                        };
                    } else {
                        ui.add_space(5.);
                        empty_armature_starters(shared_ui, config, ui);
                    }
                });
            });
            shared_ui.bone_panel_rect = Some(ui.min_rect());
        }),
        events,
        context,
        camera,
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

            shared_ui.render_bar.pos.x = armature_panel.right() + 7.;
            shared_ui.camera_bar.pos.x =
                bone_panel.left() - shared_ui.camera_bar.scale.x - ((6. * 3.3) as f32).ceil();
            if keyframe_panel != None && edit_mode.anim_open {
                shared_ui.render_bar.pos.y = keyframe_panel.unwrap().top();
                shared_ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared_ui.render_bar.pos.y = context.content_rect().bottom();
                shared_ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared_ui.render_bar.pos.y -= shared_ui.render_bar.scale.y + 15.;
            shared_ui.camera_bar.pos.y -= shared_ui.camera_bar.scale.y + 15.;
        }
        UiLayout::Right => {
            shared_ui.edit_bar.pos.x = bone_panel.left() - shared_ui.edit_bar.scale.x - 28.;
            shared_ui.edit_bar.pos.y = top_panel.bottom();

            shared_ui.anim_bar.pos = Vec2::new(0., top_panel.bottom());

            shared_ui.render_bar.pos.x = 0.;
            shared_ui.camera_bar.pos.x = bone_panel.left() - shared_ui.camera_bar.scale.x - 21.;
            if keyframe_panel != None && edit_mode.anim_open {
                shared_ui.render_bar.pos.y = keyframe_panel.unwrap().top();
                shared_ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared_ui.render_bar.pos.y = context.content_rect().bottom();
                shared_ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared_ui.render_bar.pos.y -= shared_ui.render_bar.scale.y + 15.;
            shared_ui.camera_bar.pos.y -= shared_ui.camera_bar.scale.y + 15.;
        }
        UiLayout::Left => {
            shared_ui.edit_bar.pos.x = bone_panel.right();
            shared_ui.edit_bar.pos.y = top_panel.bottom();

            shared_ui.anim_bar.pos.x = context.content_rect().right() - shared_ui.anim_bar.scale.x;
            shared_ui.anim_bar.pos.y = top_panel.bottom();

            shared_ui.render_bar.pos.x = bone_panel.right() + 7.;
            shared_ui.camera_bar.pos.x =
                context.content_rect().right() - shared_ui.camera_bar.scale.x - 15.;
            if keyframe_panel != None && edit_mode.anim_open {
                shared_ui.render_bar.pos.y = keyframe_panel.unwrap().top();
                shared_ui.camera_bar.pos.y = keyframe_panel.unwrap().top();
            } else {
                shared_ui.render_bar.pos.y = context.content_rect().bottom();
                shared_ui.camera_bar.pos.y = context.content_rect().bottom();
            }
            shared_ui.render_bar.pos.y -= shared_ui.render_bar.scale.y + 15.;
            shared_ui.camera_bar.pos.y -= shared_ui.camera_bar.scale.y + 15.;
        }
    }

    if selections.bone_idx != usize::MAX {
        edit_mode_bar(
            context, armature, selections, edit_mode, events, shared_ui, config,
        );
    }

    if armature.bones.len() > 0 {
        animate_bar(context, shared_ui, edit_mode, events);
    }

    // check if mouse is on ui
    //
    // this check always returns false on mouse click, so it's only checked when the mouse isn't clicked
    if !input.left_down && camera.on_ui != context.is_pointer_over_area() {
        events.toggle_pointer_on_ui(context.is_pointer_over_area());
    }

    // show ID of vertex being hovered
    if selections.hovering_vert_id != -1 && !camera.on_ui {
        let mouse = input.mouse / shared_ui.scale;
        let pos = egui::Pos2::new(mouse.x, mouse.y - 13.);
        let str = format!("#{}", selections.hovering_vert_id);
        let painter = context.debug_painter();
        painter.debug_text(pos, egui::Align2::CENTER_CENTER, egui::Color32::GREEN, str);
    }

    // show hovered bone's name
    let hbid = selections.hovering_bone_id;
    let sel_bone = armature.sel_bone(selections);
    if hbid != -1 && (sel_bone == None || hbid != sel_bone.unwrap().id) {
        let mouse = input.mouse / shared_ui.scale;
        let pos = egui::Pos2::new(mouse.x, mouse.y - 13.);
        let bones = &armature.bones;
        let bone = bones.iter().find(|b| b.id == hbid);
        let str = format!("{}", bone.unwrap().name);
        let painter = context.debug_painter();
        painter.debug_text(pos, egui::Align2::CENTER_CENTER, egui::Color32::GREEN, str);
    }

    // show hovering triangle helpers
    if selections.hovering_tri_dur > 25 {
        let mouse = input.mouse / shared_ui.scale;
        let pos = egui::Pos2::new(mouse.x, mouse.y - 20.);
        let str = shared_ui.loc("bone_panel.mesh_deformation.hovering_tri_tooltip");
        let painter = context.debug_painter();
        painter.debug_text(pos, egui::Align2::CENTER_CENTER, egui::Color32::GREEN, str);
    }

    // show hovering line helpers
    if selections.hovering_line_dur > 25 {
        let mouse = input.mouse / shared_ui.scale;
        let pos = egui::Pos2::new(mouse.x, mouse.y - 20.);
        let str = shared_ui.loc("bone_panel.mesh_deformation.hovering_line_tooltip");
        let painter = context.debug_painter();
        painter.debug_text(pos, egui::Align2::CENTER_CENTER, egui::Color32::GREEN, str);
    }

    macro_rules! helper_text {
        ($text:expr, $offset:expr) => {
            let align = egui::Align2::CENTER_CENTER;
            //let font = egui::FontId::default();
            let mouse_pos = input.mouse / shared_ui.scale + $offset;
            let painter = context.debug_painter();

            //// drop-shadow
            //let gap = 1.;
            //let pos = (mouse_pos + Vec2::new(gap, gap)).into();
            //painter.text(pos, align, $text, font.clone(), egui::Color32::BLACK);

            // helper text
            let point_col = config.colors.center_point.into();
            painter.debug_text(mouse_pos.into(), align, point_col, $text);
        };
    }

    if armature.sel_bone(&sel) == None {
        return;
    }

    // show which temporary mode will activate on press
    if !input.left_down {
        if let Some(temporary) = &edit_mode.temporary {
            if *temporary == EditModes::Move {
                helper_text!("Hold to Move", Vec2::new(0., -10.));
            } else if *temporary == EditModes::Rotate {
                helper_text!("Hold to Rotate", Vec2::new(0., -10.));
            } else if *temporary == EditModes::Scale {
                helper_text!("Hold to Scale", Vec2::new(0., -10.));
            }
        }
    }

    if selections.bone_ids.len() == 1 {
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
            let helper_str = format!("⏵ w: {}{}", formatted.to_string(), padding);
            helper_text!(helper_str.to_string(), offset);

            let offset = Vec2::new(-1., -38.);
            let formatted = (selected_bone.scale.y * 100.).round() / 100.;
            let mut padding = "";
            if formatted.to_string() == "1" {
                padding = ".00";
            }
            let helper_str = format!("h: {}{}\n     ⏶", &formatted.to_string(), padding);
            helper_text!(helper_str.to_string(), offset);
        }
    }
}

pub fn process_inputs(
    context: &Context,
    input: &mut InputStates,
    shared_ui: &mut crate::Ui,
    config: &Config,
    edit_mode: &mut EditMode,
    events: &mut EventState,
    camera: &Camera,
    selections: &SelectionState,
    armature: &mut Armature,
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

        if shared_ui.context_menu.id == "" {
            input.left_clicked = i.pointer.primary_clicked();
        }
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
            input.mouse_prev_left = input.mouse;
        } else {
            shared_ui.started_edit_dragging = false;
        }
        input.mouse_prev = input.mouse;
        if shared_ui.mobile {
            input.left_clicked = i.pointer.any_pressed();
        }
        if let Some(mouse) = i.pointer.latest_pos() {
            input.mouse = mouse.into();
            input.mouse *= shared_ui.scale;
        }

        shared_ui.edited_dragging = false;

        // disabled: no dragging on web for now, until cursor locking is figured out
        let can_drag;
        #[cfg(not(target_arch = "wasm32"))]
        {
            can_drag = true;
        }
        #[cfg(target_arch = "wasm32")]
        {
            can_drag = false;
        }

        // don't record prev mouse on first frame of touch as it
        // goes all over the place
        if i.any_touches() && i.pointer.primary_pressed() {
            input.mouse_prev = input.mouse;
        }

        if i.raw_scroll_delta.y != 0. {
            input.scroll_delta = i.raw_scroll_delta.y;

            let timeline_mod = config.keys.timeline_zoom_mode.modifiers;
            let timeline_mode = i.modifiers.matches_any(timeline_mod);
            if timeline_mode && shared_ui.pointer_on_timeline {
                // zoom timeline instead of scrolling
                shared_ui.anim.timeline_zoom -= input.scroll_delta / 10.;
                shared_ui.anim.timeline_zoom = shared_ui.anim.timeline_zoom.min(10.).max(0.1);
            } else if !camera.on_ui {
                events.cam_zoom_scroll();
            }
        }

        edit_mode.sel_time += i.time as f32 - edit_mode.time;
        edit_mode.time = i.time as f32;

        // dragging inputs
        let drag_mod = shared_ui.drag_modifier;
        if !(can_drag && shared_ui.rename_id != "" && input.mouse_init != None && drag_mod != 0.) {
            return;
        }
        let diff = input.mouse_init.unwrap() - input.mouse;
        let vel = input.mouse - input.mouse_prev;
        if !(shared_ui.edit_value != None && vel.x.abs() > 0.) {
            return;
        }
        match shared_ui.edit_value.as_ref().unwrap().parse::<f32>() {
            Ok(mut output) => {
                #[cfg(target_arch = "wasm32")]
                {
                    output += vel.x * shared_ui.drag_modifier;
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    output -= diff.x * shared_ui.drag_modifier;
                }
                *shared_ui.edit_value.as_mut().unwrap() = output.to_string();

                // since save_edited_bone won't save on drag, this will be set to true on
                // the frame after save_edited_bone() has been called
                shared_ui.edited_dragging = shared_ui.started_edit_dragging;

                // save bone just before beginning the drag
                if !shared_ui.started_edit_dragging {
                    events.save_edited_bone(selections.bone_idx);
                    shared_ui.started_edit_dragging = true;
                }
            }
            _ => {}
        };
    });

    // de-focus input if it has dragging enabled.
    // prevents dragging from persisting outside of the input
    if input.left_pressed && shared_ui.drag_modifier != 0. {
        shared_ui.drag_modifier = 0.;
        context.memory_mut(|m| {
            m.request_focus("".into());
        });
    }
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
    mouse_button_as_key(input, egui::PointerButton::Primary, egui::Key::F31);
    mouse_button_as_key(input, egui::PointerButton::Secondary, egui::Key::F32);
    mouse_button_as_key(input, egui::PointerButton::Middle, egui::Key::F33);
    mouse_button_as_key(input, egui::PointerButton::Extra1, egui::Key::F34);
    mouse_button_as_key(input, egui::PointerButton::Extra2, egui::Key::F35);

    // cancel key
    let ui = &shared_ui;
    #[rustfmt::skip]
    let modal_open = ui.styles_modal || ui.polar_modal || ui.settings_modal
        || ui.export_modal || ui.lang_import_modal || ui.feedback_modal;
    if input.consume_shortcut(&config.keys.cancel) {
        if shared_ui.context_menu.id != "" && !shared_ui.context_menu.hide {
            shared_ui.context_menu.close();
        } else if edit_mode.setting_ik_target {
            events.toggle_setting_ik_target(0);
        } else if edit_mode.setting_bind_bone {
            events.toggle_setting_bind_bone(0);
        } else if shared_ui.modal {
            shared_ui.modal = false;
        } else if shared_ui.polar_modal {
            shared_ui.polar_modal = false;
        } else if modal_open {
            shared_ui.styles_modal = false;
            shared_ui.settings_modal = false;
            shared_ui.atlas_modal = false;
            shared_ui.export_modal = false;
            shared_ui.feedback_modal = false;
            shared_ui.lang_input = "".to_string();
        } else {
            events.unselect_all();
        }
        shared_ui.context_menu.id = "".to_string();
    }

    if shared_ui.feedback_modal {
        return;
    }

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

    if input.consume_shortcut(&config.keys.delete) {
        // delete selected bone(s)
        if selections.bone_idx != usize::MAX {
            events.open_polar_modal(PolarId::DeleteBone, shared_ui.loc("polar.delete_bone"));
            let context_id = &format!("bone_{}", selections.bone_idx);
            shared_ui.context_menu.show(context_id);
            shared_ui.context_menu.hide = true;
        }
    }

    if input.consume_shortcut(&config.keys.save) {
        #[cfg(target_arch = "wasm32")]
        {
            let saving = shared_ui.saving.lock().unwrap().clone();
            utils::save_web(armature, camera, edit_mode, saving);
        }
        #[cfg(not(target_arch = "wasm32"))]
        utils::save_native(shared_ui);
    }

    if input.consume_shortcut(&config.keys.export) {
        events.open_export_modal();
    }

    #[cfg(not(target_arch = "wasm32"))]
    if input.consume_shortcut(&config.keys.save_as) && !shared_ui.startup_window {
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
        if selections.anim_frame != -1 {
            events.copy_keyframes_in_frame(selections.anim_frame);
        } else if selections.bone_idx != usize::MAX {
            events.copy_bone(selections.bone_idx);
        }
    }

    // paste shortcut
    if input.consume_shortcut(&config.keys.paste) {
        if selections.anim_frame != -1 {
            events.paste_keyframes_on_frame(selections.anim_frame);
        } else {
            events.paste_bone(selections.bone_idx);
        }
    }

    if input.consume_shortcut(&config.keys.transform_move) {
        events.edit_mode_move();
    }
    if input.consume_shortcut(&config.keys.transform_rotate) {
        events.edit_mode_rotate();
    }
    if input.consume_shortcut(&config.keys.transform_scale) {
        events.edit_mode_scale();
    }
    if input.consume_shortcut(&config.keys.toggle_animation) && armature.bones.len() > 0 {
        events.toggle_anim_panel_open(if edit_mode.anim_open { 0 } else { 1 });
    }

    let mod_key = &config.keys.edit_modifier.modifiers;
    let holding_edit_mod = input.modifiers.matches_any(*mod_key);
    if holding_edit_mod && !edit_mode.holding_edit_mod {
        events.toggle_edit_modifying(1);
    } else if !holding_edit_mod && edit_mode.holding_edit_mod {
        events.toggle_edit_modifying(0);
    }

    if input.consume_shortcut(&config.keys.next_bone) && armature.bones.len() > 0 {
        let mut idx;
        if selections.bone_idx == usize::MAX {
            idx = armature.bones.len() - 1;
        } else {
            idx = if selections.bone_idx == 0 {
                armature.bones.len() - 1
            } else {
                selections.bone_idx - 1
            };
        }
        while armature.is_bone_folded(armature.bones[idx].id) {
            idx = idx.overflowing_sub(1).0;
            if idx == usize::MAX {
                break;
            }
        }
        events.select_bone(idx, false);
    }

    if input.consume_shortcut(&config.keys.prev_bone) && armature.bones.len() > 0 {
        let mut idx;
        if selections.bone_idx == usize::MAX {
            idx = 0;
        } else {
            let is_last = selections.bone_idx == armature.bones.len() - 1;
            idx = if is_last { 0 } else { selections.bone_idx + 1 };
        }
        while armature.is_bone_folded(armature.bones[idx].id) {
            idx += 1;
            if idx > armature.bones.len() - 1 {
                idx = 0;
                break;
            }
        }
        events.select_bone(idx, false);
    }

    if input.consume_shortcut(&config.keys.toggle_bone_fold) {
        let bone = &armature.sel_bone(selections);
        if *bone != None {
            events.toggle_bone_folded(selections.bone_idx, !bone.unwrap().folded);
        }
    }

    if input.consume_shortcut(&config.keys.toggle_edit_vertices) {
        let bone = armature.sel_bone(selections);
        if bone != None && armature.tex_of(bone.unwrap().id) != None {
            events.toggle_showing_mesh(if edit_mode.showing_mesh { 0 } else { 1 });
        }
    }

    let snap_key = &config.keys.edit_snap.modifiers;
    let holding_edit_snap = input.modifiers.matches_any(*snap_key);
    if holding_edit_snap && !edit_mode.holding_edit_snap {
        events.toggle_edit_snapping(1);
    } else if !holding_edit_snap && edit_mode.holding_edit_snap {
        events.toggle_edit_snapping(0);
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

    let none = egui::Modifiers::NONE;

    if input.pointer.button_pressed(button) {
        input.keys_down.insert(fake_key);
        input.events.push(egui::Event::Key {
            key: fake_key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: none,
        });
    }

    if input.pointer.button_released(button) {
        input.keys_down.remove(&fake_key);
        input.events.push(egui::Event::Key {
            key: fake_key,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: none,
        });
    }
}

fn context_menu_content(
    config: &Config,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
    ui: &mut egui::Ui,
    context_id: String,
    armature: &Armature,
    selections: &SelectionState,
    copy_buffer: &CopyBuffer,
) {
    let raw_split = shared_ui.context_id_parsed();
    let split: Vec<String> = raw_split.iter().map(|s| s.to_string()).collect();
    let id = &split[0];
    if id == "bone" {
        ui.context_rename(shared_ui, &config, context_id.clone());
        let delete_bone = PolarId::DeleteBone;
        ui.context_delete(shared_ui, &config, events, "delete_bone", delete_bone);
        if ui.context_button("Copy Bone", &config).clicked() {
            events.copy_bone(split[1].parse().unwrap());
            shared_ui.context_menu.close();
        }
        if ui.context_button("Paste Bone", &config).clicked() {
            events.paste_bone(split[1].parse().unwrap());
            shared_ui.context_menu.close();
        }
    } else if id == "style" {
        ui.context_rename(shared_ui, config, context_id);
        let str = "delete_style";
        ui.context_delete(shared_ui, config, events, str, PolarId::DeleteStyle);
    } else if id == "tex" {
        ui.context_rename(shared_ui, &config, context_id);
        ui.context_delete(shared_ui, &config, events, "delete_tex", PolarId::DeleteTex);
    } else if id == "anim" {
        ui.context_rename(shared_ui, config, context_id);
        let del_anim = PolarId::DeleteAnim;
        ui.context_delete(shared_ui, config, events, "delete_anim", del_anim);
        let duplicate_str = shared_ui.loc("keyframe_editor.duplicate");
        if ui.context_button(&duplicate_str, config).clicked() {
            events.duplicate_anim(split[1].parse().unwrap());
            shared_ui.context_menu.close();
        }
    } else if id == "keyframe" {
        if ui.context_button("Copy Keyframe", &config).clicked() {
            events.copy_keyframe(split[4].parse().unwrap());
            shared_ui.context_menu.close();
        }
        if ui.context_button("Paste Keyframe", &config).clicked() {
            events.paste_keyframes_on_frame(split[3].parse().unwrap());
            shared_ui.context_menu.close();
        }
    } else if id == "kfline" {
        // copy option, if there are any keyframes in this frame
        let frame = split[1].parse().unwrap();
        let anim = armature.sel_anim(&selections).unwrap();
        let has_kf = anim.keyframes.iter().find(|kf| kf.frame == frame) != None;
        if has_kf && ui.context_button("Copy Keyframes", &config).clicked() {
            events.copy_keyframes_in_frame(frame);
            shared_ui.context_menu.close();
        }

        // paste option, if there are keyframes in copy buffer
        if copy_buffer.keyframes.len() > 0
            && ui.context_button("Paste Keyframes", &config).clicked()
        {
            events.paste_keyframes_on_frame(split[1].parse().unwrap());
            shared_ui.context_menu.close();
        }

        // immediately close menu if there's nothing to show
        if !has_kf && copy_buffer.keyframes.len() == 0 {
            shared_ui.context_menu.close();
        }
    } else if id == "kfdiamond" {
        if ui.context_button("Copy Keyframes", &config).clicked() {
            events.copy_keyframes_in_frame(split[1].parse().unwrap());
            shared_ui.context_menu.close();
        }
        if ui.context_button("Paste Keyframes", &config).clicked() {
            events.paste_keyframes_on_frame(split[1].parse().unwrap());
            shared_ui.context_menu.close();
        }
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
    copy_buffer: &CopyBuffer,
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
            menu_file_button(
                ui,
                &config,
                shared_ui,
                events,
                &selections,
                &armature,
                edit_mode,
                &camera,
            );
            menu_edit_button(ui, &config, &shared_ui, selections, events, copy_buffer);
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
                if top_bar_button(ui, str_user_docs, None, &mut offset, config, true, s_ui)
                    .clicked()
                {
                    utils::open_docs(true, "");
                }
                let str_dev_docs = &shared_ui.loc("top_bar.help.dev_docs");
                if top_bar_button(ui, str_dev_docs, None, &mut offset, config, true, s_ui).clicked()
                {
                    utils::open_docs(false, "");
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let str_binary = &shared_ui.loc("top_bar.help.binary_folder");
                    if top_bar_button(ui, str_binary, None, &mut offset, config, true, s_ui)
                        .clicked()
                    {
                        match open::that(utils::bin_path()) {
                            Err(_) => {}
                            Ok(file) => file,
                        };
                    }
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let str_config = &shared_ui.loc("top_bar.help.config_folder");
                    if top_bar_button(ui, str_config, None, &mut offset, config, true, s_ui)
                        .clicked()
                    {
                        match open::that(config_path().parent().unwrap()) {
                            Err(_) => {}
                            Ok(file) => file,
                        };
                    }
                }
            });

            let str_feedback = title!(&shared_ui.loc("top_bar.feedback"));
            let button = ui.menu_button(str_feedback, |ui| ui.close());
            if button.response.clicked() {
                shared_ui.feedback_modal = true;
            }

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
    fn skf_button(&mut self, text: impl Into<egui::WidgetText>) -> egui::Response {
        self.add(egui::Button::new(text).corner_radius(egui::CornerRadius::ZERO))
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn sized_skf_button(
        &mut self,
        size: impl Into<egui::Vec2>,
        text: impl Into<egui::WidgetText>,
    ) -> egui::Response {
        self.add_sized(
            size,
            egui::Button::new(text).corner_radius(egui::CornerRadius::ZERO),
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
        let hand = egui::CursorIcon::PointingHand;
        let label = self
            .add(egui::Button::selectable(false, text))
            .on_hover_cursor(hand);

        if label.contains_pointer() || label.has_focus() {
            return label.highlight();
        }

        label
    }

    fn context_button(&mut self, text: &str, config: &Config) -> egui::Response {
        let self_width = self.available_width();
        let button = self
            .allocate_ui([0., 0.].into(), |ui| {
                ui.set_min_width(self_width);
                ui.set_height(20.);
                let width = ui.available_width();
                let mut col = config.colors.main;
                if ui.ui_contains_pointer() {
                    col = config.colors.light_accent;
                }
                egui::Frame::new().fill(col.into()).show(ui, |ui| {
                    ui.style_mut().interaction.selectable_labels = false;
                    ui.set_min_width(width);
                    ui.set_height(20.);
                    ui.horizontal(|ui| {
                        ui.add_space(5.);
                        ui.label(text);
                        ui.add_space(5.);
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

        // setup default options, as well as size (separately since it depends on UI width)
        if options == None {
            options = Some(TextInputOptions::default());
        }
        if options.as_ref().unwrap().size == Vec2::ZERO {
            options.as_mut().unwrap().size = Vec2::new(self.available_width(), 20.);
        }

        // if the input was out of focus due to selecting another, save the value
        if shared_ui.last_rename_id != shared_ui.rename_id && shared_ui.last_rename_id == id {
            let singleline =
                egui::TextEdit::singleline(shared_ui.last_edit_value.as_mut().unwrap())
                    .hint_text(options.as_ref().unwrap().placeholder.clone());
            input = self.add_sized(options.as_ref().unwrap().size, singleline);
            shared_ui.last_rename_id = "".to_string();
            return (true, shared_ui.last_edit_value.clone().unwrap(), input);
        }

        if options.as_ref().unwrap().focus && !shared_ui.input_focused {
            shared_ui.edit_value = Some(value.clone());
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

            // save this as the last edited input, so its value can be saved if focus is lost
            // due to selecting another input
            shared_ui.last_edit_value = shared_ui.edit_value.clone();
            shared_ui.last_rename_id = shared_ui.rename_id.clone();

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
                shared_ui.last_rename_id = "".to_string();
                return (false, value, input);
            } else if self.input(|i| i.key_pressed(egui::Key::Enter)) {
                entered = true;
            }

            if input.lost_focus() {
                entered = true;
                shared_ui.rename_id = "".to_string();
            }

            if entered {
                let mut final_value = shared_ui.edit_value.as_ref().unwrap();
                if final_value == "" {
                    final_value = &options.as_ref().unwrap().default;
                }
                shared_ui.input_focused = false;
                shared_ui.rename_id = "".to_string();
                shared_ui.last_rename_id = "".to_string();
                return (true, final_value.clone(), input);
            }
        }

        if options.as_ref().unwrap().focus && !shared_ui.input_focused {
            shared_ui.input_focused = true;
            input.request_focus();
            shared_ui.edit_value = Some(value.clone());
            if shared_ui.mobile {
                open_mobile_input(shared_ui.edit_value.clone().unwrap());
            }
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
        let default_size = Vec2::new(40., 20.);
        if options == None {
            options = Some(TextInputOptions {
                size: default_size,
                ..Default::default()
            })
        } else if options.as_ref().unwrap().size == Vec2::new(0., 0.) {
            options.as_mut().unwrap().size = default_size;
        }

        let mod_value = (value * modifier).to_string();
        let (edited, mut str_value, input) =
            self.text_input(id.clone(), shared_ui, mod_value, options.clone());
        if edited {
            shared_ui.rename_id = "".to_string();
            shared_ui.last_rename_id = "".to_string();
            if str_value == "" {
                str_value = "0".to_string();
            }

            match str_value.parse::<f32>() {
                Ok(output) => return (true, output / modifier, input),
                Err(_) => return (false, value, input),
            }
        }

        if shared_ui.rename_id == id {
            shared_ui.drag_modifier = options.as_ref().unwrap().drag_modifier;
            // save edit if dragging input
            if shared_ui.edited_dragging && options.unwrap().drag_modifier != 0. {
                match shared_ui.edit_value.as_ref().unwrap().parse::<f32>() {
                    Ok(output) => return (true, output / modifier, input),
                    Err(_) => return (false, value, input),
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

    fn context_rename(&mut self, shared_ui: &mut crate::Ui, config: &Config, id: String) {
        let str = shared_ui.loc("rename");
        if self.context_button(&str, config).clicked() {
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
        let str = shared_ui.loc("delete");
        if self.context_button(&str, config).clicked() {
            let str_del = &shared_ui.loc(&format!("polar.{}", loc_code)).clone();
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
                top_bar_button(ui, $name, $kb, &mut offset, &config, true, &shared_ui)
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
            utils::save_web(
                armature,
                camera,
                edit_mode,
                shared_ui.saving.lock().unwrap().clone(),
            );
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
            events.open_export_modal();
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
                top_bar_button(ui, $name, $kb, &mut offset, &config, true, &shared_ui)
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
    selections: &SelectionState,
    events: &mut EventState,
    copy_buffer: &CopyBuffer,
) {
    let mut offset = 0.;
    let str_edit = &shared_ui.loc("top_bar.edit.heading");
    let title = egui::RichText::new(str_edit).color(config.colors.text);
    ui.menu_button(title, |ui| {
        ui.set_width(90.);
        let str_undo = &shared_ui.loc("top_bar.edit.undo");
        let key_undo = Some(&config.keys.undo);
        #[rustfmt::skip]
        if top_bar_button(ui, str_undo, key_undo, &mut offset, &config, true, &shared_ui).clicked() {
            events.undo();
            ui.close();
        };
        let str_redo = &shared_ui.loc("top_bar.edit.redo");
        let key_redo = Some(&config.keys.redo);
        #[rustfmt::skip]
        if top_bar_button(ui, str_redo, key_redo, &mut offset, &config, true, &shared_ui).clicked() {
            events.redo();
            ui.close();
        };

        let str_copy = &shared_ui.loc("top_bar.edit.copy");
        let key_copy = Some(&config.keys.copy);
        let can_copy = selections.anim_frame != -1 || selections.bone_idx != usize::MAX;
        #[rustfmt::skip]
        let button = top_bar_button(ui, str_copy, key_copy, &mut offset, &config, can_copy, &shared_ui);
        if can_copy && button.clicked() {
            if selections.anim_frame != -1 {
                events.copy_keyframes_in_frame(selections.anim_frame);
            } else if selections.bone_idx != usize::MAX {
                events.copy_bone(selections.bone_idx);
            }
            ui.close();
        }

        let can_paste = copy_buffer.bones.len() > 0 || copy_buffer.keyframes.len() > 0;
        let str_paste = &shared_ui.loc("top_bar.edit.paste");
        let key_paste = Some(&config.keys.paste);
        #[rustfmt::skip]
        let button = top_bar_button(ui, str_paste, key_paste, &mut offset, &config, can_paste, &shared_ui);
        if can_paste && button.clicked() {
            if selections.anim_frame != -1 {
                events.paste_keyframes_on_frame(selections.anim_frame);
            } else {
                events.paste_bone(selections.bone_idx);
            }
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
    config: &Config,
) {
    let mut has_ik = true;
    let sel = selections.clone();
    if let Some(bone) = armature.sel_bone(&sel) {
        has_ik = bone.ik_family_id != -1
            && !bone.ik_disabled
            && armature.bone_eff(bone.id) != JointEffector::Start;
    }

    // edit mode window
    #[rustfmt::skip]
    let window = egui::Window::new("Mode").resizable(false).title_bar(false).max_width(100.).movable(false)
        .current_pos(egui::Pos2::new(
            shared_ui.edit_bar.pos.x + 7.5,
            shared_ui.edit_bar.pos.y - 1.,
        ));
    window.show(egui_ctx, |ui| {
        let keys = &config.keys;
        ui.horizontal(|ui| {
            macro_rules! edit_mode_button {
                ($label:expr, $edit_mode:expr, $event:ident, $check:expr, $key:expr) => {
                    ui.add_enabled_ui($check, |ui| {
                        let mut str = egui::text::LayoutJob::default();
                        ui::job_text(&format!("{} ", $label), None, &mut str);
                        let mut col = config.colors.text;
                        col -= Color::new(50, 50, 50, 0);
                        ui::job_text(&$key, Some(col.into()), &mut str);
                        if selection_button(str, edit_mode.current == $edit_mode, ui).clicked() {
                            events.$event()
                        };
                    })
                };
            }
            let ikd = !edit_mode.showing_mesh && !has_ik;
            type E = EditModes;

            let key_move = keys.transform_move.display();
            let key_rotate = keys.transform_rotate.display();
            let key_scale = keys.transform_scale.display();
            let move_str = &shared_ui.loc("move");
            let rotate_str = &shared_ui.loc("rotate");
            let scale_str = &shared_ui.loc("scale");
            edit_mode_button!(move_str, E::Move, edit_mode_move, ikd, key_move);
            edit_mode_button!(rotate_str, E::Rotate, edit_mode_rotate, ikd, key_rotate);
            edit_mode_button!(scale_str, E::Scale, edit_mode_scale, ikd, key_scale);
        });
        shared_ui.edit_bar.scale = ui.min_rect().size().into();

        // display edit features via shortcuts (snapping, etc) when actively editing
        macro_rules! edit_feature {
            ($str:expr, $key:expr, $pressed:expr) => {
                ui.horizontal(|ui| {
                    let col = if $pressed {
                        let mut col = config.colors.light_accent;
                        col -= Color::new(10, 10, 10, 0);
                        col
                    } else {
                        config.colors.main
                    };
                    egui::Frame::new().fill(col.into()).show(ui, |ui| {
                        ui.label($str);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label($key.display());
                        });
                    });
                });
            };
        }

        // shortcuts won't do anything on IK bones, so don't show
        if has_ik {
            return;
        }

        if edit_mode.is_moving || edit_mode.is_rotating || edit_mode.is_scaling {
            shared_ui.cursor_icon = egui::CursorIcon::Crosshair;
        }

        if edit_mode.is_moving {
            let str = "Snap X/Y";
            edit_feature!(str, config.keys.edit_snap, edit_mode.holding_edit_snap);
        } else if edit_mode.is_rotating {
            let str = format!("Snap to {}°", config.rot_snap_step);
            edit_feature!(str, config.keys.edit_snap, edit_mode.holding_edit_snap);
        } else if edit_mode.is_scaling {
            let str = "Snap X/Y";
            edit_feature!(str, config.keys.edit_snap, edit_mode.holding_edit_snap);
            let str = "Maintain aspect ratio";
            edit_feature!(str, config.keys.edit_modifier, edit_mode.holding_edit_mod);
        }
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

fn render_bar(
    egui_ctx: &Context,
    config: &Config,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
    armature: &Armature,
) {
    // check which toggles are eligible, and return if none are
    let mut eligibles = vec![false, false, false, false, false];
    for bone in &armature.bones {
        eligibles[1] = true;
        if armature.tex_of(bone.id) != None {
            eligibles[0] = true;
            if bone.verts_edited {
                eligibles[3] = true
            } else if !bone.verts_edited {
                eligibles[4] = true
            }
        }
        if bone.parent_id != -1 {
            eligibles[2] = true;
        }
    }
    if !eligibles.contains(&true) {
        return;
    }

    let margin = 6.;
    let window = egui::Window::new("Render")
        .resizable(false)
        .title_bar(false)
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
            shared_ui.render_bar.pos.x,
            shared_ui.render_bar.pos.y,
        ));
    window.show(egui_ctx, |ui| {
        let mut hovered = false;
        let mut idx = -1;

        if shared_ui.render_bar.expanded {
            ui.set_width(101.);
            //ui.set_height(122.);
        }

        macro_rules! button {
            ($field:expr, $str:expr, $icon:expr) => {
                idx += 1;
                if eligibles[idx as usize] {
                    let mut bg_col = config.colors.light_accent;
                    if !$field {
                        bg_col -= Color::new(20, 20, 20, 0);
                    }
                    let size;
                    let margin;
                    let font_size;
                    #[rustfmt::skip]
                    let str = if shared_ui.render_bar.prev_expanded {
                        size = [90., 20.];
                        font_size = 14;
                        margin = egui::Margin { top: 3, bottom: 3, right: 5, left: 6 };
                        format!("{} {}", $icon, $str)
                    } else {
                        size = [8., 20.];
                        font_size = 8;
                        margin = egui::Margin { top: 3, bottom: 3, right: 5, left: 6 };
                                                                                                                                        $icon.to_string()
                                                                                                                                    };
                    let cursor = egui::CursorIcon::PointingHand;
                    let mut col = if $field { config.colors.light_accent } else { config.colors.dark_accent };
                    if shared_ui.hovering_render_toggle == idx {
                        col += Color::new(25, 25, 25, 0);
                    }

                    let rect = egui::Rect::from_min_size(egui::Pos2::new(ui.cursor().left(), ui.cursor().top()), egui::Vec2::new(size[0], 21.));
                    let button = ui.interact(rect, $str.into(), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                    egui::Frame::new()
                        .inner_margin(margin)
                        .fill(col.into())
                        .show(ui, |ui| {
                            ui.set_width(size[0]);
                            ui.scope(|ui| {
                                ui.style_mut().interaction.selectable_labels = false;
                                ui.label(egui::RichText::new(str).size(font_size as f32));
                            });
                        })
                        .response;
                    if button.hovered() || button.has_focus() {
                        shared_ui.hovering_render_toggle = idx;
                        hovered = true;
                        shared_ui.render_bar.expanded = true;
                    }
                    if button.on_hover_cursor(cursor).clicked() {
                        $field = !$field;
                        events.update_render_options();
                    }
                }
            };
        }

        button!(shared_ui.render_textures, "Textures", "🖻");
        button!(shared_ui.render_points, "Points", "⏺");
        button!(shared_ui.render_kites, "Kites", "♦");
        button!(shared_ui.render_mesh_wf, "Mesh Wires", "⬟");
        button!(shared_ui.render_rects, "Rects", "⬛");

        shared_ui.render_bar.prev_expanded = shared_ui.render_bar.expanded;
        if !hovered {
            shared_ui.hovering_render_toggle = -1;
            shared_ui.render_bar.expanded = ui.ui_contains_pointer();
        }
        shared_ui.render_bar.scale = ui.min_rect().size().into();
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
    let mut hovered = false;
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
        // invisible focus element, for expansion via Tab
        let focus_id = ui.make_persistent_id("camera_bar_focus");
        let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(0.0, 0.0));
        let focus_response = ui.interact(rect, focus_id, egui::Sense::focusable_noninteractive());
        if focus_response.has_focus() {
            shared_ui.camera_bar.expanded = true;
            hovered = true;
        }

        if !shared_ui.camera_bar.expanded {
            ui.scope(|ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.label(egui::RichText::new("🎥").size(16.));
            });
        } else {
            ui.set_width(60.);
            ui.set_height(66.);

            // show inputs on 2nd frame of being expanded, to prevent visible layout jittering
            if shared_ui.camera_bar.prev_expanded {
                macro_rules! input {
                    ($float:expr, $id:expr, $label:expr, $tip:expr) => {
                        ui.horizontal(|ui| {
                            ui.label($label).on_hover_text(&shared_ui.loc($tip));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let id = $id.to_string();
                                let (edited, value, input) =
                                    ui.float_input(id, shared_ui, $float.round(), 1., None);
                                if input.has_focus() {
                                    hovered = true;
                                    shared_ui.camera_bar.expanded = true;
                                }
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
            }
        }

        shared_ui.camera_bar.prev_expanded = shared_ui.camera_bar.expanded;
        shared_ui.camera_bar.expanded = hovered || ui.ui_contains_pointer();
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

pub fn selection_button(
    text: impl Into<egui::WidgetText>,
    selected: bool,
    ui: &mut egui::Ui,
) -> egui::Response {
    let mut cursor = egui::CursorIcon::PointingHand;
    let mut bg_col = ui.visuals().widgets.active.weak_bg_fill;

    if selected {
        cursor = egui::CursorIcon::Default;
        bg_col = bg_col + egui::Color32::from_rgb(20, 20, 20);
    }

    let button = egui::Button::new(text)
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
    enabled: bool,
    shared_ui: &crate::Ui,
) -> egui::Response {
    let height = 20.;

    let rect = egui::Rect::from_min_size(
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset),
        egui::Vec2::new(ui.min_rect().width(), height),
    );

    // give appropriate sense to allow/prevent button from being tab-focused
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::empty()
    };

    let response: egui::Response = ui.allocate_rect(rect, sense);
    let painter = ui.painter_at(ui.min_rect());

    let col = if (response.hovered() || response.has_focus()) && enabled {
        config.colors.light_accent.into()
    } else {
        egui::Color32::TRANSPARENT
    };
    painter.rect_filled(rect, egui::CornerRadius::ZERO, col);

    // text
    let pos =
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset) + egui::vec2(5., 2.);
    let mut text_col = config.colors.text;
    if !enabled {
        text_col -= Color::new(60, 60, 60, 0);
    }
    let font = egui::FontId::new(13., egui::FontFamily::Proportional);
    painter.text(
        pos,
        egui::Align2::LEFT_TOP,
        text,
        font.clone(),
        text_col.into(),
    );

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

#[derive(PartialEq, Clone)]
pub struct TextInputOptions {
    pub size: Vec2,
    pub focus: bool,
    pub placeholder: String,
    pub default: String,
    pub drag_modifier: f32, // 0 - disabled
}

impl Default for TextInputOptions {
    fn default() -> Self {
        TextInputOptions {
            size: Vec2::new(0., 0.),
            focus: false,
            placeholder: "".to_string(),
            default: "".to_string(),
            drag_modifier: 0.,
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
    camera: &Camera,
) {
    if let Some(resize) = context.read_response(egui::Id::new(id).with("__resize")) {
        if resize.hovered() || panel.response.hovered() {
            if !camera.on_ui {
                events.toggle_pointer_on_ui(true);
            }
        }
    }
}

pub fn load_png(
    handle: &mut Option<egui::TextureHandle>,
    bytes: &[u8],
    id: &str,
    context: &Context,
) {
    if *handle != None {
        return;
    }
    let img = image::load_from_memory(bytes).unwrap();
    let egui_img = egui::ColorImage::from_rgba_unmultiplied(
        [img.width() as usize, img.height() as usize],
        &img.into_rgba8(),
    );
    *handle = Some(context.load_texture(id, egui_img, Default::default()))
}

pub fn keyboard_shortcut(ui: &mut egui::Ui, label: String, key: egui::KeyboardShortcut) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(key.display());
        });
    });
}

pub fn empty_armature_starters(shared_ui: &mut crate::Ui, config: &Config, ui: &mut egui::Ui) {
    ui.label(shared_ui.loc("armature_panel.empty_armature"));
    ui.add_space(5.);
    let str = egui::RichText::new("User Documentation").color(config.colors.link);
    if ui.clickable_label(str).clicked() {
        utils::open_docs(false, "index.html");
    }
    ui.add_space(5.);
    let str = egui::RichText::new("Starter Guide").color(config.colors.link);
    if ui.clickable_label(str).clicked() {
        utils::open_docs(false, "starter-guide/main.html");
    }
}
