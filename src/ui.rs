//! Core user interface logic

use egui::Context;

pub fn draw_ui(context: &Context) {
    egui::Window::new("lol").show(context, |ui| {
        ui.label("test");
    });
}
