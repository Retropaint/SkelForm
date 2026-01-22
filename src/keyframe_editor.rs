//! Animation keyframe editor. Very early and only proof-of-concept.

use egui::Stroke;

use ui::{EguiUi, TextInputOptions};

use crate::*;

const LINE_OFFSET: f32 = 30.;

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    if !shared.input.left_down {
        shared.ui.anim.dragged_keyframe.frame = -1;
    }

    let sel = shared.selections.clone();

    // navigating frames with kb input
    if shared.ui.rename_id == "" {
        let right = egui_ctx.input_mut(|i| i.consume_shortcut(&shared.config.keys.next_anim_frame));
        let left = egui_ctx.input_mut(|i| i.consume_shortcut(&shared.config.keys.prev_anim_frame));
        if right {
            shared.selections.anim_frame += 1;
            let last_frame = shared.armature.sel_anim(&sel).unwrap().keyframes.last();
            if last_frame != None && shared.selections.anim_frame > last_frame.unwrap().frame {
                shared.selections.anim_frame = 0;
            }
        } else if left {
            shared.selections.anim_frame -= 1;
            let last_frame = shared.armature.sel_anim(&sel).unwrap().keyframes.last();
            if last_frame != None && shared.selections.anim_frame < 0 {
                shared.selections.anim_frame = last_frame.unwrap().frame;
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
                            draw_animations_list(
                                ui,
                                &mut shared.ui,
                                &shared.armature,
                                &shared.config,
                                &shared.selections,
                                &mut shared.events,
                            );
                        })
                    });
                });

                if shared.selections.anim != usize::MAX {
                    timeline_editor(ui, shared);
                }
            });
            shared.ui.keyframe_panel_rect = Some(ui.min_rect());
        }),
        &mut shared.camera.on_ui,
        &egui_ctx,
    );
}

