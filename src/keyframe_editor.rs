//! Animation keyframe editor. Very early and only proof-of-concept.

use egui::Stroke;

use ui::{EguiUi, TextInputOptions};

use crate::*;

const LINE_OFFSET: f32 = 30.;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    if !shared.input.left_down {
        shared.ui.anim.dragged_keyframe.frame = -1;
    }

    // navigating frames with kb input
    if shared.ui.rename_id == "" {
        let right = egui_ctx.input_mut(|i| i.consume_shortcut(&shared.config.keys.next_anim_frame));
        let left = egui_ctx.input_mut(|i| i.consume_shortcut(&shared.config.keys.prev_anim_frame));
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
            }
        }
    }

    let panel_id = "Keyframe";
    let panel = egui::TopBottomPanel::bottom(panel_id)
        .min_height(150.)
        .resizable(true);
    ui::draw_resizable_panel(
        panel_id,
        panel.show(egui_ctx, |ui| {
            ui.gradient(
                ui.ctx().content_rect(),
                egui::Color32::TRANSPARENT,
                shared.config.colors.gradient.into(),
            );

            let full_height = ui.available_height();
            ui.horizontal(|ui| {
                ui.set_height(full_height);

                // animations list
                let resize = egui::Resize::default()
                    .min_height(full_height) // make height unadjustable
                    .max_height(full_height) //
                    .default_width(150.)
                    .max_width(200.)
                    .with_stroke(false);
                resize.show(ui, |ui| {
                    egui::Frame::new().show(ui, |ui| {
                        ui.vertical(|ui| {
                            draw_animations_list(ui, shared);
                        })
                    });
                });

                if shared.ui.anim.selected != usize::MAX {
                    timeline_editor(ui, shared);
                }
            });
            shared.ui.keyframe_panel_rect = Some(ui.min_rect());
        }),
        &mut shared.input.on_ui,
        &egui_ctx,
    );
}

fn draw_animations_list(ui: &mut egui::Ui, shared: &mut Shared) {
    ui.horizontal(|ui| {
        let str_anim = shared.loc("keyframe_editor.heading");
        ui.heading(str_anim);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
            let str_new = &&shared.loc("new");
            let button = ui.skf_button(str_new);

            if !button.clicked() {
                return;
            }

            shared.new_undo_anims();

            shared.armature.new_animation();
            let idx = shared.armature.animations.len() - 1;
            shared.ui.original_name = "".to_string();
            shared.ui.rename_id = "anim_".to_owned() + &idx.to_string();
            shared.ui.edit_value = Some("".to_string());
        });
    });
    egui::ScrollArea::vertical().show(ui, |ui| {
        let frame = egui::Frame::new().fill(shared.config.colors.dark_accent.into());
        frame.show(ui, |ui| {
            let width = ui.available_width();
            let mut hovered = false;
            for i in 0..shared.armature.animations.len() {
                let name = &mut shared.armature.animations[i].name.clone();
                let context_id = "anim_".to_owned() + &i.to_string();

                // show input field if renaming
                if shared.ui.rename_id == context_id {
                    let str_new_anim = &shared.loc("keyframe_editor.new_animation");
                    let options = Some(TextInputOptions {
                        focus: true,
                        placeholder: str_new_anim.to_string(),
                        default: str_new_anim.to_string(),
                        ..Default::default()
                    });
                    let (edited, value, _) =
                        ui.text_input(context_id, shared, name.to_string(), options);
                    if edited {
                        shared.new_undo_anims();
                        shared.armature.animations[i].name = value;
                        shared.ui.anim.selected = i;
                        shared.ui.anim.selected_frame = 0;
                    }
                    continue;
                }

                ui.horizontal(|ui| {
                    let button_padding = if shared.armature.animations[i].keyframes.len() > 0 {
                        25.
                    } else {
                        0.
                    };
                    let mut col = shared.config.colors.dark_accent;
                    if i == shared.ui.hovering_anim as usize {
                        col += crate::Color::new(20, 20, 20, 0);
                    }
                    if i == shared.ui.anim.selected {
                        col += crate::Color::new(20, 20, 20, 0);
                    }
                    let cursor_icon = if shared.ui.anim.selected != i {
                        egui::CursorIcon::PointingHand
                    } else {
                        egui::CursorIcon::Default
                    };
                    //let button = ui::selection_button(&name, i == shared.ui.anim.selected, ui);
                    let button = egui::Frame::new()
                        .fill(col.into())
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_width(width - button_padding);
                                ui.set_height(21.);
                                ui.add_space(5.);
                                let col = shared.config.colors.text;
                                ui.label(egui::RichText::new(name.clone()).color(col));
                            });
                        })
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(cursor_icon);
                    if button.contains_pointer() {
                        shared.ui.hovering_anim = i as i32;
                        hovered = true;
                    }
                    if button.clicked() {
                        if shared.ui.anim.selected != i {
                            shared.ui.anim.selected = i;
                            shared.ui.select_anim_frame(0);
                        } else {
                            shared.ui.rename_id = context_id.clone();
                            shared.ui.edit_value = Some(name.to_string());
                        }
                    }

                    if shared.armature.animations[i].keyframes.len() > 0 {
                        let anim = &mut shared.armature.animations[i];
                        let align = egui::Layout::right_to_left(egui::Align::Center);
                        ui.with_layout(align, |ui| {
                            let icon = if anim.elapsed == None { "⏵" } else { "⏹" };
                            if ui.skf_button(icon).clicked() {
                                anim.elapsed = if anim.elapsed == None {
                                    Some(Instant::now())
                                } else {
                                    None
                                };
                            }
                        });
                    }

                    context_menu!(button, shared, context_id, |ui: &mut egui::Ui| {
                        ui.context_rename(shared, context_id);
                        ui.context_delete(shared, "delete_anim", PolarId::DeleteAnim);
                        let duplicate_str = shared.loc("keyframe_editor.duplicate");
                        if ui.context_button(duplicate_str, shared).clicked() {
                            shared.new_undo_anims();
                            let anims = &mut shared.armature.animations;
                            anims.push(anims[i].clone());
                            shared.ui.context_menu.close();
                        }
                    });
                });
            }

            if !hovered {
                shared.ui.hovering_anim = -1;
            }
        });
    });
}

