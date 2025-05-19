//! Core UI (user interface) logic.

use std::io::Read;

use egui::{Color32, Context, Shadow, Stroke};

use crate::*;

macro_rules! ui_color {
    ($name:ident, $r:expr, $g:expr, $b:expr) => {
        pub const $name: Color32 = Color32::from_rgb($r, $g, $b);
    };
}

// UI colors
#[rustfmt::skip] ui_color!(COLOR_ACCENT,             65, 46, 105);
#[rustfmt::skip] ui_color!(COLOR_BORDER,             44, 36, 64);
#[rustfmt::skip] ui_color!(COLOR_BORDER_HOVERED,     84, 59, 138);
#[rustfmt::skip] ui_color!(COLOR_MAIN,               32, 25, 46);
#[rustfmt::skip] ui_color!(COLOR_MAIN_DARK,          28, 20, 42);
#[rustfmt::skip] ui_color!(COLOR_TEXT,               180, 180, 180);
#[rustfmt::skip] ui_color!(COLOR_TEXT_SELECTED,      210, 210, 210);
#[rustfmt::skip] ui_color!(COLOR_FRAMELINE,          80, 60, 130);
#[rustfmt::skip] ui_color!(COLOR_FRAMELINE_HOVERED,  108, 80, 179);
#[rustfmt::skip] ui_color!(COLOR_FRAMELINE_PASTLAST, 50, 41, 74);

/// The `main` of this module.
pub fn draw(context: &Context, shared: &mut Shared) {
    default_styling(context);

    // apply individual element styling once, then immediately go back to default
    macro_rules! style_once {
        ($func:expr) => {
            $func;
            default_styling(context);
        };
    }

    let size = 18;
    #[allow(unused_assignments)]
    let mut full_img = image::DynamicImage::default();
    if shared.ui.anim.icon_images.len() == 0 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            full_img = image::load_from_memory(include_bytes!("../anim_icons.png")).unwrap();
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some((pixels, dims)) = file_reader::load_image_wasm("img-anim-icons".to_string())
            {
                let buffer = image::ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_vec(
                    dims.x as u32,
                    dims.y as u32,
                    pixels,
                )
                .unwrap();
                let mut full_img = image::DynamicImage::ImageRgba8(buffer);
            }
        }

        if full_img.width() > 0 {
            let mut x = 0;
            while full_img.width() > 0 && x < full_img.width() - 1 {
                let img = full_img.crop(x, 0, 18, 18).into_rgba8();
                x += size;
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [img.width() as usize, img.height() as usize],
                    img.as_flat_samples().as_slice(),
                );
                let tex = context.load_texture("anim_icons", color_image, Default::default());
                shared.ui.anim.icon_images.push(tex);
            }
        }
    }

    if shared.ui.has_state(UiState::PolarModal) {
        polar_dialog(shared, context);
    }
    if shared.ui.has_state(UiState::Modal) {
        modal_dialog(shared, context);
    }
    if shared.ui.has_state(UiState::ImageModal) {
        modal_image(shared, context);
    }

    // close modals on pressing escape
    if shared.input.is_pressing(winit::keyboard::KeyCode::Escape) {
        shared.ui.set_state(UiState::ImageModal, false);
    }

    // Although counter-intuitive, mouse inputs are recorded here.
    // This is because egui can detect all of them even if they were not on the UI itself.
    // To determine if the mouse is on the UI, winit's mouse input is used instead (see input.rs).
    context.input(|i| {
        shared.input.mouse_left_prev = shared.input.mouse_left;
        shared.input.mouse_right_prev = shared.input.mouse_right;
        if i.pointer.primary_down() {
            shared.input.mouse_left += 1;
        } else {
            shared.input.mouse_left = -1;
        }

        if i.pointer.secondary_down() {
            shared.input.mouse_right += 1;
        } else {
            shared.input.mouse_right = -1;
        }
    });

    context.set_cursor_icon(shared.cursor_icon);
    shared.cursor_icon = egui::CursorIcon::Default;

    style_once!(top_panel(context, shared));

    camera_bar(context, shared);

    if shared.ui.anim.open {
        style_once!(keyframe_editor::draw(context, shared));
    } else {
        shared.ui.camera_bar_pos.y = shared.window.y;
    }

    style_once!(armature_window::draw(context, shared));

    // right side panel
    let response = egui::SidePanel::right("Bone")
        .resizable(true)
        .max_width(250.)
        .show(context, |ui| {
            draw_gradient(
                ui,
                ui.ctx().screen_rect(),
                Color32::TRANSPARENT,
                COLOR_MAIN_DARK,
            );
            ui.set_min_width(175.);

            if shared.selected_bone_idx != usize::MAX {
                bone_panel::draw(ui, shared);
            } else if shared.ui.anim.selected_frame != -1 {
                keyframe_panel::draw(ui, shared);
            }

            shared.ui.animate_mode_bar_pos.x = ui.min_rect().left();
            shared.ui.camera_bar_pos.x = ui.min_rect().left();
        })
        .response;
    if response.hovered() {
        shared.input.on_ui = true;
    }

    if shared.selected_bone_idx != usize::MAX {
        edit_mode_bar(context, shared);
    }

    if shared.armature.bones.len() > 0 {
        animate_bar(context, shared);
    }

    // check if mouse is on ui
    //
    // this check always returns false on mouse click, so it's only checked when the mouse isn't clicked
    if shared.input.mouse_left == -1 {
        shared.input.on_ui = context.is_pointer_over_area();
    }
}