fn draw_animations_list(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    armature: &Armature,
    config: &Config,
    selections: &crate::SelectionState,
    events: &mut EventState,
) {
    ui.horizontal(|ui| {
        let str_anim = shared_ui.loc("keyframe_editor.heading");
        ui.heading(str_anim);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
            let str_new = &&shared_ui.loc("new");
            let button = ui.skf_button(str_new);

            if !button.clicked() {
                return;
            }

            events.new_animation();
        });
    });
    egui::ScrollArea::vertical().show(ui, |ui| {
        let frame = egui::Frame::new().fill(config.colors.dark_accent.into());
        frame.show(ui, |ui| {
            let width = ui.available_width();
            let mut hovered = false;
            for i in 0..armature.animations.len() {
                let name = &mut armature.animations[i].name.clone();
                let context_id = "anim_".to_owned() + &i.to_string();

                // show input field if renaming
                if shared_ui.rename_id == context_id {
                    let str_new_anim = &shared_ui.loc("keyframe_editor.new_animation");
                    let options = Some(TextInputOptions {
                        focus: true,
                        placeholder: str_new_anim.to_string(),
                        default: str_new_anim.to_string(),
                        ..Default::default()
                    });
                    let (edited, value, _) =
                        ui.text_input(context_id, shared_ui, name.to_string(), options);
                    if edited {
                        events.rename_animation(i, value);
                        events.select_anim(i);
                    }
                    continue;
                }

                ui.horizontal(|ui| {
                    let has_kf = armature.animations[i].keyframes.len() > 0;
                    let button_padding = if has_kf { 25. } else { 0. };
                    let mut col = config.colors.dark_accent;
                    if i == shared_ui.hovering_anim as usize {
                        col += crate::Color::new(20, 20, 20, 0);
                    }
                    if i == selections.anim {
                        col += crate::Color::new(20, 20, 20, 0);
                    }
                    let cursor_icon = if selections.anim != i {
                        egui::CursorIcon::PointingHand
                    } else {
                        egui::CursorIcon::Default
                    };
                    //let button = ui::selection_button(&name, i == shared.selections.anim, ui);
                    let button = egui::Frame::new()
                        .fill(col.into())
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_width(width - button_padding);
                                ui.set_height(21.);
                                ui.add_space(5.);
                                let col = config.colors.text;
                                ui.label(egui::RichText::new(name.clone()).color(col));
                            });
                        })
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(cursor_icon);
                    if button.contains_pointer() {
                        shared_ui.hovering_anim = i as i32;
                        hovered = true;
                    }
                    if button.clicked() {
                        if selections.anim != i {
                            events.select_anim(i);
                        } else {
                            shared_ui.rename_id = context_id.clone();
                            shared_ui.edit_value = Some(name.to_string());
                        }
                    }

                    if armature.animations[i].keyframes.len() > 0 {
                        let anim = &armature.animations[i];
                        let align = egui::Layout::right_to_left(egui::Align::Center);
                        ui.with_layout(align, |ui| {
                            let icon = if anim.elapsed == None { "⏵" } else { "⏹" };
                            if ui.skf_button(icon).clicked() {
                                events.toggle_anim_playing(i, anim.elapsed == None);
                            }
                        });
                    }

                    context_menu!(button, shared_ui, context_id, |ui: &mut egui::Ui| {
                        ui.context_rename(shared_ui, config, context_id);
                        let del_anim = PolarId::DeleteAnim;
                        ui.context_delete(shared_ui, config, events, "delete_anim", del_anim);
                        let duplicate_str = shared_ui.loc("keyframe_editor.duplicate");
                        if ui.context_button(duplicate_str, config).clicked() {
                            events.duplicate_anim(i);
                            shared_ui.context_menu.close();
                        }
                    });
                });
            }

            if !hovered {
                shared_ui.hovering_anim = -1;
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

        let sel = shared.selections.clone();

        if shared.armature.sel_anim(&sel).unwrap().keyframes.len() > 0 {
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
        let fps = shared.armature.sel_anim(&sel).unwrap().fps as f32;
        let hitbox = gap / shared.ui.anim.timeline_zoom / fps / 2.;

        // add 1 second worth of frames after the last keyframe
        let frames: i32;
        let extra = shared.armature.sel_anim(&sel).unwrap().fps * 5;
        if shared.armature.sel_anim(&sel).unwrap().keyframes.last() != None {
            let sel_anim = shared.armature.sel_anim(&sel).unwrap();
            frames = sel_anim.keyframes.last().unwrap().frame + extra;
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
    let sel = &shared.selections;
    let scroll_area = egui::ScrollArea::vertical()
        .id_salt("bones_list")
        .vertical_scroll_offset(shared.ui.anim.timeline_offset.y)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
    scroll_area.show(ui, |ui| {
        // sort keyframes by element & bone
        let mut keyframes = shared.armature.sel_anim(sel).unwrap().keyframes.clone();
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
                    shared.events.select_bone(sel.unwrap());

                    let parents = shared.armature.get_all_parents(kf.bone_id);
                    for parent in &parents {
                        let bones = &shared.armature.bones;
                        let idx = bones.iter().position(|b| b.id == parent.id).unwrap();
                        let folded = shared.armature.bones[idx].folded;
                        shared.events.toggle_bone_folded(idx, false);
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
                        .ui
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
    let sel = shared.selections.clone();

    scroll_area.show(ui, |ui| {
        egui::Frame::new().show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(20.);

            let mut second = 0;
            for (i, x) in shared.ui.anim.lines_x.iter().enumerate() {
                if i as i32 % shared.armature.sel_anim(&sel).unwrap().fps != 0 {
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

            for i in 0..shared.armature.sel_anim(&sel).unwrap().keyframes.len() {
                let frame = shared.armature.sel_anim(&sel).unwrap().keyframes[i].frame;

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
                    shared.events.select_anim_frame(frame as usize);
                }

                if response.hovered() {
                    shared.ui.cursor_icon = egui::CursorIcon::Grab;
                }

                let cursor = get_cursor(ui);

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

                let anim = shared.armature.sel_anim(&sel).unwrap().clone();
                shared.undo_states.new_undo_anim(&anim);

                shared.ui.cursor_icon = egui::CursorIcon::Grabbing;

                // remove keyframe if dragged out
                if cursor.y < 0. {
                    let anim = shared.armature.sel_anim_mut(&sel).unwrap();
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
                    let selected_anim = &mut shared.armature.sel_anim_mut(&sel).unwrap();
                    selected_anim.keyframes.retain(|kf| kf.frame != j as i32);
                    for kf in &mut shared.armature.sel_anim_mut(&sel).unwrap().keyframes {
                        if kf.frame == frame as i32 {
                            kf.frame = j as i32;
                        }
                    }
                    shared.armature.sel_anim_mut(&sel).unwrap().sort_keyframes();
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

                let mut cursor = get_cursor(ui);
                // keep cursor on the frame
                cursor.y -= shared.ui.anim.timeline_offset.y;

                // render darkened background after last keyframe
                let sel = shared.selections.clone();
                let lkf = shared.armature.sel_anim(&sel).unwrap().keyframes.last();
                let frame = lkf.unwrap().frame as usize;
                if lkf != None && frame < shared.ui.anim.lines_x.len() {
                    let left_top_rect = egui::vec2(shared.ui.anim.lines_x[frame], -3.);
                    let right_bottom_rect = egui::vec2(0., 999.);

                    let rect_to_fill = egui::Rect::from_min_size(
                        ui.min_rect().left_top() + left_top_rect,
                        ui.min_rect().size() + right_bottom_rect,
                    );

                    ui.painter()
                        .rect_filled(rect_to_fill, 0., shared.config.colors.dark_accent);
                }

                draw_frame_lines(
                    ui,
                    &mut shared.ui,
                    &mut shared.armature,
                    &mut shared.config,
                    &mut shared.input,
                    &mut shared.selections,
                    &mut shared.events,
                    &bone_tops,
                    hitbox,
                    cursor,
                );
            });
            shared.ui.anim.timeline_offset = response.state.offset.into();
            shared.ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;
        });
    });
}

pub fn draw_bottom_bar(ui: &mut egui::Ui, shared: &mut Shared) {
    let sel = shared.selections.clone();
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
                let play_str = if shared.armature.sel_anim(&sel).unwrap().elapsed != None {
                    &shared.ui.loc("keyframe_editor.pause")
                } else {
                    &shared.ui.loc("keyframe_editor.play")
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
                if !pressed || shared.armature.sel_anim(&sel).unwrap().keyframes.len() == 0 {
                    return;
                }

                let anim = shared.armature.sel_anim_mut(&sel).unwrap();
                anim.elapsed = if anim.elapsed == None {
                    Some(Instant::now())
                } else {
                    None
                };
                shared.ui.anim.played_frame = shared.selections.anim_frame;
            });

            if ui.skf_button("+").clicked() {
                shared.ui.anim.timeline_zoom -= 0.1;
            }
            if ui.skf_button("-").clicked() {
                shared.ui.anim.timeline_zoom += 0.1;
            }

            ui.add_space(20.);

            ui.label(&shared.ui.loc("keyframe_editor.frame"));
            ui.add(egui::DragValue::new(&mut shared.selections.anim_frame).speed(0.1));

            let fps = shared.armature.sel_anim(&sel).unwrap().fps;

            ui.label(&shared.ui.loc("keyframe_editor.fps"))
                .on_hover_text(&shared.ui.loc("keyframe_editor.frames_per_second"));
            let (edited, value, _) =
                ui.float_input("fps".to_string(), &mut shared.ui, fps as f32, 1., None);
            if edited {
                let anim_mut = shared.armature.sel_anim_mut(&sel).unwrap();

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
                    shared
                        .events
                        .open_modal("keyframe_editor.invalid_fps", false);
                }
            }
            shared.ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;

            if ui
                .skf_button(&shared.ui.loc("keyframe_editor.copy"))
                .clicked()
            {
                shared.copy_buffer = CopyBuffer::default();
                for kf in 0..shared.armature.sel_anim(&sel).unwrap().keyframes.len() {
                    let frame = shared.selections.anim_frame;
                    if shared.armature.sel_anim(&sel).unwrap().keyframes[kf].frame == frame {
                        let keyframe =
                            shared.armature.sel_anim(&sel).unwrap().keyframes[kf].clone();
                        shared.copy_buffer.keyframes.push(keyframe);
                    }
                }
            }

            let paste_str = &shared.ui.loc("keyframe_editor.paste");
            if ui.skf_button(paste_str).clicked() {
                let anim = shared.armature.sel_anim(&sel).unwrap().clone();
                shared.undo_states.new_undo_anim(&anim);

                let frame = shared.selections.anim_frame;
                let buffer_frames = shared.copy_buffer.keyframes.clone();
                let anim = &mut shared.armature.sel_anim_mut(&sel).unwrap();

                anim.keyframes.retain(|kf| kf.frame != frame);

                for kf in 0..buffer_frames.len() {
                    let keyframe = buffer_frames[kf].clone();
                    anim.keyframes.push(Keyframe { frame, ..keyframe })
                }

                shared.armature.sel_anim_mut(&sel).unwrap().sort_keyframes();
            }
        });
    });
}

