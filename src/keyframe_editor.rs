//! Animation keyframe editor. Very early and only proof-of-concept.

use egui::Stroke;

use ui::{TextInputOptions, COLOR_ACCENT};

use crate::*;

const LINE_OFFSET: f32 = 30.;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    if shared.ui.anim.playing {
        let mut elapsed = (chrono::Utc::now() - shared.ui.anim.started.unwrap()).as_seconds_f32();
        let frametime = 1. / shared.selected_animation().unwrap().fps as f32;

        // Offset elapsed time with the selected frame.
        // This only applies for the first play cycle, since selected frame
        // is reset on the next one.
        elapsed += shared.ui.anim.played_frame as f32 * frametime;

        shared.ui.anim.selected_frame = (elapsed / frametime) as i32;
        if shared.ui.anim.selected_frame >= shared.last_keyframe().unwrap().frame {
            if shared.recording {
                if shared.ui.anim.loops > 0 {
                    shared.ui.anim.loops -= 1;
                } else {
                    shared.ui.anim.playing = false;
                    shared.recording = false;
                }
            } else {
                shared.ui.anim.started = Some(chrono::Utc::now());
                shared.ui.anim.played_frame = 0;
            }
        }
    }

    // navigating frames with kb input
    if shared.ui.rename_id == "" {
        let right = egui_ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
        let left = egui_ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
        if right {
            shared.ui.anim.selected_frame += 1;
            let last_frame = shared.last_keyframe();
            if last_frame != None && shared.ui.anim.selected_frame > last_frame.unwrap().frame {
                shared.ui.select_anim_frame(0);
            }
        } else if left {
            shared.ui.anim.selected_frame -= 1;
            let last_frame = shared.last_keyframe();
            if last_frame != None && shared.ui.anim.selected_frame < 0 {
                shared.ui.select_anim_frame(last_frame.unwrap().frame);
            } else if last_frame == None && shared.ui.anim.selected_frame < 0 {
                shared.ui.select_anim_frame(0);
            }
        }
    }

    let panel_id = "Keyframe";
    ui::draw_resizable_panel(
        panel_id,
        egui::TopBottomPanel::bottom(panel_id)
            .min_height(150.)
            .resizable(true)
            .show(egui_ctx, |ui| {
                ui::draw_gradient(
                    ui,
                    ui.ctx().screen_rect(),
                    egui::Color32::TRANSPARENT,
                    ui::COLOR_MAIN_DARK,
                );
                shared.ui.camera_bar_pos.y = ui.min_rect().top();
                let full_height = ui.available_height();
                ui.horizontal(|ui| {
                    ui.set_height(full_height);
                    draw_animations_list(ui, shared);

                    if shared.ui.anim.selected != usize::MAX {
                        timeline_editor(ui, shared);
                    }
                });
            }),
        &mut shared.input.on_ui,
        &egui_ctx,
    );
}

fn draw_animations_list(ui: &mut egui::Ui, shared: &mut Shared) {
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
                            let button = ui::button("New", ui);
                            ui::draw_tutorial_rect(
                                TutorialStep::CreateAnim,
                                button.rect,
                                shared,
                                ui,
                            );
                            if button.clicked() {
                                shared.undo_actions.push(Action {
                                    action: ActionEnum::Animation,
                                    action_type: ActionType::Created,
                                    ..Default::default()
                                });
                                new_animation(shared);
                                let idx = shared.armature.animations.len() - 1;
                                shared.ui.original_name = "".to_string();
                                shared.ui.rename_id = "animation ".to_owned() + &idx.to_string();
                                shared.ui.edit_value = Some("".to_string());
                            }
                        });
                    });
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        for i in 0..shared.armature.animations.len() {
                            let name = &mut shared.armature.animations[i].name.clone();

                            // show input field if renaming
                            if shared.ui.rename_id == "animation ".to_string() + &i.to_string() {
                                let (edited, value, _) = ui::text_input(
                                    "animation ".to_string() + &i.to_string(),
                                    shared,
                                    ui,
                                    name.to_string(),
                                    Some(TextInputOptions {
                                        focus: true,
                                        placeholder: "New Animation".to_string(),
                                        default: "New Animation".to_string(),
                                        ..Default::default()
                                    }),
                                );
                                if edited {
                                    shared.armature.animations[i].name = value;
                                    shared.ui.anim.selected = i;
                                }
                                continue;
                            }

                            let button =
                                ui::selection_button(&name, i == shared.ui.anim.selected, ui);
                            if button.clicked() {
                                if shared.ui.anim.selected != i {
                                    shared.ui.anim.selected = i;
                                    shared.ui.select_anim_frame(0);
                                } else {
                                    shared.ui.rename_id = "animation ".to_string() + &i.to_string();
                                    shared.ui.edit_value = Some(name.to_string());
                                }
                            }
                        }
                    });
                })
            });
        });
}