fn timeline_editor(ui: &mut egui::Ui, shared: &mut Shared) {
    let frame = egui::Frame::new().outer_margin(egui::Margin {
        left: 0,
        ..Default::default()
    });
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_height(ui.available_height());

        // track the Y of bone change labels for their diamonds
        let mut bone_tops = BoneTops::default();

        if shared.selected_animation().unwrap().keyframes.len() > 0 {
            let frame = egui::Frame::new().inner_margin(egui::Margin {
                top: 27,
                bottom: 27,
                left: 0,
                right: 0,
            });
            frame.show(ui, |ui| {
                ui.vertical(|ui| {
                    draw_bones_list(ui, shared, &mut bone_tops);
                });
            });
        }

        // calculate how far apart each keyframe should visually be
        let gap = 400.;
        let fps = shared.selected_animation().unwrap().fps as f32;
        let hitbox = gap / shared.ui.anim.timeline_zoom / fps / 2.;

        // add 1 second worth of frames after the last keyframe
        let frames: i32;
        let extra = shared.selected_animation().unwrap().fps * 5;
        if shared.last_keyframe() != None {
            frames = shared.last_keyframe().unwrap().frame + extra;
        } else {
            frames = extra
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
            let light_accent = shared.config.colors.light_accent;
            let zero_corner = egui::CornerRadius::ZERO;
            ui.painter().rect_filled(rect, zero_corner, light_accent);

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
    let scroll_area = egui::ScrollArea::vertical()
        .id_salt("bones_list")
        .vertical_scroll_offset(shared.ui.anim.timeline_offset.y)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
    scroll_area.show(ui, |ui| {
        // sort keyframes by element & bone
        let mut keyframes = shared.selected_animation().unwrap().keyframes.clone();
        keyframes.sort_by(|a, b| (a.element.clone() as i32).cmp(&(b.element.clone() as i32)));
        keyframes.sort_by(|a, b| a.bone_id.cmp(&b.bone_id));

        let mut last_bone_id = -1;

        // keep track of elements, to prevent showing multiple of the same
        let mut added_elements: Vec<AnimElement> = vec![];

        for i in 0..keyframes.len() {
            let kf = &keyframes[i];

            if last_bone_id != kf.bone_id {
                let bones = &shared.armature.bones;
                let bone = bones.iter().find(|b| b.id == kf.bone_id).unwrap();
                let label = ui
                    .label(bone.name.clone())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .interact(egui::Sense::click());
                if label.clicked() {
                    let kf_id = kf.bone_id;
                    let sel = shared.armature.bones.iter().position(|b| b.id == kf_id);
                    shared.ui.selected_bone_idx = sel.unwrap();

                    let parents = shared.armature.get_all_parents(kf.bone_id);
                    for parent in &parents {
                        shared.armature.find_bone_mut(parent.id).unwrap().folded = false;
                    }
                }
                last_bone_id = kf.bone_id;

                // reset element tracker, since this is a new bone
                added_elements = vec![];
            }

            if added_elements.contains(&kf.element) {
                continue;
            }

            added_elements.push(kf.element.clone());
            ui.horizontal(|ui| {
                ui.add_space(30.);
                let label = ui.label(
                    &shared
                        .loc(&("keyframe_editor.elements.".to_owned() + &kf.element.to_string())),
                );
                bone_tops.tops.push(BoneTop {
                    id: kf.bone_id,
                    element: kf.element.clone(),
                    height: label.rect.top(),
                });
            });
        }
    });
}

pub fn draw_top_bar(ui: &mut egui::Ui, shared: &mut Shared, width: f32, hitbox: f32) {
    let mut drew_drag = false;
    let scroll_area = egui::ScrollArea::horizontal()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .scroll_offset(egui::Vec2::new(shared.ui.anim.timeline_offset.x, 0.));

    scroll_area.show(ui, |ui| {
        egui::Frame::new().show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(20.);

            let mut second = 0;
            for (i, x) in shared.ui.anim.lines_x.iter().enumerate() {
                if i as i32 % shared.selected_animation().unwrap().fps != 0 {
                    continue;
                }
                second += 1;
                let pos = Vec2::new(ui.min_rect().left() + x, ui.min_rect().top() + 10.);
                let center = egui::Align2::CENTER_CENTER;
                let fontid = egui::FontId::default();
                let col = shared.config.colors.text;
                let painter = ui.painter_at(ui.min_rect());
                let str = second.to_string() + "s";
                painter.text(pos.into(), center, str, fontid, col.into());
            }

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

                let cursor = shared.ui.get_cursor(ui);

                if response.dragged() {
                    shared.ui.anim.dragged_keyframe = Keyframe {
                        frame,
                        bone_id: -1,
                        ..Default::default()
                    };
                    let color = if cursor.y < 0. {
                        egui::Color32::RED
                    } else {
                        egui::Color32::WHITE
                    };
                    let pos = cursor + ui.min_rect().left_top().into();
                    draw_diamond(&ui.ctx().debug_painter(), pos, color);
                }

                let kf = &shared.ui.anim.dragged_keyframe;
                if kf.frame != frame || kf.bone_id != -1 {
                    draw_diamond(ui.painter(), pos, egui::Color32::WHITE);
                } else if !drew_drag {
                    let white = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30);
                    draw_diamond(ui.painter(), pos, white);
                    drew_drag = true;
                }

                if !response.drag_stopped() || shared.input.left_clicked {
                    continue;
                }

                shared.new_undo_sel_anim();

                shared.cursor_icon = egui::CursorIcon::Grabbing;

                // remove keyframe if dragged out
                if cursor.y < 0. {
                    let anim = shared.selected_animation_mut().unwrap();
                    let frame = anim.keyframes[i].frame;
                    anim.keyframes.retain(|kf| kf.frame != frame);
                    // break loop to prevent OOB errors
                    break;
                }

                // move all keyframes under this one over
                for j in 0..shared.ui.anim.lines_x.len() {
                    let x = shared.ui.anim.lines_x[j];
                    if !(cursor.x < x + hitbox && cursor.x > x - hitbox) {
                        continue;
                    }
                    let selected_anim = &mut shared.selected_animation_mut().unwrap();
                    selected_anim.keyframes.retain(|kf| kf.frame != j as i32);
                    for kf in &mut shared.selected_animation_mut().unwrap().keyframes {
                        if kf.frame == frame as i32 {
                            kf.frame = j as i32;
                        }
                    }
                    shared.selected_animation_mut().unwrap().sort_keyframes();
                    return;
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
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    ui.with_layout(layout, |ui| {
        let frame = egui::Frame::new()
            .fill(shared.config.colors.light_accent.into())
            .inner_margin(3);
        frame.show(ui, |ui| {
            let response = egui::ScrollArea::both().id_salt("test").show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(ui.available_height());

                let mut cursor = shared.ui.get_cursor(ui);
                // keep cursor on the frame
                cursor.y -= shared.ui.anim.timeline_offset.y;

                // render darkened background after last keyframe
                let lkf = shared.last_keyframe();
                if lkf != None && (lkf.unwrap().frame as usize) < shared.ui.anim.lines_x.len() {
                    let left_top_rect = egui::vec2(
                        shared.ui.anim.lines_x[shared.last_keyframe().unwrap().frame as usize],
                        -3.,
                    );
                    let right_bottom_rect = egui::vec2(0., 999.);

                    let rect_to_fill = egui::Rect::from_min_size(
                        ui.min_rect().left_top() + left_top_rect,
                        ui.min_rect().size() + right_bottom_rect,
                    );

                    ui.painter()
                        .rect_filled(rect_to_fill, 0., shared.config.colors.dark_accent);
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
                shared.config.colors.main,
            );

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                let play_str = if shared.selected_animation().unwrap().elapsed != None {
                    &shared.loc("keyframe_editor.pause")
                } else {
                    &shared.loc("keyframe_editor.play")
                };

                let play_text = egui::RichText::new(play_str).color(shared.config.colors.text);
                let button = egui::Button::new(play_text)
                    .fill(shared.config.colors.light_accent)
                    .corner_radius(0.);

                let button = ui
                    .add_sized([50., 20.], button)
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                let mut pressed = ui.input(|i| i.key_pressed(egui::Key::Space));
                if button.clicked() {
                    pressed = true;
                }
                if !pressed || shared.selected_animation().unwrap().keyframes.len() == 0 {
                    return;
                }

                let anim = shared.selected_animation_mut().unwrap();
                anim.elapsed = if anim.elapsed == None {
                    Some(Instant::now())
                } else {
                    None
                };
                shared.ui.anim.played_frame = shared.ui.anim.selected_frame;
            });

            if ui.skf_button("+").clicked() {
                shared.ui.anim.timeline_zoom -= 0.1;
            }
            if ui.skf_button("-").clicked() {
                shared.ui.anim.timeline_zoom += 0.1;
            }

            ui.add_space(20.);

            ui.label(&shared.loc("keyframe_editor.frame"));
            ui.add(egui::DragValue::new(&mut shared.ui.anim.selected_frame).speed(0.1));

            let fps = shared.selected_animation().unwrap().fps;

            ui.label(&shared.loc("keyframe_editor.fps"))
                .on_hover_text(&shared.loc("keyframe_editor.frames_per_second"));
            let (edited, value, _) =
                ui.float_input("fps".to_string(), shared, fps as f32, 1., None);
            if edited {
                let anim_mut = shared.selected_animation_mut().unwrap();

                let mut old_unique_keyframes: Vec<i32> =
                    anim_mut.keyframes.iter().map(|kf| kf.frame).collect();
                old_unique_keyframes.dedup();

                let mut anim_clone = anim_mut.clone();

                // adjust keyframes to maintain spacing
                let div = anim_mut.fps as f32 / value;
                for kf in &mut anim_clone.keyframes {
                    kf.frame = ((kf.frame as f32) / div) as i32
                }

                let mut unique_keyframes: Vec<i32> =
                    anim_clone.keyframes.iter().map(|kf| kf.frame).collect();
                unique_keyframes.dedup();

                if unique_keyframes.len() == old_unique_keyframes.len() {
                    anim_mut.fps = value as i32;
                    anim_mut.keyframes = anim_clone.keyframes;
                } else {
                    let str_invalid = shared.loc("keyframe_editor.invalid_fps").to_string();
                    shared.ui.open_modal(str_invalid, false);
                }
            }
            shared.ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;

            if ui.skf_button(&shared.loc("keyframe_editor.copy")).clicked() {
                shared.copy_buffer = CopyBuffer::default();
                for kf in 0..shared.selected_animation().unwrap().keyframes.len() {
                    let frame = shared.ui.anim.selected_frame;
                    if shared.selected_animation().unwrap().keyframes[kf].frame == frame {
                        let keyframe = shared.selected_animation().unwrap().keyframes[kf].clone();
                        shared.copy_buffer.keyframes.push(keyframe);
                    }
                }
            }

            let paste_str = &shared.loc("keyframe_editor.paste");
            if ui.skf_button(paste_str).clicked() {
                shared.new_undo_sel_anim();

                let frame = shared.ui.anim.selected_frame;
                let buffer_frames = shared.copy_buffer.keyframes.clone();
                let anim = &mut shared.selected_animation_mut().unwrap();

                anim.keyframes.retain(|kf| kf.frame != frame);

                for kf in 0..buffer_frames.len() {
                    let keyframe = buffer_frames[kf].clone();
                    anim.keyframes.push(Keyframe { frame, ..keyframe })
                }

                shared.selected_animation_mut().unwrap().sort_keyframes();
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

        let mut color: egui::Color32 = shared.config.colors.frameline.into();
        if shared.last_keyframe() != None && i > shared.last_keyframe().unwrap().frame {
            color = shared.config.colors.dark_accent.into();
        }
        let anim = &mut shared.armature.animations[shared.ui.anim.selected];
        if i == anim.get_frame() && anim.elapsed != None {
            color = color + egui::Color32::from_rgb(60, 60, 60);
        }

        let above_bar = cursor.y < ui.min_rect().height() - 13.;
        let in_ui = cursor.y > 0.;
        let in_modal = shared.ui.modal || shared.ui.settings_modal;
        let cur = cursor;

        if shared.ui.anim.selected_frame == i {
            color = egui::Color32::WHITE;
        } else if !in_modal && in_ui && cur.x < x + hitbox && cur.x > x - hitbox && above_bar {
            shared.cursor_icon = egui::CursorIcon::PointingHand;
            color = egui::Color32::WHITE;

            // select this frame if clicked
            if shared.input.left_clicked {
                shared.ui.anim.selected_frame = i;
                shared.input.on_ui = true;
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
        let kf = shared.selected_animation().unwrap().keyframes[i].clone();
        let size = Vec2::new(17., 17.);

        // the Y position is based on this diamond's respective label
        let top: f32;

        let el = kf.element.clone();
        let b_id = kf.bone_id;
        let tops = &bone_tops.tops;
        if let Some(b_top) = tops.iter().find(|bt| bt.id == b_id && bt.element == el) {
            top = b_top.height;
        } else {
            return;
        }
        let x = shared.ui.anim.lines_x[kf.frame as usize] + ui.min_rect().left();
        let pos = Vec2::new(x, top + size.y / 2.);
        let offset = size / 2.;

        if top > height {
            height = top;
        }

        let rect = egui::Rect::from_min_size((pos - offset).into(), size.into());
        let mut idx = kf.element.clone().clone() as usize;
        if idx > shared::ANIM_ICON_ID.len() - 1 {
            idx = shared::ANIM_ICON_ID.len() - 1;
        }

        let dkf = &shared.ui.anim.dragged_keyframe;
        let mut color = egui::Color32::WHITE;
        if *dkf == kf {
            color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30);
        }

        egui::Image::new(&shared.ui.anim.icon_images[shared::ANIM_ICON_ID[idx]])
            .tint(color)
            .paint_at(ui, rect);

        let rect = egui::Rect::from_center_size(pos.into(), (size * 0.5).into());
        let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

        if response.hovered() {
            shared.cursor_icon = egui::CursorIcon::Grab;
        }

        if response.dragged() {
            shared.ui.anim.dragged_keyframe = kf.clone();
            if let Some(cursor) = ui.ctx().pointer_latest_pos() {
                let pos = egui::Pos2::new(cursor.x - offset.x, cursor.y - offset.y);
                let drag_rect = egui::Rect::from_min_size(pos, size.into());
                egui::Image::new(&shared.ui.anim.icon_images[shared::ANIM_ICON_ID[idx]])
                    .paint_at(ui, drag_rect);
            }
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
            if !(cursor.x < x + hitbox && cursor.x > x - hitbox) {
                continue;
            }

            let curr_kf = &shared.selected_animation().unwrap().keyframes[i];

            // ignore if icon is dragged to the same line
            if curr_kf.frame == j as i32 {
                return;
            }

            // remove keyframe that is the same as this
            let keyframes = shared.selected_animation().unwrap().keyframes.clone();
            let k = keyframes.iter().position(|kf| {
                kf.bone_id == curr_kf.bone_id
                    && kf.element == curr_kf.element
                    && kf.frame == j as i32
            });
            let mut curr = i;
            if k != None {
                let keyframes = &mut shared.selected_animation_mut().unwrap().keyframes;
                keyframes.remove(k.unwrap());
                curr -= 1;
            }

            shared.selected_animation_mut().unwrap().keyframes[curr].frame = j as i32;

            shared.selected_animation_mut().unwrap().sort_keyframes();
            return;
        }
    }

    // create extra space at the bottom
    let rect = egui::Rect::from_min_size(egui::pos2(0., height), egui::Vec2::new(1., 40.));
    ui.allocate_rect(rect, egui::Sense::empty());
}

pub fn draw_diamond(painter: &egui::Painter, pos: Vec2, color: egui::Color32) {
    let size = 5.0;

    let points = vec![
        egui::Pos2::new(pos.x, pos.y - size), // Top
        egui::Pos2::new(pos.x + size, pos.y), // Right
        egui::Pos2::new(pos.x, pos.y + size), // Bottom
        egui::Pos2::new(pos.x - size, pos.y), // Left
    ];

    painter.add(egui::Shape::convex_polygon(
        points,
        egui::Color32::TRANSPARENT,
        egui::Stroke::new(2.0, color),
    ));
}