/// Draw all lines representing frames in the timeline.
fn draw_frame_lines(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    armature: &Armature,
    config: &Config,
    input: &InputStates,
    selections: &SelectionState,
    events: &mut EventState,
    bone_tops: &BoneTops,
    hitbox: f32,
    cursor: Vec2,
) {
    shared_ui.anim.lines_x = vec![];

    let mut x = 0.;
    let mut i = 0;
    while x < ui.min_rect().width() {
        x = i as f32 * hitbox * 2. + LINE_OFFSET;

        shared_ui.anim.lines_x.push(x);

        let mut color: egui::Color32 = config.colors.frameline.into();
        let last_keyframe = armature.animations[selections.anim].keyframes.last();
        if last_keyframe != None && i > last_keyframe.unwrap().frame {
            color = config.colors.dark_accent.into();
        }
        let anim = &armature.animations[selections.anim];
        if i == anim.get_frame() && anim.elapsed != None {
            color = color + egui::Color32::from_rgb(60, 60, 60);
        }

        let above_bar = cursor.y < ui.min_rect().height() - 13.;
        let in_ui = cursor.y > 0.;
        let in_modal = shared_ui.modal || shared_ui.settings_modal;
        let cur = cursor;

        if selections.anim_frame == i {
            color = egui::Color32::WHITE;
        } else if !in_modal && in_ui && cur.x < x + hitbox && cur.x > x - hitbox && above_bar {
            shared_ui.cursor_icon = egui::CursorIcon::PointingHand;
            color = egui::Color32::WHITE;

            // select this frame if clicked
            if input.left_clicked {
                events.select_anim_frame(i as usize);
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
    let sel_anim = &armature.animations[selections.anim];
    for i in 0..sel_anim.keyframes.len() {
        let kf = sel_anim.keyframes[i].clone();
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
        let x = shared_ui.anim.lines_x[kf.frame as usize] + ui.min_rect().left();
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

        let dkf = &shared_ui.anim.dragged_keyframe;
        let mut color = egui::Color32::WHITE;
        if *dkf == kf {
            color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30);
        }

        egui::Image::new(&shared_ui.anim.icon_images[shared::ANIM_ICON_ID[idx]])
            .tint(color)
            .paint_at(ui, rect);

        let rect = egui::Rect::from_center_size(pos.into(), (size * 0.5).into());
        let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

        if response.hovered() {
            shared_ui.cursor_icon = egui::CursorIcon::Grab;
        }

        if response.dragged() {
            shared_ui.anim.dragged_keyframe = kf.clone();
            if let Some(cursor) = ui.ctx().pointer_latest_pos() {
                let pos = egui::Pos2::new(cursor.x - offset.x, cursor.y - offset.y);
                let drag_rect = egui::Rect::from_min_size(pos, size.into());
                egui::Image::new(&shared_ui.anim.icon_images[shared::ANIM_ICON_ID[idx]])
                    .paint_at(ui, drag_rect);
            }
        }

        if !response.drag_stopped() {
            continue;
        }

        if cursor.y < 0. {
            events.delete_keyframe(i);
            // break the loop to prevent OOB errors
            break;
        }

        for j in 0..shared_ui.anim.lines_x.len() {
            let x = shared_ui.anim.lines_x[j];
            if !(cursor.x < x + hitbox && cursor.x > x - hitbox) {
                continue;
            }

            let curr_kf = &sel_anim.keyframes[i];

            // ignore if icon is dragged to the same line
            if curr_kf.frame == j as i32 {
                return;
            }

            // remove keyframe that is the same as this
            let keyframes = sel_anim.keyframes.clone();
            let k = keyframes.iter().position(|kf| {
                kf.bone_id == curr_kf.bone_id
                    && kf.element == curr_kf.element
                    && kf.frame == j as i32
            });
            let mut curr = i;
            if k != None {
                events.delete_keyframe(k.unwrap());
                curr -= 1;
            }

            events.set_keyframe_frame(curr, j);
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

pub fn get_cursor(ui: &egui::Ui) -> Vec2 {
    let cursor_pos = ui.ctx().input(|i| {
        if let Some(result) = i.pointer.hover_pos() {
            result
        } else {
            egui::Pos2::new(0., 0.)
        }
    });
    (cursor_pos - ui.min_rect().left_top()).into()
}