fn top_panel(egui_ctx: &Context, shared: &mut Shared) {
    egui::TopBottomPanel::top("top_bar")
        .frame(egui::Frame {
            fill: COLOR_MAIN,
            stroke: Stroke::new(0., COLOR_ACCENT),
            ..Default::default()
        })
        .show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                let mut offset = 0.;
                macro_rules! str {
                    ($string:expr) => {
                        $string.to_string()
                    };
                }
                ui.menu_button("File", |ui| {
                    if top_bar_button(ui, str!("Import"), str!("I"), &mut offset).clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        utils::open_import_dialog();
                        ui.close_menu();
                    }
                    if top_bar_button(ui, str!("Save"), str!("S"), &mut offset).clicked() {
                        #[cfg(not(target_arch = "wasm32"))]
                        utils::open_save_dialog();
                        ui.close_menu();
                    }
                    if top_bar_button(ui, str!("Export Video"), str!("E"), &mut offset).clicked() {
                        // check if ffmpeg exists and complain if it doesn't
                        let mut ffmpeg = false;
                        match std::process::Command::new("ffmpeg").arg("-version").output() {
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
                            let headline = "ffmpeg is not available.\n\nPlease ensure it is installed and in your $PATH.";
                            shared.ui.open_modal(headline.to_string(), false);
                            return
                        }

                        // complain if there's no proper animation to export
                        if shared.ui.anim.selected == usize::MAX {
                            if shared.armature.animations.len() == 0
                                || shared.armature.animations[0].keyframes.len() == 0
                            {
                                shared.ui.open_modal("No animation available.".to_string(), false);
                                return;
                            } else {
                                shared.ui.anim.selected = 0;
                            }
                        } else if shared.last_keyframe() == None {
                            shared.ui.open_modal("No animation available.".to_string(), false);
                            return;
                        }
                        shared.recording = true;
                        shared.done_recording = true;
                        shared.ui.anim.playing = true;
                        shared.select_frame(0);
                        shared.ui.anim.loops = 1;
                        shared.ui.anim.elapsed = Some(std::time::Instant::now());
                        ui.close_menu();
                    }
                });
                offset = 0.;
                ui.menu_button("View", |ui| {
                    if top_bar_button(ui, str!("Zoom In"), str!("="), &mut offset).clicked() {
                        set_zoom(shared.camera.zoom - 0.1, shared);
                        ui.close_menu();
                    }
                    if top_bar_button(ui, str!("Zoom Out"), str!("-"), &mut offset).clicked() {
                        set_zoom(shared.camera.zoom + 0.1, shared);
                        ui.close_menu();
                    }
                });
                shared.ui.edit_bar_pos.y = ui.min_rect().bottom();
                shared.ui.animate_mode_bar_pos.y = ui.min_rect().bottom();
            });
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
            shared.ui.edit_bar_pos.y + 1.,
        ))
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                macro_rules! edit_mode_button {
                    ($label:expr, $edit_mode:expr) => {
                        if selection_button($label, shared.edit_mode == $edit_mode, ui).clicked() {
                            shared.edit_mode = $edit_mode;
                        };
                    };
                }
                edit_mode_button!("Move", EditMode::Move);
                edit_mode_button!("Rotate", EditMode::Rotate);
                edit_mode_button!("Scale", EditMode::Scale);
            });
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
            shared.ui.animate_mode_bar_pos.y + 1.,
        ))
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                if selection_button("Armature", !shared.ui.anim.open, ui).clicked() {
                    shared.ui.anim.open = false;
                }
                if selection_button("Animation", shared.ui.anim.open, ui).clicked() {
                    shared.ui.anim.open = true;
                }
                shared.ui.animate_mode_bar_scale = ui.min_rect().size().into();
            });
        });
}

