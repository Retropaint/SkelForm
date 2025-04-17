//! Core UI (user interface) logic.

use egui::{Context, Shadow, Stroke};

use crate::{armature_window, bone_window, keyframe_editor};
use crate::{input, shared::*};

// UI colors
pub const COLOR_ACCENT: egui::Color32 = egui::Color32::from_rgb(60, 60, 60);
pub const COLOR_MAIN: egui::Color32 = egui::Color32::from_rgb(30, 30, 30);

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

    // load anim change icons
    #[cfg(not(target_arch = "wasm32"))]
    {
        let size = 18;
        if shared.ui.anim.images.len() == 0 {
            let mut full_img =
                image::load_from_memory(include_bytes!("../anim_icons.png")).unwrap();
            let mut x = 0;
            while x < full_img.width() - 1 {
                let img = full_img.crop(x, 0, 18, 18).into_rgba8();
                x += size;
                let color_image = egui::ColorImage::from_rgba_unmultiplied(
                    [img.width() as usize, img.height() as usize],
                    img.as_flat_samples().as_slice(),
                );
                let tex = context.load_texture("anim_icons", color_image, Default::default());
                shared.ui.anim.images.push(tex);
            }
        }
    }

    if shared.ui.polar_id != "" {
        polar_dialog(shared, context);
    }

    // Although counter-intuitive, mouse inputs are recorded here.
    // This is because egui can detect all of them even if they were not on the UI itself.
    // To determine if the mouse is on the UI, winit's mouse input is used instead (see input.rs).
    context.input(|i| {
        if i.pointer.primary_down() {
            if shared.input.mouse_left == -1 {
                shared.input.mouse_left = 0;
            }
        } else {
            shared.input.mouse_left = -1;
            shared.input.initial_points = vec![];
        }
        shared.input.scroll = Vec2::new(i.raw_scroll_delta.x, i.raw_scroll_delta.y);
    });

    context.set_cursor_icon(shared.cursor_icon);
    shared.cursor_icon = egui::CursorIcon::Default;

    style_once!(top_panel(context, shared));
    if shared.animating {
        style_once!(keyframe_editor::draw(context, shared));
    }
    style_once!(armature_window::draw(context, shared));
    style_once!(bone_window::draw(context, shared));

    edit_mode_bar(context, shared);
    animate_bar(context, shared);
}

fn top_panel(egui_ctx: &Context, shared: &mut Shared) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = COLOR_MAIN;
    egui_ctx.set_visuals(visuals);
    egui::TopBottomPanel::top("test")
        .frame(egui::Frame {
            fill: COLOR_MAIN,
            stroke: Stroke::new(0., COLOR_ACCENT),
            ..Default::default()
        })
        .show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    ui.horizontal(|ui| {
                        ui.set_max_width(80.);
                        if ui.button("Export").clicked() {
                            #[cfg(not(target_arch = "wasm32"))]
                            crate::utils::open_export_dialog();
                            ui.close_menu();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("E");
                        });

                        if input::is_pressing(winit::keyboard::KeyCode::KeyE, &shared) {
                            #[cfg(not(target_arch = "wasm32"))]
                            crate::utils::open_export_dialog();
                            ui.close_menu();
                        }
                    });
                });
                ui.menu_button("View", |ui| {
                    ui.horizontal(|ui| {
                        ui.set_max_width(80.);
                        if ui.button("Zoom in").clicked() {
                            set_zoom(shared.camera.zoom - 0.1, shared);
                            ui.close_menu();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("=");
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.set_max_width(80.);
                        if ui.button("Zoom out").clicked() {
                            set_zoom(shared.camera.zoom + 0.1, shared);
                            ui.close_menu();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("-");
                        });
                    })
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
                if selection_button("Move", shared.edit_mode == 0, ui).clicked() {
                    shared.edit_mode = 0;
                };
                if selection_button("Rotate", shared.edit_mode == 1, ui).clicked() {
                    shared.edit_mode = 1;
                };
                if selection_button("Scale", shared.edit_mode == 2, ui).clicked() {
                    shared.edit_mode = 2;
                };
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
                if selection_button("Armature", !shared.animating, ui).clicked() {
                    shared.animating = false;
                }
                if selection_button("Animation", shared.animating, ui).clicked() {
                    shared.animating = true;
                }
                shared.ui.animate_mode_bar_scale = ui.min_rect().size().into();
            });
        });
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

    visuals.window_shadow = Shadow::NONE;
    visuals.window_fill = COLOR_MAIN;
    visuals.panel_fill = COLOR_MAIN;
    visuals.window_stroke = egui::Stroke::new(1., COLOR_ACCENT);

    context.set_visuals(visuals);
}

pub fn styling_once<T: FnOnce(&mut egui::Visuals)>(context: &Context, changes: T) {
    let mut visuals = egui::Visuals::dark();
    changes(&mut visuals);
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
    let mut text_col = egui::Color32::from_rgb(170, 170, 170);

    if selected {
        bg_col = bg_col + egui::Color32::from_rgb(20, 20, 20);
        cursor = egui::CursorIcon::Default;
        text_col = egui::Color32::from_rgb(200, 200, 200);
    }

    let button = egui::Button::new(egui::RichText::new(text).color(text_col))
        .fill(bg_col)
        .corner_radius(egui::CornerRadius::ZERO);

    ui.add(button).on_hover_cursor(cursor)
}

pub fn polar_dialog(shared: &mut Shared, ctx: &egui::Context) {
    egui::Modal::new(shared.ui.polar_id.clone().into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: COLOR_MAIN,
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., COLOR_ACCENT),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(250.);
            ui.label(shared.ui.polar_headline.to_string());
            ui.add_space(20.);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if button("No", ui).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    shared.ui.polar_id = "".to_string();
                }
                if button("Yes", ui).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if shared.ui.polar_id == "delete_bone" {
                        // detach this bone's children, before deleting it
                        let mut children = vec![];
                        armature_window::get_all_children(
                            &shared.armature.bones,
                            &mut children,
                            &shared.armature.bones[shared.selected_bone_idx],
                        );
                        let id = shared.armature.bones[shared.selected_bone_idx].id;
                        for bone in children {
                            if bone.parent_id == id {
                                shared.find_bone_mut(bone.id).unwrap().parent_id = -1;
                            }
                        }

                        shared.armature.bones.remove(shared.selected_bone_idx);
                        shared.selected_bone_idx = usize::MAX;
                    }
                    shared.ui.polar_id = "".to_string();
                }
            });
        });
}