fn timeline_editor(ui: &mut egui::Ui, shared: &mut Shared) {
    egui::Frame::new()
        .outer_margin(egui::Margin {
            left: 0,
            ..Default::default()
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_height(ui.available_height());

            // track the Y of bone change labels for their diamonds
            let mut bone_tops = BoneTops::default();

            if shared.selected_animation().unwrap().keyframes.len() > 0 {
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        top: 27,
                        bottom: 27,
                        left: 0,
                        right: 0,
                    })
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_bones_list(ui, shared, &mut bone_tops);
                        });
                    });
            }

            // calculate how far apart each keyframe should visually be
            let gap = 400.;
            let hitbox = gap
                / shared.ui.anim.timeline_zoom
                / shared.selected_animation().unwrap().fps as f32
                / 2.;

            // add 1 second worth of frames after the last keyframe
            let frames: i32;
            if shared.last_keyframe() != None {
                frames = shared.last_keyframe().unwrap().frame
                    + shared.selected_animation().unwrap().fps;
            } else {
                frames = shared.selected_animation().unwrap().fps
            }

            let width: f32;
            let generated_width = hitbox * frames as f32 * 2. + LINE_OFFSET;
            if generated_width > ui.min_rect().width() {
                width = generated_width;
            } else {
                width = ui.min_rect().width();
            }

            ui.vertical(|ui| {
                // render top bar background
                let rect = egui::Rect::from_min_size(
                    ui.cursor().left_top(),
                    egui::vec2(ui.available_width(), 20.),
                );
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::ZERO, COLOR_ACCENT);

                if shared.ui.anim.lines_x.len() > 0 {
                    draw_top_bar(ui, shared, width, hitbox);
                }

                // The options bar has to be at the bottom, but it needs to be created first
                // so that the remaining height can be taken up by timeline graph.
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    draw_bottom_bar(ui, shared);
                    draw_timeline_graph(ui, shared, width, bone_tops, hitbox);
                });
            });
        });
}

pub fn draw_bones_list(ui: &mut egui::Ui, shared: &mut Shared, bone_tops: &mut BoneTops) {
    egui::ScrollArea::vertical()
        .id_salt("bones_list")
        .vertical_scroll_offset(shared.ui.anim.timeline_offset.y)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
            // sort keyframes by element & bone
            let mut keyframes = shared.selected_animation().unwrap().keyframes.clone();
            keyframes.sort_by(|a, b| (a.element.clone() as i32).cmp(&(b.element.clone() as i32)));
            keyframes.sort_by(|a, b| a.bone_id.cmp(&b.bone_id));

            let mut last_bone_id = -1;

            for i in 0..keyframes.len() {
                let kf = &keyframes[i];

                if last_bone_id != kf.bone_id {
                    ui.label(shared.armature.find_bone(kf.bone_id).unwrap().name.clone());
                    last_bone_id = kf.bone_id
                }

                ui.horizontal(|ui| {
                    ui.add_space(30.);
                    let label = ui.label(kf.element.to_string());
                    bone_tops.tops.push(BoneTop {
                        id: kf.bone_id,
                        element: kf.element.clone(),
                        height: label.rect.top(),
                        vert_id: kf.vert_id,
                    });
                });
            }
        });
}

