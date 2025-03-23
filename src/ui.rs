//! Core user interface logic.

use egui::Context;

use crate::{armature_window, bone_window};
use crate::shared::Shared;

/// The `main` of this module.
pub fn draw(context: &Context, shared: &mut Shared) {
    egui::Window::new("lol").show(context, |ui| {
        ui.label("test");
    });

    armature_window::draw(context, shared);
    bone_window::draw(context, shared);
}