fn camera_bar(egui_ctx: &Context, shared: &mut Shared) {
    egui::Window::new("Camera")
        .resizable(false)
        .title_bar(false)
        .max_width(100.)
        .movable(false)
        .frame(egui::Frame{
            fill: COLOR_MAIN_DARK,
            inner_margin: 6.into(),
            stroke: Stroke{ width: 1., color: COLOR_BORDER},
            ..Default::default()
        })
        .current_pos(egui::Pos2::new(
            shared.ui.camera_bar_pos.x - shared.ui.camera_bar_scale.x - 21.,
            shared.ui.camera_bar_pos.y - shared.ui.camera_bar_scale.y - 15.,
        ))
        .show(egui_ctx, |ui| {
            macro_rules! input {
                ($element:expr, $float:expr, $id:expr, $edit_id:expr, $modifier:expr, $ui:expr, $label:expr) => {
                    if $label != "" {
                        $ui.label($label);
                    }
                    (_, $float) = bone_panel::float_input($id.to_string(), shared, $ui, $float, $modifier);
                };
            }

            ui.horizontal(|ui| {
                ui.label("Camera:");
                input!(shared.camera.pos, shared.camera.pos.x, "cam_pos_y", 0, 1., ui, "X");                
                input!(shared.camera.pos, shared.camera.pos.y, "cam_pos_x", 0, 1., ui, "Y");
            });

            ui.horizontal(|ui| {
                ui.label("Zoom:");
                input!(shared.camera.zoom, shared.camera.zoom, "cam_zoom", 0, 1., ui, "");
            });

            shared.ui.camera_bar_scale = ui.min_rect().size().into();
        }).unwrap().response;
}

/// Default styling to apply across all UI.
pub fn default_styling(context: &Context) {
    let mut visuals = egui::Visuals::dark();

    // remove rounded corners on windows
    visuals.window_corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.open.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;

    visuals.window_shadow = Shadow::NONE;
    visuals.window_fill = COLOR_MAIN;
    visuals.panel_fill = COLOR_MAIN;
    visuals.window_stroke = egui::Stroke::new(1., COLOR_BORDER);

    visuals.widgets.active.bg_fill = COLOR_BORDER;
    visuals.widgets.hovered.bg_fill = COLOR_BORDER;
    visuals.widgets.inactive.bg_fill = COLOR_BORDER;

    visuals.widgets.active.weak_bg_fill = COLOR_ACCENT;
    visuals.widgets.hovered.weak_bg_fill = COLOR_ACCENT;
    visuals.widgets.inactive.weak_bg_fill = COLOR_ACCENT;

    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1., COLOR_BORDER_HOVERED);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1., COLOR_BORDER_HOVERED);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1., COLOR_BORDER_HOVERED);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1., COLOR_BORDER_HOVERED);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1., COLOR_BORDER);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1., COLOR_BORDER);

    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1., COLOR_BORDER);

    visuals.override_text_color = Some(COLOR_TEXT);

    context.set_visuals(visuals);
}

