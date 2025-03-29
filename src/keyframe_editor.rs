//! Animation keyframe editor. Very early and only proof-of-concept.

use crate::*;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    egui::TopBottomPanel::bottom("bruh")
        .min_height(150.)
        .show(egui_ctx, |ui| {

            egui::Frame::new().show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.set_min_height(20.);
                let pos = Vec2::from(ui.min_rect().center());
                let mut i = 0;
                while i < 10 {
                    let painter = ui.painter_at(ui.min_rect());
                    draw_diamond(&painter, Vec2::new(40. * i as f32, pos.y));
                    i += 1;
                }
            });

            egui::Frame::new()
                .outer_margin(5.)
                .fill(egui::Color32::DARK_GRAY)
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    let painter = ui.painter_at(ui.min_rect());
                    let mut i = 0;
                    while i < 10 {
                        painter.vline(
                            40. * i as f32,
                            egui::Rangef::new(ui.min_rect().top(), ui.min_rect().bottom()),
                            egui::Stroke::new(1., egui::Color32::WHITE),
                        );
                        i += 1;
                    }
                });
        });
}

fn draw_diamond(painter: &egui::Painter, pos: Vec2) {
    let center = egui::Pos2::new(pos.x, pos.y); // (x, y)

    // Define the center and size of the diamond
    let size = 5.0; // Half of the width/height

    //let rect = egui::Rect::from_center_size(center, egui::Vec2::splat(size * 2.0));
    //let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

    //if response.dragged() {
    //  println!("Dragging!");
    //}

    // Define the four points of the diamond
    let points = vec![
        egui::Pos2::new(center.x, center.y - size), // Top
        egui::Pos2::new(center.x + size, center.y), // Right
        egui::Pos2::new(center.x, center.y + size), // Bottom
        egui::Pos2::new(center.x - size, center.y), // Left
    ];

    // Draw the diamond
    painter.add(egui::Shape::convex_polygon(
        points,
        egui::Color32::TRANSPARENT,             // Fill color (transparent)
        egui::Stroke::new(2.0, egui::Color32::WHITE), // Stroke width & color
    ));
}
