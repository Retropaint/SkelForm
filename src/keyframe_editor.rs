//! Animation keyframe editor. Very early and only proof-of-concept.

use egui::Stroke;
use ui as ui_mod;

use ui::COLOR_ACCENT;

use crate::*;

const LINE_OFFSET: f32 = 30.;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    egui::TopBottomPanel::bottom("Keyframe")
        .min_height(150.)
        .resizable(true)
        .show(egui_ctx, |ui| {
            let full_height = ui.available_height();
            ui.horizontal(|ui| {
                ui.set_height(full_height);
                animations_list(ui, shared);

                if shared.ui.anim.selected == usize::MAX {
                    return;
                }

                timeline_editor(egui_ctx, ui, shared);
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
                                    ..Default::default()
                                });
                                let idx = shared.armature.animations.len() - 1;
                                shared.ui.original_name = "".to_string();
                                shared.ui.rename_id = "animation ".to_owned() + &idx.to_string();
                            }
                        });
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for i in 0..shared.armature.animations.len() {
                            // initialize renaming
                            let rename_id = "animation ".to_owned() + &i.to_string();
                            let name = &mut shared.armature.animations[i].name;
                            if shared.ui.check_renaming(&rename_id, name, ui) {
                                continue;
                            }

                            let button =
                                ui_mod::selection_button(&name, i == shared.ui.anim.selected, ui);
                            if button.clicked() {
                                shared.ui.anim.selected = i;
                                shared.ui.anim.selected_frame = 0;
                            }
                            if button.double_clicked() {
                                shared.ui.rename_id = rename_id;
                            }
                        }
                    });
                })
            });
        });
}

fn timeline_editor(egui_ctx: &egui::Context, ui: &mut egui::Ui, shared: &mut Shared) {
    egui::Frame::new()
        .outer_margin(egui::Margin {
            left: 0,
            ..Default::default()
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());

            let mut bone_tops: Vec<BoneTops> = vec![];

            // bones list
            let mut bone_ids: Vec<i32> = vec![];
            ui.vertical(|ui| {
                ui.add_space(30.);
                for i in 0..shared.selected_animation().keyframes.len() {
                    for j in 0..shared.selected_animation().keyframes[i].bones.len() {
                        // skip if bone was already added
                        if bone_ids.iter().position(|id| {
                            shared.selected_animation().keyframes[i].bones[j].id == *id
                        }) != None
                        {
                            continue;
                        } else {
                            bone_ids.push(shared.selected_animation().keyframes[i].bones[j].id);
                        }

                        let bone_id = shared.selected_animation().keyframes[i].bones[j].id;
                        ui.label(shared.find_bone(bone_id).unwrap().name.clone());

                        let mut pos_top = 0.;
                        let mut rot_top = 0.;

                        // add changes to the list
                        if shared.selected_animation().keyframes[i].bones[j].pos != Vec2::ZERO {
                            ui.horizontal(|ui| {
                                ui.add_space(20.);
                                let text = ui.label("Pos");
                                pos_top = text.rect.top();
                            });
                        }
                        if shared.selected_animation().keyframes[i].bones[j].rot != 0. {
                            ui.horizontal(|ui| {
                                ui.add_space(20.);
                                let text = ui.label("Rot");
                                rot_top = text.rect.top();
                            });
                        }
                        bone_tops.push(shared::BoneTops {
                            id: bone_id,
                            pos_top,
                            rot_top,
                        });
                    }
                }
            });

            // diamond bar
            ui.vertical(|ui| {
                egui::Frame::new().fill(COLOR_ACCENT).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(20.);
                    for (_, kf) in shared.armature.animations[shared.ui.anim.selected]
                        .keyframes
                        .iter()
                        .enumerate()
                    {
                        // wait for the last diamond's position to be determined
                        // on the next frame
                        let pos = Vec2::new(
                            ui.min_rect().left() + shared.ui.anim.lines_x[kf.frame as usize],
                            ui.min_rect().top() + 10.,
                        );
                        draw_diamond(ui, pos);
                    }
                });

                // The options bar has to be at the bottom, but it needs to be created first
                // so that the remaining height can be taken up by timeline graph.
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    // options
                    egui::Frame::new().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height(20.);
                        ui.horizontal(|ui| {
                            if ui.button("+").clicked() {
                                shared.ui.anim.timeline_zoom -= 0.1;
                            }
                            if ui.button("-").clicked() {
                                shared.ui.anim.timeline_zoom += 0.1;
                            }
                        });
                    });

                    // timeline graph
                    egui::Frame::new().fill(COLOR_ACCENT).show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.set_height(ui.available_height());
                        draw_frame_lines(egui_ctx, ui, shared, bone_tops);
                    });
                });
            });
        });
}

/// Draw all lines representing frames in the timeline.
fn draw_frame_lines(
    egui_ctx: &egui::Context,
    ui: &egui::Ui,
    shared: &mut Shared,
    bone_tops: Vec<BoneTops>,
) {
    // get cursor pos on the graph (or 0, 0 if can't)
    let mut cursor_x = -1.;
    if ui.ui_contains_pointer() {
        let cursor_pos = egui_ctx.input(|i| {
            if let Some(result) = i.pointer.hover_pos() {
                result
            } else {
                egui::Pos2::new(0., 0.)
            }
        });
        cursor_x = cursor_pos.x - ui.min_rect().left();
    }

    let zoomed_width = ui.min_rect().width() / shared.ui.anim.timeline_zoom;
    let hitbox = zoomed_width / shared.selected_animation().fps as f32 / 2.;

    shared.ui.anim.lines_x = vec![];

    let mut x = 0.;
    let mut i = 0;
    while x < ui.min_rect().width() {
        x = i as f32 / shared.selected_animation().fps as f32 * zoomed_width + LINE_OFFSET;

        shared.ui.anim.lines_x.push(x);

        let mut color = egui::Color32::DARK_GRAY;
        if shared.ui.anim.selected_frame == i {
            color = egui::Color32::WHITE;
        } else if cursor_x < x + hitbox && cursor_x > x - hitbox {
            color = egui::Color32::GRAY;

            // select this frame if clicked
            if shared.input.mouse_left == 0 {
                shared.ui.anim.selected_frame = i;
            }
        }

        // draw the line!
        let painter = ui.painter_at(ui.min_rect());
        painter.vline(
            ui.min_rect().left() + x,
            egui::Rangef {
                min: ui.min_rect().top(),
                max: ui.min_rect().bottom(),
            },
            Stroke { width: 2., color },
        );

        i += 1;
    }

    // draw per-change diamonds
    for kf in &shared.armature.animations[shared.ui.anim.selected].keyframes {
        for b in &kf.bones {
            let x = ui.min_rect().left() + shared.ui.anim.lines_x[kf.frame as usize];
            let mut pos_top = 0.;
            let mut rot_top = 0.;
            for bt in &bone_tops {
                if bt.id == b.id {
                    pos_top = bt.pos_top;
                    rot_top = bt.rot_top;
                }
            }
            if b.pos != Vec2::ZERO {
                let pos = Vec2::new(x, pos_top + 10.);
                draw_diamond(ui, pos);
            }
            if b.rot != 0. {
                let pos = Vec2::new(x, rot_top + 10.);
                draw_diamond(ui, pos);
            }
        }
    }
}

fn draw_diamond(ui: &egui::Ui, pos: Vec2) {
    let painter = ui.painter_at(ui.min_rect());

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