pub fn draw_top_bar(ui: &mut egui::Ui, shared: &mut Shared, width: f32, hitbox: f32) {
    let mut drew_drag = false;
    egui::ScrollArea::horizontal()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .scroll_offset(egui::Vec2::new(shared.ui.anim.timeline_offset.x, 0.))
        .show(ui, |ui| {
            egui::Frame::new().show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(20.);

                //draw_connecting_lines(shared, ui);

                for i in 0..shared.selected_animation().unwrap().keyframes.len() {
                    let frame = shared.selected_animation().unwrap().keyframes[i].frame;

                    // don't draw diamond if it's beyond the recorded lines
                    if shared.ui.anim.lines_x.len() - 1 < frame as usize {
                        break;
                    }

                    let pos = Vec2::new(
                        ui.min_rect().left() + shared.ui.anim.lines_x[frame as usize] + 3.,
                        ui.min_rect().top() + 10.,
                    );

                    // create dragging area for diamond
                    let rect = egui::Rect::from_center_size(pos.into(), egui::Vec2::splat(5.));
                    let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

                    if response.drag_started() {
                        shared.ui.select_anim_frame(frame);
                    }

                    if response.hovered() {
                        shared.cursor_icon = egui::CursorIcon::Grab;
                    }

                    if response.dragged() {
                        shared.ui.anim.dragged_keyframe = frame;
                    }

                    if shared.ui.anim.dragged_keyframe != frame {
                        draw_diamond(ui.painter(), pos, egui::Color32::WHITE);
                    } else if !drew_drag {
                        draw_diamond(
                            ui.painter(),
                            pos,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        );
                        drew_drag = true;
                    }

                    let just_clicked = shared.input.mouse_left_prev < 10;
                    if !response.drag_stopped() || just_clicked {
                        continue;
                    }

                    shared.ui.anim.dragged_keyframe = -1;

                    shared.undo_actions.push(shared::Action {
                        action: ActionEnum::Animation,
                        action_type: ActionType::Edited,
                        id: shared.ui.anim.selected as i32,
                        animation: shared.selected_animation().unwrap().clone(),
                        ..Default::default()
                    });
                    shared.cursor_icon = egui::CursorIcon::Grabbing;

                    // remove keyframe if dragged out
                    let cursor = shared.ui.get_cursor(ui);
                    if cursor.y < 0. {
                        let frame = shared.selected_animation_mut().unwrap().keyframes[i].frame;
                        let _ = shared
                            .selected_animation_mut()
                            .unwrap()
                            .keyframes
                            .iter()
                            .filter(|kf| kf.frame != frame);

                        // break loop to prevent OOB errors
                        break;
                    }

                    // move all keyframes under this one over
                    for j in 0..shared.ui.anim.lines_x.len() {
                        let x = shared.ui.anim.lines_x[j];
                        if cursor.x < x + hitbox && cursor.x > x - hitbox {
                            for kf in &mut shared.selected_animation_mut().unwrap().keyframes {
                                if kf.frame == frame as i32 {
                                    kf.frame = j as i32;
                                }
                            }
                            shared.sort_keyframes();
                            break;
                        }
                    }
                }
            });
        });
}

pub fn draw_timeline_graph(
    ui: &mut egui::Ui,
    shared: &mut Shared,
    width: f32,
    bone_tops: BoneTops,
    hitbox: f32,
) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        egui::Frame::new()
            .fill(COLOR_ACCENT)
            .inner_margin(3)
            .show(ui, |ui| {
                let response = egui::ScrollArea::both().id_salt("test").show(ui, |ui| {
                    ui.set_width(width);
                    ui.set_height(ui.available_height());

                    let mut cursor = shared.ui.get_cursor(ui);
                    // keep cursor on the frame
                    cursor.y -= shared.ui.anim.timeline_offset.y;

                    // render darkened background after last keyframe
                    if shared.last_keyframe() != None
                        && (shared.last_keyframe().unwrap().frame as usize)
                            < shared.ui.anim.lines_x.len()
                    {
                        let left_top_rect = egui::vec2(
                            shared.ui.anim.lines_x[shared.last_keyframe().unwrap().frame as usize],
                            -3.,
                        );
                        let right_bottom_rect = egui::vec2(0., 999.);

                        let rect_to_fill = egui::Rect::from_min_size(
                            ui.min_rect().left_top() + left_top_rect,
                            ui.min_rect().size() + right_bottom_rect,
                        );

                        ui.painter().rect_filled(rect_to_fill, 0., ui::COLOR_BORDER);
                    }

                    draw_frame_lines(ui, shared, &bone_tops, hitbox, cursor);
                });
                shared.ui.anim.timeline_offset = response.state.offset.into();
                shared.ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;
            });
    });
}

