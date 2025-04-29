//! Animation keyframe editor. Very early and only proof-of-concept.

use egui::Stroke;

use ui::COLOR_ACCENT;

use crate::*;

const LINE_OFFSET: f32 = 30.;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    if !shared.ui.anim.playing {
    } else {
        let mut elapsed = shared.ui.anim.elapsed.unwrap().elapsed().as_millis() as f32 / 1e3 as f32;
        let frametime = 1. / shared.selected_animation().fps as f32;

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
                shared.ui.anim.elapsed = Some(std::time::Instant::now());
                shared.ui.anim.played_frame = 0;
            }
        }
    }

    // navigating frames with kb input
    let right = egui_ctx.input(|i| i.key_pressed(egui::Key::ArrowRight));
    let left = egui_ctx.input(|i| i.key_pressed(egui::Key::ArrowLeft));
    if right {
        shared.ui.anim.selected_frame += 1;
        let last_frame = shared.last_keyframe();
        if last_frame != None && shared.ui.anim.selected_frame > last_frame.unwrap().frame {
            shared.ui.anim.selected_frame = 0;
        }
    } else if left {
        shared.ui.anim.selected_frame -= 1;
        let last_frame = shared.last_keyframe();
        if last_frame != None && shared.ui.anim.selected_frame < 0 {
            shared.ui.anim.selected_frame = last_frame.unwrap().frame;
        } else if last_frame == None && shared.ui.anim.selected_frame < 0 {
            shared.ui.anim.selected_frame = 0;
        }
    }

    let response = egui::TopBottomPanel::bottom("Keyframe")
        .min_height(150.)
        .resizable(true)
        .show(egui_ctx, |ui| {
            shared.ui.camera_bar_pos.y = ui.min_rect().top();
            let full_height = ui.available_height();
            ui.horizontal(|ui| {
                ui.set_height(full_height);
                draw_animations_list(ui, shared);

                if shared.ui.anim.selected != usize::MAX {
                    timeline_editor(ui, shared);
                }
            });
        })
        .response;
    if response.hovered() {
        shared.input.on_ui = true;
    }
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
                            if ui::button("New", ui).clicked() {
                                shared.undo_actions.push(Action {
                                    action: ActionEnum::Animation,
                                    action_type: ActionType::Created,
                                    ..Default::default()
                                });
                                new_animation(shared);
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
                            let mut just_made = false;
                            if shared
                                .ui
                                .check_renaming(&rename_id, name, ui, |_| just_made = true)
                            {
                                if just_made {
                                    shared.ui.anim.selected = i;
                                    shared.ui.anim.selected_frame = 0;
                                }
                                continue;
                            }

                            let button =
                                ui::selection_button(&name, i == shared.ui.anim.selected, ui);
                            if button.clicked() {
                                if shared.ui.anim.selected != i {
                                    shared.ui.anim.selected = i;
                                    shared.ui.anim.selected_frame = 0;
                                } else {
                                    shared.ui.rename_id = rename_id;
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

            draw_bones_list(ui, shared, &mut bone_tops);

            // calculate how far apart each keyframe should visually be
            let gap = 400.;
            let hitbox =
                gap / shared.ui.anim.timeline_zoom / shared.selected_animation().fps as f32 / 2.;

            // add 1 second worth of frames after the last keyframe
            let frames: i32;
            if shared.last_keyframe() != None {
                frames = shared.last_keyframe().unwrap().frame + shared.selected_animation().fps;
            } else {
                frames = shared.selected_animation().fps
            }

            let width: f32;
            let generated_width = hitbox * frames as f32 * 2. + LINE_OFFSET;
            if generated_width > ui.min_rect().width() {
                width = generated_width;
            } else {
                width = ui.min_rect().width();
            }

            ui.vertical(|ui| {
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

struct AnimTopInit {
    pub id: i32,
    pub elements: Vec<AnimElement>,
}

pub fn draw_bones_list(ui: &mut egui::Ui, shared: &mut Shared, bone_tops: &mut BoneTops) {
    ui.vertical(|ui| {
        ui.add_space(30.);
        let mut tops_init: Vec<AnimTopInit> = vec![];
        for i in 0..shared.selected_animation().keyframes.len() {
            for b in 0..shared.selected_animation().keyframes[i].bones.len() {
                let bone = &shared.selected_animation().keyframes[i].bones[b];

                // find the bone in the list
                let mut idx: i32 = -1;
                for (i, ti) in tops_init.iter().enumerate() {
                    if ti.id == bone.id {
                        idx = i as i32;
                        break;
                    }
                }

                // add init if the bone can't be found
                if idx == -1 {
                    tops_init.push(AnimTopInit {
                        id: bone.id,
                        elements: vec![],
                    });
                    idx = tops_init.len() as i32 - 1;
                }

                // add all elements
                for af in &bone.fields {
                    let mut add = true;
                    for el in &tops_init[idx as usize].elements {
                        if *el == af.element {
                            add = false;
                            break;
                        }
                    }
                    if add {
                        tops_init[idx as usize].elements.push(af.element.clone());
                    }
                }
            }
        }

        // render the labels!
        for ti in &mut tops_init {
            let bone = shared.find_bone(ti.id).unwrap();
            ui.label(bone.name.to_string());

            // sort the elements by enum index
            ti.elements.sort_by(|a, b| a.cmp(b));

            for e in &ti.elements {
                ui.horizontal(|ui| {
                    ui.add_space(20.);
                    let label = ui.label(e.to_string());
                    bone_tops.tops.push(BoneTop {
                        id: ti.id,
                        element: e.clone(),
                        height: label.rect.top(),
                    })
                });
            }
        }
    });
}

pub fn draw_top_bar(ui: &mut egui::Ui, shared: &mut Shared, width: f32, hitbox: f32) {
    egui::ScrollArea::horizontal()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .scroll_offset(egui::Vec2::new(shared.ui.anim.timeline_offset, 0.))
        .show(ui, |ui| {
            egui::Frame::new().fill(COLOR_ACCENT).show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(20.);

                //draw_connecting_lines(shared, ui);

                for i in 0..shared.selected_animation().keyframes.len() {
                    let kf = &shared.selected_animation().keyframes[i];
                    if shared.ui.anim.lines_x.len() - 1 < kf.frame as usize {
                        break;
                    }

                    let pos = Vec2::new(
                        ui.min_rect().left() + shared.ui.anim.lines_x[kf.frame as usize],
                        ui.min_rect().top() + 10.,
                    );
                    draw_diamond(ui, pos);

                    // create dragging area for diamond
                    let rect = egui::Rect::from_center_size(pos.into(), egui::Vec2::splat(5.));
                    let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

                    if response.drag_started() {
                        shared.ui.anim.selected_frame = kf.frame as i32;
                    }

                    if response.hovered() {
                        shared.cursor_icon = egui::CursorIcon::Grab;
                    }

                    if response.drag_stopped() {
                        shared.cursor_icon = egui::CursorIcon::Grabbing;
                        let cursor = shared.ui.get_cursor(ui);

                        for j in 0..shared.ui.anim.lines_x.len() {
                            let x = shared.ui.anim.lines_x[j];
                            if cursor.x < x + hitbox && cursor.x > x - hitbox {
                                shared.selected_animation_mut().keyframes[i].frame = j as i32;
                                shared.sort_keyframes();
                            }
                        }
                    }
                }
            });
        });
}

pub fn draw_connecting_lines(shared: &Shared, ui: &egui::Ui) {
    let mut prev_frame = -1;
    for kf in &shared.selected_animation().keyframes {
        if prev_frame == -1 {
            prev_frame = kf.frame;
            continue;
        }

        let left = ui.min_rect().left() + shared.ui.anim.lines_x[kf.frame as usize];
        let right = ui.min_rect().left() + shared.ui.anim.lines_x[prev_frame as usize];
        let y = ui.min_rect().top() + 10.;
        let painter = ui.painter_at(ui.min_rect());
        painter.hline(
            egui::Rangef::new(left, right),
            y,
            egui::Stroke::new(2., egui::Color32::WHITE),
        );
    }
}

pub fn draw_timeline_graph(
    ui: &mut egui::Ui,
    shared: &mut Shared,
    width: f32,
    bone_tops: BoneTops,
    hitbox: f32,
) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        let response = egui::ScrollArea::horizontal()
            .id_salt("test")
            .show(ui, |ui| {
                egui::Frame::new().fill(COLOR_ACCENT).show(ui, |ui| {
                    ui.set_width(width);
                    ui.set_height(ui.available_height());

                    // render darkened background after last keyframe
                    if shared.last_keyframe() != None
                        && (shared.last_keyframe().unwrap().frame as usize)
                            < shared.ui.anim.lines_x.len()
                    {
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), ui.available_height()),
                            egui::Sense::empty(),
                        );
                        let painter = ui.painter_at(rect);
                        let rect = egui::vec2(
                            shared.ui.anim.lines_x[shared.last_keyframe().unwrap().frame as usize],
                            0.,
                        );

                        let rect_to_fill = egui::Rect::from_min_size(
                            ui.min_rect().left_top() + rect,
                            ui.min_rect().size(),
                        );

                        painter.rect_filled(
                            rect_to_fill,
                            0.0, // corner rounding radius
                            ui::COLOR_BORDER
                        );
                    }

                    draw_frame_lines(ui, shared, &bone_tops, hitbox);
                });
            });
        shared.ui.anim.timeline_offset = response.state.offset.x;
    });
}

