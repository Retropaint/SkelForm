//! Core UI (user interface) logic.

use egui::Context;

use crate::shared::Shared;
use crate::{armature_window, bone_window};

/// The `main` of this module.
pub fn draw(context: &Context, shared: &mut Shared) {
    styling(context);

    armature_window::draw(context, shared);
    bone_window::draw(context, shared);

    egui::Window::new("Mode").show(context, |ui| {
        if ui.button("Translate").clicked() {
            shared.edit_mode = 0;
        }
        if ui.button("Rotate").clicked() {
            shared.edit_mode = 1;
        }
    });
}

/// General styling to apply across all UI.
pub fn styling(context: &Context) {
    let mut visuals = egui::Visuals::dark();

    // remove rounded corners on windows
    visuals.window_corner_radius = egui::CornerRadius::ZERO;

    context.set_visuals(visuals);
}
