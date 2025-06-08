//! Core UI (user interface) logic.

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

const HELP_LIGHT_CANT: &str = "There is already an animation! Looks like you've figured it out.\n\nTo activate the help light, please start a new project.";

const FFMPEG_ERR: &str =
    "ffmpeg is not available.\n\nPlease ensure it is installed and in your $PATH.";

/// The `main` of this module.
#[allow(unused_variables)]
pub fn draw(context: &Context, shared: &mut Shared, window_factor: f32) {
    default_styling(context);

    let scale_mod: f32;

    #[cfg(not(target_arch = "wasm32"))]
    {
        scale_mod = 1.;
    }

    #[cfg(target_arch = "wasm32")]
    {
        scale_mod = window_factor;
    }

    context.set_zoom_factor(shared.ui.scale * scale_mod);

    // apply individual element styling once, then immediately go back to default
    macro_rules! style_once {
        ($func:expr) => {
            $func;
            default_styling(context);
        };
    }

    #[cfg(feature = "mobile")]
    #[cfg(feature = "debug")]
    // visually track UI cursor, since it differs from real
    if let Some(pos) = context.pointer_latest_pos() {
        context
            .debug_painter()
            .circle_filled(pos, 2., egui::Color32::GREEN);
    }

    let anim_icon_size = 18;
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
                full_img = image::DynamicImage::ImageRgba8(buffer);
            }
        }

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

    if shared.ui.has_state(UiState::PolarModal) {
        polar_dialog(shared, context);
    }
    if shared.ui.has_state(UiState::Modal) {
        modal_dialog(shared, context);
    }
    if shared.ui.has_state(UiState::ImageModal) {
        modal_image(shared, context);
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
        shared.ui.camera_bar_pos.y = context.screen_rect().bottom();
    }

    style_once!(armature_window::draw(context, shared));

    let min_default_size = 190.;

    // right side panel
    let response = egui::SidePanel::right("Bone")
        .resizable(true)
        .max_width(250.)
        .min_width(min_default_size)
        .default_width(min_default_size)
        .show(context, |ui| {
            draw_gradient(
                ui,
                ui.ctx().screen_rect(),
                Color32::TRANSPARENT,
                COLOR_MAIN_DARK,
            );

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
            egui::menu::bar(ui, |ui| {
                menu_file_button(ui, shared);
                menu_edit_button(ui, shared);
                menu_view_button(ui, shared);

                ui.menu_button("Help", |ui| {
                    let str = if !shared.tutorial_step_is(TutorialStep::None) {
                        "Stop Help Light"
                    } else {
                        "Help Light"
                    };
                    if top_bar_button(ui, str, "", &mut offset).clicked() {
                        if shared.armature.animations.len() > 0 {
                            shared.ui.open_modal(HELP_LIGHT_CANT.to_string(), false);
                        } else {
                            if !shared.tutorial_step_is(TutorialStep::None) {
                                shared.set_tutorial_step(TutorialStep::None);
                            } else {
                                shared.start_tutorial();
                            }
                        }
                        ui.close_menu();
                    }
                });
            });

            shared.ui.edit_bar_pos.y = ui.min_rect().bottom();
            shared.ui.animate_mode_bar_pos.y = ui.min_rect().bottom();
        });
}

fn menu_file_button(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut offset = 0.;
    ui.menu_button("File", |ui| {
        if top_bar_button(ui, "Open", "O", &mut offset).clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_import_dialog(TEMP_IMPORT_PATH.to_string());
            #[cfg(target_arch = "wasm32")]
            toggleElement(true, "file-dialog".to_string());
            ui.close_menu();
        }
        if top_bar_button(ui, "Save", "Mod + S", &mut offset).clicked() {
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_save_dialog();
            #[cfg(target_arch = "wasm32")]
            utils::save_web(shared);
            ui.close_menu();
        }
        if top_bar_button(ui, "Export Video", "E", &mut offset).clicked() {
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
            shared.done_recording = true;
            shared.ui.anim.playing = true;
            shared.ui.anim.started = Some(chrono::Utc::now());
            shared.select_frame(0);
            shared.ui.anim.loops = 1;
            ui.close_menu();
        }
    });
}

fn menu_view_button(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut offset = 0.;
    ui.menu_button("View", |ui| {
        if top_bar_button(ui, "Zoom In", "=", &mut offset).clicked() {
            set_zoom(shared.camera.zoom - 0.1, shared);
        }
        if top_bar_button(ui, "Zoom Out", "-", &mut offset).clicked() {
            set_zoom(shared.camera.zoom + 0.1, shared);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            if top_bar_button(ui, "Zoom In UI", "Mod + =", &mut offset).clicked() {
                shared.ui.scale += 0.1;
            }
            if top_bar_button(ui, "Zoom Out UI", "Mod + -", &mut offset).clicked() {
                shared.ui.scale -= 0.1;
            }
        }

        #[cfg(target_arch = "wasm32")]
        if top_bar_button(ui, "Adjust UI", "", &mut offset).clicked() {
            toggleElement(true, "ui-slider".to_string());
            ui.close_menu();
        }
    });
}