pub fn draw_bottom_bar(ui: &mut egui::Ui, shared: &mut Shared) {
    egui::Frame::new().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_height(20.);
        ui.horizontal(|ui| {
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
                if (button.clicked() || pressed) && shared.selected_animation().keyframes.len() != 0
                {
                    shared.ui.anim.playing = !shared.ui.anim.playing;
                    shared.ui.anim.elapsed = Some(std::time::Instant::now());
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
            if shared.ui.anim.selected_frame < 0 {
                shared.ui.anim.selected_frame = 0;
            }

            ui.label("FPS:").on_hover_text("Frames Per Second");
            shared.selected_animation_mut().fps = shared.ui.singleline_input(
                "fps".to_string(),
                shared.selected_animation().fps as f32,
                ui,
            ) as i32;
        });
    });
}

/// Draw all lines representing frames in the timeline.
fn draw_frame_lines(ui: &mut egui::Ui, shared: &mut Shared, bone_tops: &BoneTops, hitbox: f32) {
    // get cursor pos on the graph (or 0, 0 if can't)
    let cursor: Vec2;
    if ui.ui_contains_pointer() {
        cursor = shared.ui.get_cursor(ui);
    } else {
        cursor = Vec2::default();
    }

    shared.ui.anim.lines_x = vec![];

    let mut x = 0.;
    let mut i = 0;
    while x < ui.min_rect().width() {
        x = i as f32 * hitbox * 2. + LINE_OFFSET;

        shared.ui.anim.lines_x.push(x);

        let mut color = ui::COLOR_FRAMELINE;
        let last_keyframe = shared.selected_animation().keyframes.last();
        if last_keyframe != None && i > last_keyframe.unwrap().frame {
            color = ui::COLOR_FRAMELINE_PASTLAST;
        }

        // check if pointing at a clickable area of this line
        let above_scrollbar = cursor.y < ui.min_rect().height() - 13.;
        if shared.ui.anim.selected_frame == i {
            color = egui::Color32::WHITE;
        } else if cursor.x < x + hitbox && cursor.x > x - hitbox && above_scrollbar {
            shared.cursor_icon = egui::CursorIcon::PointingHand;
            color = ui::COLOR_FRAMELINE_HOVERED;

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

    // draw per-change icons
    for i in 0..shared.selected_animation().keyframes.len() {
        if i > shared.selected_animation().keyframes.len() - 1 {
            break;
        }
        for b in 0..shared.selected_animation().keyframes[i].bones.len() {
            let id = shared.selected_animation().keyframes[i].bones[b].id;
            let x = ui.min_rect().left()
                + shared.ui.anim.lines_x[shared.selected_animation().keyframes[i].frame as usize];

            // go thru anim fields and draw their diamonds
            for af in 0..shared.selected_animation().keyframes[i].bones[b]
                .fields
                .len()
            {
                let size = Vec2::new(17., 17.);

                let element = shared.selected_animation().keyframes[i].bones[b].fields[af]
                    .element
                    .clone();

                // the Y position is based on this diamond's respective label
                let top = bone_tops.find(id, &element).unwrap().height;
                let pos = Vec2::new(x, top + size.y / 2.);
                let offset = size / 2.;

                let rect = egui::Rect::from_min_size((pos - offset).into(), size.into());
                let mut idx = element.clone() as usize;
                if idx > shared.ui.anim.icon_images.len() - 1 {
                    idx = shared.ui.anim.icon_images.len() - 1;
                }
                egui::Image::new(&shared.ui.anim.icon_images[idx]).paint_at(ui, rect);

                let changed = check_change_icon_drag(&element, ui, shared, pos, size, hitbox, i, b);

                // stop rendering for this frame if a change occured, as elements might be out of order
                // and cause indexing issues
                if changed {
                    return;
                }
            }
        }
    }
}

fn draw_diamond(ui: &egui::Ui, pos: Vec2) {
    let painter = ui.painter_at(ui.min_rect());

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
        egui::Color32::TRANSPARENT, // Fill color (transparent)
        egui::Stroke::new(2.0, egui::Color32::WHITE), // Stroke width & color
    ));
}

