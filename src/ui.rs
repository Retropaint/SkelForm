//! Core UI (user interface) logic.

use egui::Context;

use crate::shared::Shared;
use crate::{armature_window, bone_window};

/// The `main` of this module.
pub fn draw(context: &Context, shared: &mut Shared) {
    styling(context);

    egui::TopBottomPanel::top("test").show(context, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {});
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
        });
    });

    armature_window::draw(context, shared);
    bone_window::draw(context, shared);

    // edit mode window
    egui::Window::new("Mode")
        .resizable(false)
        .max_width(100.)
        .show(context, |ui| {
            ui.horizontal(|ui| {
                macro_rules! button {
                    ($name:expr, $mode:expr) => {
                        let mut col = egui::Color32::from_rgb(60, 60, 60);
                        if shared.edit_mode == $mode {
                            col = egui::Color32::from_rgb(100, 100, 100);
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

/// General styling to apply across all UI.
pub fn styling(context: &Context) {
    let mut visuals = egui::Visuals::dark();

    // remove rounded corners on windows
    visuals.window_corner_radius = egui::CornerRadius::ZERO;

    context.set_visuals(visuals);
}

pub fn set_zoom(zoom: f32, shared: &mut Shared) {
    shared.zoom = zoom;
}
