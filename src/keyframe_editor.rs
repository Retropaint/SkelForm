//! Animation keyframe editor. Very early and only proof-of-concept.

use egui::Stroke;

use ui::{EguiUi, TextInputOptions};

use crate::*;

const LINE_OFFSET: f32 = 30.;

pub fn draw(
    egui_ctx: &egui::Context,
    shared_ui: &mut crate::Ui,
    input: &InputStates,
    armature: &mut Armature,
    config: &Config,
    selections: &mut SelectionState,
    events: &mut EventState,
    copy_buffer: &mut CopyBuffer,
    edit_mode: &EditMode,
) {
    if !input.left_down {
        shared_ui.anim.dragged_keyframe.frame = -1;
    }

    let sel = selections.clone();

    // navigating frames with kb input
    if shared_ui.rename_id == "" {
        let right = egui_ctx.input_mut(|i| i.consume_shortcut(&config.keys.next_anim_frame));
        let left = egui_ctx.input_mut(|i| i.consume_shortcut(&config.keys.prev_anim_frame));
        if right {
            selections.anim_frame += 1;
            let last_frame = armature.sel_anim(&sel).unwrap().keyframes.last();
            if last_frame != None && selections.anim_frame > last_frame.unwrap().frame {
                selections.anim_frame = 0;
            }
        } else if left {
            selections.anim_frame -= 1;
            let last_frame = armature.sel_anim(&sel).unwrap().keyframes.last();
            if last_frame != None && selections.anim_frame < 0 {
                selections.anim_frame = last_frame.unwrap().frame;
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
                config.colors.gradient.into(),
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
                                ui, shared_ui, armature, config, selections, events,
                            );
                        })
                    });
                });

                if selections.anim != usize::MAX {
                    #[rustfmt::skip]
                    timeline_editor(ui, selections, armature, events, shared_ui, config, input, copy_buffer, edit_mode);
                }
            });
            shared_ui.keyframe_panel_rect = Some(ui.min_rect());
        }),
        events,
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
            let str_new = shared_ui.loc("new");
            let button = ui.skf_button(str_new);
            if !button.clicked() {
                return;
            }
            events.new_animation();
            shared_ui.just_made_anim = true;
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

fn timeline_editor(
    ui: &mut egui::Ui,
    selections: &mut SelectionState,
    armature: &mut Armature,
    events: &mut EventState,
    shared_ui: &mut crate::Ui,
    config: &Config,
    input: &InputStates,
    copy_buffer: &mut CopyBuffer,
    edit_mode: &EditMode,
) {
    let frame = egui::Frame::new().outer_margin(egui::Margin {
        left: 0,
        ..Default::default()
    });
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_height(ui.available_height());

        // track the Y of bone change labels for their diamonds
        shared_ui.bone_tops = BoneTops::default();

        let sel = selections.clone();

        if armature.sel_anim(&sel).unwrap().keyframes.len() > 0 {
            let frame = egui::Frame::new().inner_margin(egui::Margin {
                top: 27,
                bottom: 27,
                left: 0,
                right: 0,
            });
            frame.show(ui, |ui| {
                ui.vertical(|ui| {
                    draw_bones_list(ui, selections, armature, config, shared_ui, events);
                });
            });
        }

        // calculate how far apart each keyframe should visually be
        let gap = 400.;
        let fps = armature.sel_anim(&sel).unwrap().fps as f32;
        let hitbox = gap / shared_ui.anim.timeline_zoom / fps / 2.;

        // add 1 second worth of frames after the last keyframe
        let frames: i32;
        let extra = armature.sel_anim(&sel).unwrap().fps * 5;
        if armature.sel_anim(&sel).unwrap().keyframes.last() != None {
            let sel_anim = armature.sel_anim(&sel).unwrap();
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
            let light_accent = config.colors.light_accent;
            let zero_corner = egui::CornerRadius::ZERO;
            ui.painter().rect_filled(rect, zero_corner, light_accent);

            if shared_ui.anim.lines_x.len() > 0 {
                draw_top_bar(
                    ui, width, hitbox, shared_ui, selections, armature, input, config, events,
                );
            }

            // The options bar has to be at the bottom, but it needs to be created first
            // so that the remaining height can be taken up by timeline graph.
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                draw_bottom_bar(
                    ui,
                    selections,
                    &config,
                    &armature,
                    shared_ui,
                    events,
                    copy_buffer,
                    edit_mode,
                );
                draw_timeline_graph(
                    ui, width, hitbox, shared_ui, config, selections, armature, events, input,
                );
            });
        });
    });
}

