//! Animation keyframe editor. Very early and only proof-of-concept.

use std::ops::MulAssign;

use egui::{epaint::Marginf, Margin};
use ui::COLOR_ACCENT;

use crate::*;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    egui::TopBottomPanel::bottom("Keyframe")
        .min_height(150.)
        .show(egui_ctx, |ui| {
            let full_height = ui.available_height();
            ui.horizontal(|ui| {
                ui.set_height(full_height);

                // animations list
                egui::Resize::default()
                    .min_height(full_height) // make height unadjustable
                    .max_height(full_height) //
                    .default_width(150.)
                    .with_stroke(false)
                    .show(ui, |ui| {
                        egui::Frame::new().show(ui, |ui| {
                            // prevent self-resizing with ver and hor wraps
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

                // keyframe editor
                egui::Frame::new()
                    .outer_margin(egui::Margin {
                        left: 0,
                        ..Default::default()
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height(ui.available_height());

                        // bones list
                        ui.vertical(|ui| {
                            ui.add_space(30.);
                            ui.label("test");
                        });

                        ui.vertical(|ui| {
                            // diamond bar
                            egui::Frame::new().fill(COLOR_ACCENT).show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_height(20.);
                                let painter = ui.painter_at(ui.min_rect());
                                let pos = Vec2::new(
                                    ui.min_rect().left() + 10.,
                                    ui.min_rect().top() + 10.,
                                );
                                draw_diamond(&painter, pos);
                            });
                        });
                    });
            });
        });
}

fn draw_diamond(painter: &egui::Painter, pos: Vec2) {
    // Define the center and size of the diamond
    let size = 5.0; // Half of the width/height

    //let rect = egui::Rect::from_center_size(center, egui::Vec2::splat(size * 2.0));
    //let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

    //if response.dragged() {
    //  println!("Dragging!");
    //}

    // Define the four points of the diamond
    let points = vec![
        egui::Pos2::new(pos.x, pos.y - size), // Top
        egui::Pos2::new(pos.x + size, pos.y), // Right
        egui::Pos2::new(pos.x, pos.y + size), // Bottom
        egui::Pos2::new(pos.x - size, pos.y), // Left
    ];

    // Draw the diamond
    painter.add(egui::Shape::convex_polygon(
        points,
        egui::Color32::TRANSPARENT, // Fill color (transparent)
        egui::Stroke::new(2.0, egui::Color32::WHITE), // Stroke width & color
    ));
}
