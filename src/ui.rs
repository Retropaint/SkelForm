//! Core UI (user interface) logic.

use std::borrow::BorrowMut;

use egui::{Context, Rangef, Stroke, Ui};

use crate::shared::*;
use crate::{armature_window, bone_window, keyframe_editor, Vec2};

// UI colors
pub const COLOR_ACCENT: egui::Color32 = egui::Color32::from_rgb(65, 43, 87);

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
    style_once!(keyframe_editor::draw(context, shared));
    style_once!(armature_window::draw(context, shared));
    style_once!(bone_window::draw(context, shared));

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
        .show(context, |ui| {
            ui.horizontal(|ui| {
                macro_rules! button {
                    ($name:expr, $mode:expr) => {
                        let mut col = COLOR_ACCENT;
                        if shared.edit_mode == $mode {
                            let add = 20;
                            col = egui::Color32::from_rgb(col.r() + add, col.g() + add, col.b() + add);
                        }
                        if ui.add(egui::Button::new($name).fill(col)).clicked() {
                            shared.edit_mode = $mode;
                        }
                    };
                }
                button!("Translate", 0);
                button!("Rotate", 1);
                button!("Scale", 2);
            });
        });
}
 fn top_panel(egui_ctx: &Context, shared: &mut Shared) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(36, 24, 43);
    egui_ctx.set_visuals(visuals);
    egui::TopBottomPanel::top("test").show(egui_ctx, |ui| {
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
        });
    });
}

/// Default styling to apply across all UI.
pub fn default_styling(context: &Context) {
    let mut visuals = egui::Visuals::dark();

    // remove rounded corners on windows
    visuals.window_corner_radius = egui::CornerRadius::ZERO;

    let main_color = egui::Color32::from_rgb(46, 31, 56);

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

pub fn button(text: &str, ui: &mut egui::Ui) -> egui::Response{
    ui.add(egui::Button::new(text).fill(COLOR_ACCENT))
}