pub fn draw_bones_list(
    ui: &mut egui::Ui,
    selections: &SelectionState,
    armature: &Armature,
    config: &Config,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
) {
    ui.set_width(150.);
    let sel = &selections;
    let scroll_area = egui::ScrollArea::vertical()
        .id_salt("bones_list")
        .vertical_scroll_offset(shared_ui.anim.timeline_offset.y)
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
    let response = scroll_area.show(ui, |ui| {
        // sort keyframes by element & bone
        let mut keyframes = armature.sel_anim(sel).unwrap().keyframes.clone();
        keyframes.sort_by(|a, b| (a.element.clone() as i32).cmp(&(b.element.clone() as i32)));
        keyframes.sort_by(|a, b| a.bone_id.cmp(&b.bone_id));

        let mut last_bone_id = -1;
        let mut first = true;

        // keep track of elements, to prevent showing multiple of the same
        let mut added_elements: Vec<AnimElement> = vec![];

        for i in 0..keyframes.len() {
            let kf = &keyframes[i];

            let bones = &armature.bones;
            let bone = bones.iter().find(|b| b.id == kf.bone_id).unwrap();
            let highlighted =
                selections.bone_ids.len() != 0 && selections.bone_ids[0] == kf.bone_id;

            if last_bone_id != kf.bone_id {
                if !first {
                    ui.separator();
                }
                first = false;
                let mut bone_str = egui::RichText::new(bone.name.clone());
                if highlighted {
                    bone_str = bone_str.strong();
                }
                let label = ui
                    .label(bone_str)
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .interact(egui::Sense::click());
                if label.clicked() {
                    let kf_id = kf.bone_id;
                    let sel = armature.bones.iter().position(|b| b.id == kf_id);
                    events.select_bone(sel.unwrap(), false);

                    let parents = armature.get_all_parents(false, kf.bone_id);
                    for parent in &parents {
                        let bones = &armature.bones;
                        let idx = bones.iter().position(|b| b.id == parent.id).unwrap();
                        events.toggle_bone_folded(idx, false);
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
                let str = &("keyframe_editor.elements.".to_owned() + &kf.element.to_string());
                let mut element_str = egui::RichText::new(shared_ui.loc(str));
                if highlighted {
                    element_str = element_str.strong();
                }
                let label = ui.label(element_str);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut col = config.colors.text;
                    col -= Color::new(60, 60, 60, 0);
                    let text = egui::RichText::new("🗑").size(15.).color(col);
                    let pointing_hand = egui::CursorIcon::PointingHand;
                    let label = ui.label(text).on_hover_cursor(pointing_hand);
                    if label.clicked() {
                        shared_ui.anim.deleting_line_bone_id = kf.bone_id;
                        shared_ui.anim.deleting_line_element = kf.element.clone();
                        events.open_polar_modal(
                            PolarId::DeleteKeyframeLine,
                            shared_ui.loc("polar.delete_keyframe_line"),
                        );
                    }
                });
                shared_ui.bone_tops.tops.push(BoneTop {
                    id: kf.bone_id,
                    element: kf.element.clone(),
                    height: label.rect.top(),
                });
            });
        }

        ui.add_space(20.);
    });

    if ui.ui_contains_pointer() {
        shared_ui.anim.timeline_offset.y = response.state.offset.y;
    }
}