pub fn draw_bottom_bar(ui: &mut egui::Ui, shared: &mut Shared) {
    egui::Frame::new().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_height(20.);
        ui.horizontal(|ui| {
            ui.painter_at(ui.min_rect()).rect_filled(
                ui.min_rect(),
                egui::CornerRadius::ZERO,
                ui::COLOR_MAIN,
            );

            let str = if shared.ui.anim.playing {
                "Pause"
            } else {
                "Play"
            };

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                let button = ui
                    .add_sized(
                        [50., 20.],
                        egui::Button::new(str).fill(COLOR_ACCENT).corner_radius(0.),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                let pressed = ui.input(|i| i.key_pressed(egui::Key::Space));
                if (button.clicked() || pressed)
                    && shared.selected_animation().unwrap().keyframes.len() != 0
                {
                    shared.ui.anim.playing = !shared.ui.anim.playing;
                    shared.ui.anim.started = Some(chrono::Utc::now());
                    shared.ui.anim.played_frame = shared.ui.anim.selected_frame;
                }
            });

            if ui::button("+", ui).clicked() {
                shared.ui.anim.timeline_zoom -= 0.1;
            }
            if ui::button("-", ui).clicked() {
                shared.ui.anim.timeline_zoom += 0.1;
            }

            ui.add_space(20.);

            ui.label("Frame:");
            ui.add(egui::DragValue::new(&mut shared.ui.anim.selected_frame).speed(0.1));

            let fps = shared.selected_animation().unwrap().fps;

            ui.label("FPS:").on_hover_text("Frames Per Second");
            let (edited, value, _) = ui::float_input("fps".to_string(), shared, ui, fps as f32, 1.);
            if edited {
                shared.selected_animation_mut().unwrap().fps = value as i32;
            }
            shared.ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;

            if ui::button("Copy", ui).clicked() {
                macro_rules! keyframes {
                    () => {
                        shared.selected_animation().unwrap().keyframes
                    };
                }
                for kf in 0..keyframes!().len() {
                    if keyframes!()[kf].frame == shared.ui.anim.selected_frame {
                        shared.copy_buffer.keyframes.push(keyframes!()[kf].clone())
                    }
                }
            }

            if ui::button("Paste", ui).clicked() {
                let frame = shared.ui.anim.selected_frame;

                // remove current keyframes on this frame
                let _ = shared
                    .selected_animation()
                    .unwrap()
                    .keyframes
                    .iter()
                    .filter(|kf| kf.frame != shared.ui.anim.selected_frame);

                for kf in 0..shared.copy_buffer.keyframes.len() {
                    let keyframe = shared.copy_buffer.keyframes[kf].clone();
                    shared
                        .selected_animation_mut()
                        .unwrap()
                        .keyframes
                        .push(Keyframe { frame, ..keyframe })
                }
            }
        });
    });
}

/// Draw all lines representing frames in the timeline.
fn draw_frame_lines(
    ui: &mut egui::Ui,
    shared: &mut Shared,
    bone_tops: &BoneTops,
    hitbox: f32,
    cursor: Vec2,
) {
    shared.ui.anim.lines_x = vec![];

    let mut x = 0.;
    let mut i = 0;
    while x < ui.min_rect().width() {
        x = i as f32 * hitbox * 2. + LINE_OFFSET;

        shared.ui.anim.lines_x.push(x);

        let mut color = ui::COLOR_FRAMELINE;
        if shared.last_keyframe() != None && i > shared.last_keyframe().unwrap().frame {
            color = ui::COLOR_FRAMELINE_PASTLAST;
        }

        let above_scrollbar = cursor.y < ui.min_rect().height() - 13.;
        let in_ui = cursor.y > 0.;

        if shared.ui.anim.selected_frame == i {
            color = egui::Color32::WHITE;
        } else if in_ui && cursor.x < x + hitbox && cursor.x > x - hitbox && above_scrollbar {
            shared.cursor_icon = egui::CursorIcon::PointingHand;
            color = ui::COLOR_FRAMELINE_HOVERED;

            // select this frame if clicked
            if shared.input.mouse_left == 0 {
                shared.ui.anim.selected_frame = i;
            }
        }

        // draw the line!
        ui.painter().vline(
            ui.min_rect().left() + x,
            egui::Rangef { min: 0., max: 999. },
            Stroke { width: 2., color },
        );

        i += 1;
    }

    // used to determine lowest rendered icon, to add extra space at the bottom
    let mut height = 0.;

    // draw per-change icons
    for i in 0..shared.selected_animation().unwrap().keyframes.len() {
        let kf = &shared.selected_animation().unwrap().keyframes[i];
        let size = Vec2::new(17., 17.);

        // the Y position is based on this diamond's respective label
        let top = bone_tops
            .find(kf.bone_id, &kf.element.clone(), kf.vert_id)
            .unwrap()
            .height;
        let x = shared.ui.anim.lines_x[kf.frame as usize] + ui.min_rect().left();
        let pos = Vec2::new(x, top + size.y / 2.);
        let offset = size / 2.;

        if top > height {
            height = top;
        }

        let rect = egui::Rect::from_min_size((pos - offset).into(), size.into());
        let mut idx = kf.element.clone().clone() as usize;
        if idx > shared.ui.anim.icon_images.len() - 1 {
            idx = shared.ui.anim.icon_images.len() - 1;
        }
        egui::Image::new(&shared.ui.anim.icon_images[shared::ANIM_ICON_ID[idx]]).paint_at(ui, rect);

        let rect = egui::Rect::from_center_size(pos.into(), (size * 0.5).into());
        let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

        if response.hovered() {
            shared.cursor_icon = egui::CursorIcon::Grab;
        }

        if !response.drag_stopped() {
            continue;
        }

        if cursor.y < 0. {
            shared.selected_animation_mut().unwrap().keyframes.remove(i);

            // break the loop to prevent OOB errors
            break;
        }

        for j in 0..shared.ui.anim.lines_x.len() {
            let x = shared.ui.anim.lines_x[j];
            if cursor.x < x + hitbox && cursor.x > x - hitbox {
                shared.selected_animation_mut().unwrap().keyframes[i].frame = j as i32;
                shared.sort_keyframes();
                break;
            }
        }
    }

    // create extra space at the bottom
    let rect = egui::Rect::from_min_size(egui::pos2(0., height), egui::Vec2::new(1., 40.));
    ui.allocate_rect(rect, egui::Sense::empty());
}

