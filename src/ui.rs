//! Core UI (user interface) logic.

use std::borrow::BorrowMut;

use egui::{Context, Rangef, Shadow, Stroke, Ui};

use crate::shared::*;
use crate::{armature_window, bone_window, keyframe_editor, Vec2};

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
                ui.menu_button("File", |_ui| {});
                ui.menu_button("View", |ui| {
                    if shared.input.mouse_left != -1 && !ui.rect_contains_pointer(ui.min_rect()) {
                        ui.close_menu();
                    }

                    ui.horizontal(|ui| {
                        ui.set_max_width(80.);
                        if ui.button("Zoom in").clicked() {
                            set_zoom(shared.zoom - 0.1, shared);
                            ui.close_menu();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label("=");
                        });
                    });
                    ui.horizontal(|ui| {
                        ui.set_max_width(80.);
                        if ui.button("Zoom out").clicked() {
                            set_zoom(shared.zoom + 0.1, shared);
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
                if selection_button("Armature", shared.animating, ui).clicked() {
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
    shared.zoom = zoom;
    if shared.zoom < 0.1 {
        shared.zoom = 0.1;
    }
}

pub fn button(text: &str, ui: &mut egui::Ui) -> egui::Response {
    ui.add(
        egui::Button::new(text)
            .fill(COLOR_ACCENT)
            .corner_radius(egui::CornerRadius::ZERO),
    )
}

pub fn selection_button(text: &str, selected: bool, ui: &mut egui::Ui) -> egui::Response {
    let mut col = COLOR_ACCENT;
    if selected {
        // emilk forgot to add += for Color32
        col = col + egui::Color32::from_rgb(20, 20, 20);
    }
    ui.add(
        egui::Button::new(text)
            .fill(col)
            .corner_radius(egui::CornerRadius::ZERO),
    )
}