pub fn draw_top_bar(
    ui: &mut egui::Ui,
    width: f32,
    hitbox: f32,
    shared_ui: &mut crate::Ui,
    selections: &SelectionState,
    armature: &mut Armature,
    input: &InputStates,
    config: &Config,
    events: &mut EventState,
) {
    let mut drew_drag = false;
    let scroll_area = egui::ScrollArea::horizontal()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .scroll_offset(egui::Vec2::new(shared_ui.anim.timeline_offset.x, 0.));
    let sel = selections.clone();

    scroll_area.show(ui, |ui| {
        egui::Frame::new().show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(20.);

            let mut second = -1;
            for (i, x) in shared_ui.anim.lines_x.iter().enumerate() {
                if i as i32 % armature.sel_anim(&sel).unwrap().fps != 0 {
                    continue;
                }
                second += 1;
                let pos = Vec2::new(ui.min_rect().left() + x, ui.min_rect().top() + 10.);
                let center = egui::Align2::CENTER_CENTER;
                let fontid = egui::FontId::default();
                let col = config.colors.text;
                let painter = ui.painter_at(ui.min_rect());
                let str = second.to_string() + "s";
                painter.text(pos.into(), center, str, fontid, col.into());
            }

            for i in 0..armature.sel_anim(&sel).unwrap().keyframes.len() {
                let frame = armature.sel_anim(&sel).unwrap().keyframes[i].frame;

                // don't draw diamond if it's beyond the recorded lines
                if shared_ui.anim.lines_x.len() - 1 < frame as usize {
                    break;
                }

                let pos = Vec2::new(
                    ui.min_rect().left() + shared_ui.anim.lines_x[frame as usize] + 3.,
                    ui.min_rect().top() + 10.,
                );

                // create dragging area for diamond
                let rect = egui::Rect::from_center_size(pos.into(), egui::Vec2::splat(5.));
                let response: egui::Response = ui.allocate_rect(rect, egui::Sense::drag());

                if response.drag_started() {
                    events.select_anim_frame(frame as usize, false);
                }

                if response.hovered() {
                    shared_ui.cursor_icon = egui::CursorIcon::PointingHand;
                    if input.left_clicked {
                        events.select_anim_frame(frame as usize, true);
                    }
                }

                let cursor = get_cursor(ui);
                if response.dragged() {
                    shared_ui.cursor_icon = egui::CursorIcon::Grabbing;
                    shared_ui.anim.dragged_keyframe = Keyframe {
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

                let kf = &shared_ui.anim.dragged_keyframe;
                if kf.frame != frame || kf.bone_id != -1 {
                    draw_diamond(ui.painter(), pos, egui::Color32::WHITE);
                } else if !drew_drag {
                    let white = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30);
                    draw_diamond(ui.painter(), pos, white);
                    drew_drag = true;
                }

                if !response.drag_stopped() || input.left_clicked {
                    continue;
                }

                let anim = armature.sel_anim(&sel).unwrap().clone();

                shared_ui.cursor_icon = egui::CursorIcon::Grabbing;

                // remove keyframe if dragged out
                if cursor.y < 0. {
                    events.remove_keyframes_by_frame(anim.keyframes[i].frame);

                    // break loop to prevent OOB errors
                    break;
                }

                // move all keyframes under this one over
                for j in 0..shared_ui.anim.lines_x.len() {
                    let x = shared_ui.anim.lines_x[j];
                    if !(cursor.x < x + hitbox && cursor.x > x - hitbox) {
                        continue;
                    }
                    let selected_anim = &mut armature.sel_anim_mut(&sel).unwrap();
                    selected_anim.keyframes.retain(|kf| kf.frame != j as i32);
                    for kf in &mut armature.sel_anim_mut(&sel).unwrap().keyframes {
                        if kf.frame == frame as i32 {
                            kf.frame = j as i32;
                        }
                    }
                    armature.sel_anim_mut(&sel).unwrap().sort_keyframes();
                    return;
                }
            }
        });
    });
}