pub fn draw_diamond(painter: &egui::Painter, pos: Vec2, color: egui::Color32) {
    let size = 5.0;

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
        egui::Color32::TRANSPARENT,    // Fill color (transparent)
        egui::Stroke::new(2.0, color), // Stroke width & color
    ));
}

pub fn new_animation(shared: &mut Shared) {
    shared.armature.animations.push(Animation {
        name: "".to_string(),
        keyframes: vec![],
        fps: 60,
        ..Default::default()
    });
}

// pub fn draw_connecting_lines(shared: &Shared, ui: &egui::Ui) {
//     let mut prev_frame = -1;
//     for kf in &shared.selected_animation().unwrap().keyframes {
//         if prev_frame == -1 {
//             prev_frame = kf.frame;
//             continue;
//         }

//         let left = ui.min_rect().left() + shared.ui.anim.lines_x[kf.frame as usize];
//         let right = ui.min_rect().left() + shared.ui.anim.lines_x[prev_frame as usize];
//         let y = ui.min_rect().top() + 10.;
//         ui.painter_at(ui.min_rect()).hline(
//             egui::Rangef::new(left, right),
//             y,
//             egui::Stroke::new(2., egui::Color32::WHITE),
//         );
//     }
// }

// fn _draw_per_change_connecting_lines(shared: &Shared, ui: &egui::Ui, bone_tops: &BoneTops) {
//     for kf in &shared.selected_animation().unwrap().keyframes {
//         for bone in &kf.bones {
//             for field in &bone.fields {
//                 let connecting_frame = _get_first_element(kf.frame, &field.element, shared);
//                 if connecting_frame == -1 {
//                     continue;
//                 }
//                 let left = ui.min_rect().left() + shared.ui.anim.lines_x[kf.frame as usize];
//                 let right =
//                     ui.min_rect().left() + shared.ui.anim.lines_x[connecting_frame as usize];
//                 let y = bone_tops.find(bone.id, &field.element).unwrap().height + 9.;
//                 ui.painter_at(ui.min_rect()).hline(
//                     egui::Rangef::new(left, right),
//                     y,
//                     egui::Stroke::new(2., egui::Color32::WHITE),
//                 );

//                 let cursor = shared.ui.get_cursor(ui) + ui.min_rect().left_top().into();
//                 let hitbox = 5.;
//                 if cursor.x > left
//                     && cursor.x < right
//                     && cursor.y < y + hitbox
//                     && cursor.y > y - hitbox
//                 {
//                     ui.painter_at(ui.min_rect()).hline(
//                         egui::Rangef::new(left, right),
//                         y,
//                         egui::Stroke::new(2., egui::Color32::RED),
//                     );

//                     if shared.input.mouse_left != -1 {}
//                 }
//             }
//         }
//     }
// }

// fn _get_first_element(start_frame: i32, element: &AnimElement, shared: &Shared) -> i32 {
//     for kf in &shared.selected_animation().unwrap().keyframes {
//         if kf.frame <= start_frame {
//             continue;
//         }
//         for bone in &kf.bones {
//             for field in &bone.fields {
//                 if field.element == *element {
//                     return kf.frame;
//                 }
//             }
//         }
//     }
//     -1
// }
