//! Animation keyframe editor. Very early and only proof-of-concept.

use crate::*;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    egui::TopBottomPanel::bottom("Keyframe")
        .min_height(150.)
        .show(egui_ctx, |ui| {
            let full_height = ui.available_height();

            ui.horizontal(|ui| {
                ui.set_height(full_height);
                egui::Resize::default()
                    .min_height(full_height)
                    .max_height(full_height)
                    .default_width(150.)
                    .with_stroke(false)
                    .show(ui, |ui| {
                        egui::Frame::new().show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.heading("Animation");

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.add_space(5.);
                                            if ui.button("New").clicked() {
                                                shared.armature.animations.push(Animation {
                                                    name: "New_Anim".to_string(),
                                                    keyframes: vec![],
                                                })
                                            }
                                        },
                                    );
                                });
                                for a in &shared.armature.animations {
                                    if ui.button(a.name.clone()).clicked() {}
                                }
                            })
                        });
                    });
                //ui.separator();
                egui::Frame::new().show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height(ui.available_height());
                    let mut i = 0;
                    while i < 10 {
                        let painter = ui.painter_at(ui.min_rect());
                        draw_diamond(
                            &painter,
                            Vec2::new(
                                40. * i as f32 + ui.min_rect().left() + 10.,
                                ui.min_rect().top() + 12.,
                            ),
                        );
                        i += 1;
                    }
                });
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
        egui::Color32::TRANSPARENT, // Fill color (transparent)
        egui::Stroke::new(2.0, egui::Color32::WHITE), // Stroke width & color
    ));
}
