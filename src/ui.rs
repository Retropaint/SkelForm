/// All user interface logic goes here
/// certain logic may be in renderer.rs, but only if necessary

use egui::Context;

pub fn draw_ui(context: &Context) {
    egui::Window::new("lol").show(context, |ui| {
        ui.label("test");
    });
}