fn check_change_icon_drag(
    element: &AnimElement,
    ui: &mut egui::Ui,
    shared: &mut Shared,
    pos: Vec2,
    size: Vec2,
    hitbox: f32,
    kf_idx: usize,
    bone_idx: usize,
) -> bool {
    let rect = egui::Rect::from_center_size(pos.into(), (size * 0.5).into());
    let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

    let mut changed = false;

    if response.hovered() {
        shared.cursor_icon = egui::CursorIcon::Grab;
    }

    if !response.drag_stopped() {
        return false;
    }

    shared.cursor_icon = egui::CursorIcon::Grabbing;
    let cursor = shared.ui.get_cursor(ui);

    // delete element if it was dragged out of the graph
    if cursor.y < 0. {
        shared.selected_animation_mut().keyframes[kf_idx].bones[bone_idx].remove_field(element);

        let mut delete_keyframe = true;
        for bone in &shared.selected_animation().keyframes[kf_idx].bones {
            if bone.fields.len() > 0 {
                delete_keyframe = false;
                break;
            }
        }
        if delete_keyframe {
            shared.selected_animation_mut().keyframes.remove(kf_idx);
        }
        return true;
    }

    for j in 0..shared.ui.anim.lines_x.len() {
        if shared.keyframe_at(j as i32) != None
            && shared.keyframe_at(j as i32).unwrap().frame
                == shared.selected_animation().keyframes[kf_idx].frame
        {
            continue;
        }

        let x = shared.ui.anim.lines_x[j];

        let pointing_here = cursor.x < x + hitbox && cursor.x > x - hitbox;
        if !(pointing_here) {
            continue;
        }

        macro_rules! bone {
            () => {
                shared.selected_animation().keyframes[kf_idx].bones[bone_idx]
            };
        }

        let mut dupe_bone = AnimBone {
            id: bone!().id,
            ..Default::default()
        };

        dupe_bone.set_field(element, bone!().find_field(element));

        if shared.keyframe_at(j as i32) == None {
            shared.selected_animation_mut().keyframes.push(Keyframe {
                frame: j as i32,
                bones: vec![dupe_bone],
            });
        } else {
            for i in 0..shared.keyframe_at(j as i32).unwrap().bones.len() {
                if shared.keyframe_at(j as i32).unwrap().bones[i].id != bone!().id {
                    continue;
                }

                shared.keyframe_at_mut(j as i32).unwrap().bones[i]
                    .set_field(element, dupe_bone.find_field(element));

                break;
            }
        }

        // delete previous keyframe if this was the only change prior
        if bone!().fields.len() == 1 {
            shared.selected_animation_mut().keyframes.remove(kf_idx);
        } else {
            changed = true;
            shared.selected_animation_mut().keyframes[kf_idx].bones[bone_idx].remove_field(element);
        }

        shared.sort_keyframes();
    }

    changed
}