pub fn draw_timeline_graph(
    ui: &mut egui::Ui,
    width: f32,
    hitbox: f32,
    shared_ui: &mut crate::Ui,
    config: &Config,
    selections: &SelectionState,
    armature: &Armature,
    events: &mut EventState,
    input: &InputStates,
) {
    let layout = egui::Layout::left_to_right(egui::Align::Center);
    ui.with_layout(layout, |ui| {
        let frame = egui::Frame::new()
            .fill(config.colors.light_accent.into())
            .inner_margin(3);
        frame.show(ui, |ui| {
            let response = egui::ScrollArea::both().scroll_offset(shared_ui.anim.timeline_offset.into()).id_salt("test").show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(ui.available_height());

                let mut cursor = get_cursor(ui);
                // keep cursor on the frame
                cursor.y -= shared_ui.anim.timeline_offset.y;

                // render darkened background after last keyframe
                let sel = selections.clone();
                let lkf = armature.sel_anim(&sel).unwrap().keyframes.last();
                if lkf != None && (lkf.unwrap().frame as usize) < shared_ui.anim.lines_x.len() {
                    let left_top_rect = egui::vec2(shared_ui.anim.lines_x[lkf.unwrap().frame as usize], -3.);
                    let right_bottom_rect = egui::vec2(0., 999.);

                    let rect_to_fill = egui::Rect::from_min_size(
                        ui.min_rect().left_top() + left_top_rect,
                        ui.min_rect().size() + right_bottom_rect,
                    );

                    ui.painter()
                        .rect_filled(rect_to_fill, 0., config.colors.dark_accent);
                }

                #[rustfmt::skip]
                draw_frame_lines(ui, shared_ui, armature, config, input, selections, events, hitbox, cursor);
            });
            if ui.ui_contains_pointer() {
                shared_ui.anim.timeline_offset = response.state.offset.into();
            }
            shared_ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;
        });
    });
}