fn menu_edit_button(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut offset = 0.;
    ui.menu_button("Edit", |ui| {
        if top_bar_button(ui, "Undo", "Mod + Z", &mut offset).clicked() {
            utils::undo_redo(true, shared);
            ui.close_menu();
        }
        if top_bar_button(ui, "Redo", "Mod + Y", &mut offset).clicked() {
            utils::undo_redo(false, shared);
            ui.close_menu();
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
            shared.ui.animate_mode_bar_pos.y - 1.,
        ))
        .show(egui_ctx, |ui| {
            ui.horizontal(|ui| {
                if selection_button("Armature", !shared.ui.anim.open, ui).clicked() {
                    shared.ui.anim.open = false;
                }
                let button = selection_button("Animation", shared.ui.anim.open, ui);
                draw_tutorial_rect(TutorialStep::OpenAnim, button.rect, shared, ui);
                if button.clicked() {
                    shared.ui.anim.open = true;
                    shared.start_next_tutorial_step(TutorialStep::CreateAnim);
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
            fill: COLOR_MAIN_DARK,
            inner_margin: margin.into(),
            stroke: Stroke {
                width: 1.,
                color: COLOR_BORDER,
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
                    (_, $float, _) = ui::float_input($id.to_string(), shared, $ui, $float, 1.);
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
pub fn default_styling(context: &Context) {
    let mut visuals = egui::Visuals::dark();

    // remove rounded corners on windows
    visuals.window_corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.active.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.open.corner_radius = egui::CornerRadius::ZERO;
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::ZERO;
    visuals.menu_corner_radius = egui::CornerRadius::ZERO;

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
                        PolarId::FirstTime => shared.start_tutorial(),
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
            inner_margin: egui::Margin::same(8),
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
                    toggleElement(true, "image-dialog".to_string());
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
            let mut tex_idx = -1;
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
                    tex_idx = i as i32;
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
                        shared.set_bone_tex(shared.selected_bone().unwrap().id, i);
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

            if tex_idx == -1 {
                return;
            }

            // show image info at bottom left of modal

            let labels = 2;
            let label_heights = (15 * labels) as f32;
            let label_gaps = (2 * labels) as f32;
            ui.add_space(ui.available_height() - (label_heights + label_gaps));

            let tex = &shared.armature.textures[tex_idx as usize];

            let mut name = egui::text::LayoutJob::default();
            job_text("Name: ", Some(Color32::WHITE), &mut name);
            job_text(&tex.name, None, &mut name);
            let mut size = egui::text::LayoutJob::default();
            job_text("Size: ", Some(Color32::WHITE), &mut size);
            job_text(
                &(tex.size.x.to_string() + " x " + &tex.size.y.to_string()),
                None,
                &mut size,
            );
            ui.label(name);
            ui.label(size);
        });
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
    kb_key: &str,
    offset: &mut f32,
) -> egui::Response {
    let height = 20.;
    #[allow(unused_mut)]
    let mut width = 100.;

    #[cfg(feature = "mobile")]
    {
        width *= 0.8;
    }

    let rect = egui::Rect::from_min_size(
        egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() + *offset),
        egui::Vec2::new(width, height),
    );
    let response: egui::Response = ui.allocate_rect(rect, egui::Sense::click());
    let painter = ui.painter_at(ui.min_rect());
    if response.hovered() {
        painter.rect_filled(rect, egui::CornerRadius::ZERO, COLOR_ACCENT);
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
        egui::Color32::GRAY,
    );

    let kb_font = egui::FontId::new(9., egui::FontFamily::Proportional);

    // kb key text
    #[cfg(not(feature = "mobile"))]
    painter.text(
        egui::Pos2::new(ui.min_rect().right(), ui.min_rect().top() + *offset) + egui::vec2(-5., 5.),
        egui::Align2::RIGHT_TOP,
        kb_key,
        kb_font,
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
    if shared.tutorial_step_is(step) {
        ui::draw_fading_rect(ui, rect, Color32::GOLD, 60., 2.);
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

pub fn text_input(
    id: String,
    shared: &mut Shared,
    ui: &mut egui::Ui,
    mut value: String,
    mut options: Option<TextInputOptions>,
) -> (bool, String, egui::Response) {
    let input: egui::Response;

    if options == None {
        options = Some(TextInputOptions::default());
    }

    if options.as_ref().unwrap().size == Vec2::ZERO {
        options.as_mut().unwrap().size = Vec2::new(ui.available_width(), 20.);
    }

    if options.as_ref().unwrap().focus && !shared.ui.input_focused {
        #[cfg(feature = "mobile")]
        open_mobile_input(shared.ui.edit_value.clone().unwrap());
        shared.ui.input_focused = true;
    }

    if shared.ui.rename_id != id {
        input = ui.add_sized(
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
        input = ui.add_sized(
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

        if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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
pub fn float_input(
    id: String,
    shared: &mut Shared,
    ui: &mut egui::Ui,
    value: f32,
    modifier: f32,
) -> (bool, f32, egui::Response) {
    let (edited, _, input) = text_input(
        id,
        shared,
        ui,
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
