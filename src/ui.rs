//! Core UI (user interface) logic.

use egui::Context;

use crate::shared::Shared;
use crate::{armature_window, bone_window};

/// The `main` of this module.
pub fn draw(context: &Context, shared: &mut Shared) {
    styling(context);

    armature_window::draw(context, shared);
    bone_window::draw(context, shared);

    // edit mode window
    egui::Window::new("Mode").show(context, |ui| {
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
    });
}

/// General styling to apply across all UI.
pub fn styling(context: &Context) {
    let mut visuals = egui::Visuals::dark();

    // remove rounded corners on windows
    visuals.window_corner_radius = egui::CornerRadius::ZERO;

    context.set_visuals(visuals);
}