pub fn draw_bottom_bar(
    ui: &mut egui::Ui,
    selections: &mut SelectionState,
    config: &Config,
    armature: &Armature,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
    copy_buffer: &CopyBuffer,
    edit_mode: &EditMode,
) {
    let sel = selections.clone();
    egui::Frame::new().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_height(20.);
        ui.horizontal(|ui| {
            ui.painter_at(ui.min_rect()).rect_filled(
                ui.min_rect(),
                egui::CornerRadius::ZERO,
                config.colors.main,
            );

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                let play_str = if armature.sel_anim(&sel).unwrap().elapsed != None {
                    &shared_ui.loc("keyframe_editor.pause")
                } else {
                    &shared_ui.loc("keyframe_editor.play")
                };

                let play_text = egui::RichText::new(play_str).color(config.colors.text);
                let button = egui::Button::new(play_text)
                    .fill(config.colors.light_accent)
                    .corner_radius(0.);

                let button = ui
                    .add_sized([50., 20.], button)
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                let mut pressed = ui.input(|i| i.key_pressed(egui::Key::Space));
                if button.clicked() {
                    pressed = true;
                }
                if !pressed || armature.sel_anim(&sel).unwrap().keyframes.len() == 0 {
                    return;
                }

                let anim = armature.sel_anim(&sel).unwrap();
                events.toggle_anim_playing(selections.anim, anim.elapsed == None);
                shared_ui.anim.played_frame = selections.anim_frame;
            });

            if ui.skf_button("+").clicked() {
                shared_ui.anim.timeline_zoom -= 0.1;
                shared_ui.anim.timeline_zoom = shared_ui.anim.timeline_zoom.max(0.1);
            }
            if ui.skf_button("-").clicked() {
                shared_ui.anim.timeline_zoom += 0.1;
                shared_ui.anim.timeline_zoom = shared_ui.anim.timeline_zoom.min(3.);
            }

            ui.add_space(20.);

            ui.label(&shared_ui.loc("keyframe_editor.frame"));
            ui.add(egui::DragValue::new(&mut selections.anim_frame).speed(0.1));

            let fps = armature.sel_anim(&sel).unwrap().fps;

            ui.label(&shared_ui.loc("keyframe_editor.fps"))
                .on_hover_text(&shared_ui.loc("keyframe_editor.frames_per_second"));
            let (edited, value, _) =
                ui.float_input("fps".to_string(), shared_ui, fps as f32, 1., None);
            if edited {
                events.adjust_keyframes_by_fps(value as usize);
            }
            shared_ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;

            if ui
                .skf_button(&shared_ui.loc("keyframe_editor.copy"))
                .clicked()
            {
                events.copy_keyframes_in_frame();
            }

            let paste_str = &shared_ui.loc("keyframe_editor.paste");
            if ui.skf_button(paste_str).clicked() {
                events.paste_keyframes();
            }

            let mut col = config.colors.text;
            if !edit_mode.onion_layers {
                col -= Color::new(60, 60, 60, 0);
            }
            if ui
                .skf_button(egui::RichText::new("🌓").color(col))
                .on_hover_text(shared_ui.loc("keyframe_editor.onion_desc"))
                .clicked()
            {
                events.toggle_onion_layers(if edit_mode.onion_layers { 0 } else { 1 });
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
    hitbox: f32,
    cursor: Vec2,
) {
    shared_ui.anim.lines_x = vec![];

    let mut selected_line_x = 0.;
    let mut hovered_line_x = 0.;

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
            selected_line_x = ui.min_rect().left() + x;
        } else if !in_modal && in_ui && cur.x < x + hitbox && cur.x > x - hitbox && above_bar {
            hovered_line_x = ui.min_rect().left() + x;
            shared_ui.cursor_icon = egui::CursorIcon::PointingHand;
            color = egui::Color32::WHITE;

            // select this frame if clicked
            if input.left_clicked {
                events.select_anim_frame(i as usize, false);
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

    let mut last_bone = -1;
    let mut count = 0;
    for top in &shared_ui.bone_tops.tops {
        if count < 2 {
            count += 1;
            continue;
        }
        if last_bone != top.id {
            let range = egui::Rangef::new(0., ui.available_width());
            let mut col = config.colors.dark_accent;
            col -= Color::new(20, 20, 20, 0);
            let color = egui::Stroke::new(1.5, config.colors.dark_accent);
            ui.painter().hline(range, top.height - 25.5, color);
        }
        last_bone = top.id;
        continue;
    }

    // draw selected line
    ui.painter().vline(
        hovered_line_x,
        egui::Rangef { min: 0., max: 999. },
        Stroke {
            width: 2.,
            color: egui::Color32::WHITE,
        },
    );

    // draw selected line
    ui.painter().vline(
        selected_line_x,
        egui::Rangef { min: 0., max: 999. },
        Stroke {
            width: 2.,
            color: egui::Color32::WHITE,
        },
    );

    // draw per-change icons
    let sel_anim = &armature.animations[selections.anim];
    let mut context_response: Option<egui::Response> = None;
    let mut has_context = false;
    for i in 0..sel_anim.keyframes.len() {
        let kf = sel_anim.keyframes[i].clone();
        let size = Vec2::new(17., 17.);

        // the Y position is based on this diamond's respective label
        let top: f32;

        let el = kf.element.clone();
        let b_id = kf.bone_id;
        let tops = &shared_ui.bone_tops.tops;
        if let Some(b_top) = tops.iter().find(|bt| bt.id == b_id && bt.element == el) {
            top = b_top.height;
        } else {
            return;
        }
        let x = shared_ui.anim.lines_x[kf.frame as usize] + ui.min_rect().left();
        let pos = Vec2::new(x, top + size.y / 2.);
        let offset = size / 2.;

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
        let response: egui::Response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

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

        let context_id = &("keyframe_".to_string()
            + &(kf.element as usize).to_string()
            + &kf.bone_id.to_string()
            + &kf.frame.to_string());

        if response.secondary_clicked() {
            shared_ui.context_menu.show(context_id);
        }

        context_menu!(response, shared_ui, context_id, |ui: &mut egui::Ui| {
            if ui.context_button("Copy", &config).clicked() {
                events.copy_keyframe(i);
                shared_ui.context_menu.close();
            }

            if ui.context_button("Paste", &config).clicked() {
                events.paste_keyframes();
                shared_ui.context_menu.close();
            }
        });

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
    if shared_ui.bone_tops.tops.len() > 0 {
        let rect = egui::Rect::from_min_size(
            egui::pos2(0., shared_ui.bone_tops.tops.last().unwrap().height),
            egui::Vec2::new(1., 40.),
        );
        ui.add_space(40.);
        ui.allocate_rect(rect, egui::Sense::empty());
    }
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
