//! Core user interface logic.

use egui::Context;

/// The `main` of this module.
pub fn draw(context: &Context) {
    egui::Window::new("lol").show(context, |ui| {
        ui.label("test");
    });
}