pub fn new_animation(shared: &mut Shared) {
    shared.armature.animations.push(Animation {
        name: "".to_string(),
        keyframes: vec![],
        fps: 60,
        ..Default::default()
    });
}

fn _draw_per_change_connecting_lines(shared: &Shared, ui: &egui::Ui, bone_tops: &BoneTops) {
    for kf in &shared.selected_animation().keyframes {
        for bone in &kf.bones {
            for field in &bone.fields {
                let connecting_frame = _get_first_element(kf.frame, &field.element, shared);
                if connecting_frame == -1 {
                    continue;
                }
                let painter = ui.painter_at(ui.min_rect());
                let left = ui.min_rect().left() + shared.ui.anim.lines_x[kf.frame as usize];
                let right =
                    ui.min_rect().left() + shared.ui.anim.lines_x[connecting_frame as usize];
                let y = bone_tops.find(bone.id, &field.element).unwrap().height + 9.;
                painter.hline(
                    egui::Rangef::new(left, right),
                    y,
                    egui::Stroke::new(2., egui::Color32::WHITE),
                );

                let cursor = shared.ui.get_cursor(ui) + ui.min_rect().left_top().into();
                let hitbox = 5.;
                if cursor.x > left
                    && cursor.x < right
                    && cursor.y < y + hitbox
                    && cursor.y > y - hitbox
                {
                    painter.hline(
                        egui::Rangef::new(left, right),
                        y,
                        egui::Stroke::new(2., egui::Color32::RED),
                    );

                    if shared.input.mouse_left != -1 {}
                }
            }
        }
    }
}

fn _get_first_element(start_frame: i32, element: &AnimElement, shared: &Shared) -> i32 {
    for kf in &shared.selected_animation().keyframes {
        if kf.frame <= start_frame {
            continue;
        }
        for bone in &kf.bones {
            for field in &bone.fields {
                if field.element == *element {
                    return kf.frame;
                }
            }
        }
    }
    -1
}
