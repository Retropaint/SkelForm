//! Animation keyframe editor. Very early and only proof-of-concept.

use egui::Stroke;
use ui as ui_mod;

use ui::COLOR_ACCENT;

use crate::*;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    egui::TopBottomPanel::bottom("Keyframe")
        .min_height(150.)
        .show(egui_ctx, |ui| {
            let full_height = ui.available_height();
            ui.horizontal(|ui| {
                ui.set_height(full_height);
                animations_list(ui, shared);

                if shared.ui.selected_anim == usize::MAX {
                    return;
                }

                keyframe_editor(egui_ctx, ui, shared);
            });
        });
}

fn animations_list(ui: &mut egui::Ui, shared: &mut Shared) {
    let full_height = ui.available_height();
    // animations list
    egui::Resize::default()
        .min_height(full_height) // make height unadjustable
        .max_height(full_height) //
        .default_width(150.)
        .with_stroke(false)
        .show(ui, |ui| {
            egui::Frame::new().show(ui, |ui| {
                // use a ver and hor wrap to prevent self-resizing
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Animation");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(5.);
                            if ui.button("New").clicked() {
                                shared.armature.animations.push(Animation {
                                    name: "New_Anim".to_string(),
                                    keyframes: vec![],
                                    fps: 60,
                                })
                            }
                        });
                    });
                    for (i, a) in shared.armature.animations.iter().enumerate() {
                        if ui_mod::selection_button(&a.name, i == shared.ui.selected_anim, ui)
                            .clicked()
                        {
                            shared.ui.selected_anim = i;
                        }
                    }
                })
            });
        });
}

fn keyframe_editor(egui_ctx: &egui::Context, ui: &mut egui::Ui, shared: &mut Shared) {
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
                    let pos = Vec2::new(ui.min_rect().left() + 10., ui.min_rect().top() + 10.);
                    draw_diamond(&painter, pos);
                });

                // timeline graph
                egui::Frame::new().fill(COLOR_ACCENT).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(ui.available_height());
                    let mut i = 0;
                    while i < shared.armature.animations[shared.ui.selected_anim].fps {
                        let x = i as f32 / (shared.armature.animations[shared.ui.selected_anim].fps - 1) as f32 * ui.min_rect().width() as f32;
                        let painter = ui.painter_at(ui.min_rect());
                        painter.vline(
                            ui.min_rect().left() + x,
                            egui::Rangef {
                                min: ui.min_rect().top(),
                                max: ui.min_rect().bottom(),
                            },
                            Stroke {
                                width: 2.,
                                color: egui::Color32::DARK_GRAY,
                            },
                        );
                        i += 1;
                    }
                    draw_pointing_line(egui_ctx, ui, shared);
                });
            });
        });
}

fn draw_pointing_line(egui_ctx: &egui::Context, ui: &mut egui::Ui, shared: &mut Shared) {
    // get cursor pos on the frame (or 0, 0 if can't)
    let mut cursor_x: f32;
    let mut frame: i32;
    let cursor_pos = egui_ctx.input(|i| {
        if let Some(result) = i.pointer.hover_pos() {
            result
        } else {
            egui::Pos2::new(0., 0.)
        }
    });
    cursor_x = cursor_pos.x - ui.min_rect().left();

    // get cursor % of graph
    cursor_x /= ui.min_rect().width();
    cursor_x *= (shared.armature.animations[shared.ui.selected_anim].fps - 1) as f32;

    // round frame to integer
    frame = cursor_x.round() as i32;
    cursor_x = cursor_x.round();

    // reverse process to get rounded graph line
    cursor_x /= (shared.armature.animations[shared.ui.selected_anim].fps - 1) as f32;
    cursor_x *= ui.min_rect().width();

    let painter = ui.painter_at(ui.min_rect());
    painter.vline(
        ui.min_rect().left() + cursor_x,
        egui::Rangef {
            min: ui.min_rect().top(),
            max: ui.min_rect().bottom(),
        },
        Stroke {
            width: 2.,
            color: egui::Color32::WHITE,
        },
    );
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