pub fn set_zoom(zoom: f32, shared: &mut Shared) {
    shared.camera.zoom = zoom;
    if shared.camera.zoom < 0.1 {
        shared.camera.zoom = 0.1;
    }
}

pub fn button(text: &str, ui: &mut egui::Ui) -> egui::Response {
    ui.add(
        egui::Button::new(text)
            .fill(COLOR_ACCENT)
            .corner_radius(egui::CornerRadius::ZERO),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn selection_button(text: &str, selected: bool, ui: &mut egui::Ui) -> egui::Response {
    let mut bg_col = COLOR_ACCENT;
    let mut cursor = egui::CursorIcon::PointingHand;
    let mut text_col = COLOR_TEXT;

    if selected {
        bg_col = bg_col + egui::Color32::from_rgb(20, 20, 20);
        cursor = egui::CursorIcon::Default;
        text_col = COLOR_TEXT_SELECTED;
    }

    let button = egui::Button::new(egui::RichText::new(text).color(text_col))
        .fill(bg_col)
        .corner_radius(egui::CornerRadius::ZERO);

    ui.add(button).on_hover_cursor(cursor)
}

pub fn polar_dialog(shared: &mut Shared, ctx: &egui::Context) {
    egui::Modal::new(shared.ui.polar_id.clone().to_string().into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: COLOR_MAIN,
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., COLOR_ACCENT),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(250.);
            ui.label(shared.ui.headline.to_string());
            ui.add_space(20.);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if button("No", ui).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    shared.ui.set_state(UiState::PolarModal, false);
                }
                if button("Yes", ui).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    shared.ui.set_state(UiState::PolarModal, false);
                    match shared.ui.polar_id {
                        PolarId::DeleteBone => {
                            // remove all children of this bone as well
                            let mut children =
                                vec![shared.armature.bones[shared.selected_bone_idx].clone()];
                            armature_window::get_all_children(
                                &shared.armature.bones,
                                &mut children,
                                &shared.armature.bones[shared.selected_bone_idx],
                            );
                            children.reverse();
                            for bone in &children {
                                shared.delete_bone(bone.id);
                            }

                            // remove all references to this bone and it's children from all animations
                            for bone in &children {
                                for anim in &mut shared.armature.animations {
                                    for i in 0..anim.keyframes.len() {
                                        if anim.keyframes[i].bone_id == bone.id {
                                            anim.keyframes.remove(i);
                                        }
                                    }
                                }
                            }
                            shared.selected_bone_idx = usize::MAX;
                        }
                        PolarId::Exiting => shared.ui.set_state(UiState::Exiting, true),
                    }
                }
            });
        });
}

pub fn modal_dialog(shared: &mut Shared, ctx: &egui::Context) {
    egui::Modal::new(shared.ui.polar_id.to_string().into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: COLOR_MAIN,
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., COLOR_ACCENT),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(250.);
            ui.label(shared.ui.headline.to_string());
            ui.add_space(20.);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if !shared.ui.has_state(UiState::ForcedModal) {
                    if ui.button("OK").clicked() {
                        shared.ui.set_state(UiState::Modal, false);
                        shared.ui.headline = "".to_string();
                    }
                }
            });
        });
}

