use egui::Context;

pub fn draw_ui(context: &Context) {
    egui::Window::new("lol").show(context, |ui| {
        //ui.checkbox(&mut self.panels_visible, "Show Panels");
    });
}