pub fn modal_image(shared: &mut Shared, ctx: &egui::Context) {
    egui::Modal::new("test".into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: COLOR_MAIN,
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., COLOR_ACCENT),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(250.);
            ui.set_height(250.);
            ui.heading("Select Image");

            modal_x(ui, || {
                shared.ui.set_state(UiState::ImageModal, false);
            });

            ui.horizontal(|ui| {
                if selection_button("Import", shared.ui.has_state(UiState::RemovingTexture), ui)
                    .clicked()
                {
                    #[cfg(not(target_arch = "wasm32"))]
                    bone_panel::open_file_dialog();

                    #[cfg(target_arch = "wasm32")]
                    bone_panel::toggleFileDialog(true);
                }

                let label = if shared.ui.has_state(UiState::RemovingTexture) {
                    "Pick"
                } else {
                    "Remove"
                };
                if button(label, ui).clicked() {
                    shared.ui.set_state(
                        UiState::RemovingTexture,
                        shared.ui.has_state(UiState::RemovingTexture),
                    );
                }
            });

            let mut offset = 0.;
            let mut height = 0.;
            let mut current_height = 0.;
            for i in 0..shared.ui.texture_images.len() {
                // limit size
                let mut size = shared.armature.textures[i].size;
                let max = 50.;
                if size.x > max {
                    let aspect_ratio = size.y / size.x;
                    size.x = max;
                    size.y = max * aspect_ratio;
                } else if size.y > max {
                    let aspect_ratio = size.x / size.y;
                    size.y = max;
                    size.x = max * aspect_ratio;
                }

                // record tallest texture of this row
                if height > size.y {
                    height = size.y;
                }

                let pos = egui::pos2(
                    ui.min_rect().left() + offset,
                    ui.min_rect().top() + 50. + current_height,
                );

                let rect = egui::Rect::from_min_size(pos, size.into());
                let response: egui::Response = ui.allocate_rect(rect, egui::Sense::click());

                // show highlight on hover
                if response.hovered() {
                    ui.painter_at(ui.min_rect()).rect_filled(
                        rect,
                        egui::CornerRadius::ZERO,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60),
                    );
                }

                // draw image
                egui::Image::new(&shared.ui.texture_images[i]).paint_at(ui, rect);

                if response.clicked() {
                    if shared.ui.has_state(UiState::RemovingTexture) {
                        shared.remove_texture(i as i32);
                        shared.ui.set_state(UiState::RemovingTexture, false);
                        // stop the loop to prevent index errors
                        break;
                    } else {
                        shared.selected_bone_mut().unwrap().tex_idx = i as i32;
                        shared.ui.set_state(UiState::ImageModal, false);
                    }
                }

                offset += size.x;
                // go to next row if there's no space
                if offset > ui.available_width() {
                    offset = 0.;
                    current_height += height;
                    height = 0.;
                }
            }
        });
}

pub fn top_bar_button(
    ui: &mut egui::Ui,
    text: String,
    kb_key: String,
    offset: &mut f32,
) -> egui::Response {
    let height = 20.;
    let rect = egui::Rect::from_min_size(
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset),
        egui::Vec2::new(100., height),
    );
    let response: egui::Response = ui.allocate_rect(rect, egui::Sense::click());
    let painter = ui.painter_at(ui.min_rect());
    if response.hovered() {
        painter.rect_filled(rect, egui::CornerRadius::ZERO, COLOR_ACCENT);
    } else {
        painter.rect_filled(rect, egui::CornerRadius::ZERO, egui::Color32::TRANSPARENT);
    }

    // text
    painter.text(
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset) + egui::vec2(5., 2.),
        egui::Align2::LEFT_TOP,
        text,
        egui::FontId::default(),
        egui::Color32::GRAY,
    );

    // kb key text
    painter.text(
        egui::Pos2::new(ui.min_rect().right(), ui.min_rect().top() + *offset) + egui::vec2(-5., 2.),
        egui::Align2::RIGHT_TOP,
        kb_key,
        egui::FontId::default(),
        egui::Color32::GRAY,
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

// top-right X label for modals
pub fn modal_x<T: FnOnce()>(ui: &mut egui::Ui, after_close: T) {
    let x_rect = egui::Rect::from_min_size(ui.min_rect().right_top(), egui::Vec2::ZERO);
    if ui
        .put(x_rect, egui::Label::new(egui::RichText::new("X").size(18.)))
        .clicked()
    {
        after_close();
    }
}

pub fn draw_gradient(ui: &mut egui::Ui, rect: egui::Rect, top: Color32, bottom: Color32) {
    let mut mesh = egui::Mesh::default();

    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);

    mesh.add_triangle(0, 2, 3);
    mesh.add_triangle(0, 3, 1);

    ui.painter().add(egui::Shape::mesh(mesh));
}
