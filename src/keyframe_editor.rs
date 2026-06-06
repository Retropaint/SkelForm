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
    edit_mode: &EditMode,
    camera: &Camera,
) {
    let sel = selections.clone();
    let mut sel_frame = selections.anim_frame;

    // navigating frames with kb input
    if shared_ui.rename_id == "" && selections.anim != usize::MAX {
        let kfs = &armature.sel_anim(&sel).unwrap().keyframes;
        let next_kf = egui_ctx.input_mut(|i| i.consume_shortcut(&config.keys.next_keyframe));
        let prev_kf = egui_ctx.input_mut(|i| i.consume_shortcut(&config.keys.prev_keyframe));
        let right = egui_ctx.input_mut(|i| i.consume_shortcut(&config.keys.next_anim_frame));
        let left = egui_ctx.input_mut(|i| i.consume_shortcut(&config.keys.prev_anim_frame));

        if next_kf {
            for kf in kfs {
                if kf.frame > sel_frame {
                    sel_frame = kf.frame;
                    break;
                }
            }
        } else if prev_kf {
            let mut prev = 0;
            for kf in kfs {
                if kf.frame < sel_frame {
                    prev = kf.frame;
                } else {
                    break;
                }
            }
            sel_frame = prev;
        } else if right {
            let last_frame = kfs.last();
            sel_frame += 1;
            if last_frame != None && sel_frame > last_frame.unwrap().frame {
                sel_frame = 0;
            }
        } else if left {
            sel_frame -= 1;
            let last_frame = armature.sel_anim(&sel).unwrap().keyframes.last();
            if last_frame != None && sel_frame < 0 {
                sel_frame = last_frame.unwrap().frame;
            }
        }
    }
    if sel_frame != selections.anim_frame {
        events.select_anim_frame(sel_frame as usize, false, false);
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
                    .default_width(175.)
                    .max_width(300.)
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
                    timeline_editor(
                        ui, selections, armature, events, shared_ui, config, input, edit_mode,
                    );
                }
            });
            shared_ui.keyframe_panel_rect = Some(ui.min_rect());
        }),
        events,
        &egui_ctx,
        &camera,
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
            ui.add_space(13.);
            let str_new = shared_ui.loc("new");
            let button = ui.skf_button(str_new);
            if !button.clicked() {
                return;
            }
            events.new_animation();
            shared_ui.just_made_anim = true;
        });
    });
    ui.add_space(5.);
    egui::ScrollArea::vertical().show(ui, |ui| {
        let frame = egui::Frame::new()
            .fill(config.colors.dark_accent.into())
            .outer_margin(egui::Margin {
                right: 13,
                ..Default::default()
            });
        frame.show(ui, |ui| {
            let width = ui.available_width();
            let mut hovered = false;
            for i in 0..armature.animations.len() {
                let name = &mut armature.animations[i].name.clone();
                let context_id = format!("anim_{}", armature.animations[i].id);

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

                    // set up a focusable frame for the animation button
                    let rect = egui::Rect::from_min_size(
                        egui::Pos2::new(ui.cursor().left(), ui.cursor().top()),
                        egui::Vec2::new(width, 21.),
                    );
                    let id = egui::Id::new(format!("anim_{}", i.to_string()));
                    let button = ui
                        .interact(rect, id, egui::Sense::click())
                        .on_hover_cursor(cursor_icon);
                    ui.scope(|ui| {
                        ui.style_mut().interaction.selectable_labels = false;
                        egui::Frame::new().fill(col.into()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_width(width - button_padding);
                                ui.set_height(21.);
                                ui.add_space(5.);
                                let col = config.colors.text;
                                ui.label(egui::RichText::new(name.clone()).color(col));
                            });
                        });
                    });

                    if button.contains_pointer() || button.has_focus() {
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
                        shared_ui.last_selected = "anim".to_string();
                    }
                    if button.secondary_clicked() {
                        shared_ui.context_menu.show(&context_id);
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
        let hitbox = gap / shared_ui.timeline_zoom / fps / 2.;

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

            if shared_ui.lines_x.len() > 0 {
                draw_top_bar(
                    ui, width, hitbox, shared_ui, selections, armature, config, events,
                );
            }

            // The options bar has to be at the bottom, but it needs to be created first
            // so that the remaining height can be taken up by timeline graph.
            let timeline = ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                draw_bottom_bar(
                    ui, selections, &config, &armature, shared_ui, events, edit_mode,
                );
                draw_timeline_graph(
                    ui, width, hitbox, shared_ui, config, selections, armature, events, input,
                );
            });

            shared_ui.pointer_on_timeline = timeline.response.contains_pointer();
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
        .vertical_scroll_offset(shared_ui.timeline_offset.y)
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
                let str = format!("keyframe_editor.elements.{}", kf.element.to_string());
                let mut element_str = egui::RichText::new(shared_ui.loc(&str));
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
                        shared_ui.deleting_line_bone_id = kf.bone_id;
                        shared_ui.deleting_line_element = kf.element.clone();
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
        shared_ui.timeline_offset.y = response.state.offset.y;
    }
}

pub fn draw_top_bar(
    ui: &mut egui::Ui,
    width: f32,
    hitbox: f32,
    shared_ui: &mut crate::Ui,
    selections: &SelectionState,
    armature: &Armature,
    config: &Config,
    events: &mut EventState,
) {
    let scroll_area = egui::ScrollArea::horizontal()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .scroll_offset(egui::Vec2::new(shared_ui.timeline_offset.x, 0.));
    let sel = selections.clone();

    scroll_area.show(ui, |ui| {
        egui::Frame::new().show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(20.);

            let mut second = -1;
            for (i, x) in shared_ui.lines_x.iter().enumerate() {
                let pos = Vec2::new(ui.min_rect().left() + x, ui.min_rect().top() + 10.);
                if i as i32 % armature.sel_anim(&sel).unwrap().fps != 0 {
                    continue;
                }

                // if this frame line represents a second (1s, 2s, etc),
                // draw the number
                second += 1;
                // don't show 0s
                if second == 0 {
                    continue;
                }

                let center = egui::Align2::CENTER_CENTER;
                let fontid = egui::FontId::default();
                let col = config.colors.text;
                let painter = ui.painter_at(ui.min_rect());
                let str = second.to_string() + "s";
                painter.text(pos.into(), center, str, fontid, col.into());
            }

            let mut last_unique_frame = -1;
            let mut alt = false;
            for i in 0..armature.sel_anim(&sel).unwrap().keyframes.len() {
                let frame = armature.sel_anim(&sel).unwrap().keyframes[i].frame;
                if frame == last_unique_frame {
                    continue;
                }
                last_unique_frame = frame;

                // alternate diamond color
                let mut diamond_color;
                if alt {
                    diamond_color = Color::new(255, 255, 255, 0);
                    diamond_color -= Color::new(40, 40, 40, 0);
                } else {
                    diamond_color = Color::new(255, 255, 255, 0);
                }
                alt = !alt;

                // don't draw diamond if it's beyond the recorded lines
                if shared_ui.lines_x.len() - 1 < frame as usize {
                    break;
                }

                let pos = Vec2::new(
                    ui.min_rect().left() + shared_ui.lines_x[frame as usize] + 3.,
                    ui.min_rect().top() + 10.,
                );

                // create dragging area for diamond
                let rect = egui::Rect::from_center_size(pos.into(), egui::Vec2::splat(5.));
                let response: egui::Response =
                    ui.allocate_rect(rect, egui::Sense::click_and_drag());
                let mut diamond_size = 5.;

                // enlarge diamond if all of its keyframes are selected
                let keyframes = &armature.sel_anim(&sel).unwrap().keyframes;
                // used to determine if 'cmd' should be held when selecting
                // this diamond
                let mut is_selected = false;
                if shared_ui.selected_keyframes.len() > 0 {
                    let this_keyframes: Vec<&Keyframe> = keyframes
                        .iter()
                        .filter(|kf| kf.frame == last_unique_frame)
                        .collect();
                    let this_sel_kf: Vec<&Keyframe> = shared_ui
                        .selected_keyframes
                        .iter()
                        .filter(|kf| kf.frame == last_unique_frame)
                        .collect();
                    is_selected = this_keyframes.len() == this_sel_kf.len();
                    if is_selected {
                        diamond_color = Color::new(100, 255, 100, 0);
                        diamond_size += 2.;
                    }
                }

                if response.drag_started() {
                    events.select_anim_frame(frame as usize, true, is_selected);
                }

                if response.hovered() {
                    shared_ui.hovering_diamond = true;
                    shared_ui.cursor_icon = egui::CursorIcon::PointingHand;
                    diamond_size += 2.;
                }

                if response.clicked() {
                    shared_ui.last_selected = "keyframe".to_string();
                    events.select_anim_frame(frame as usize, true, is_selected);
                }

                if response.secondary_clicked() {
                    events.select_anim_frame(frame as usize, true, is_selected);
                    shared_ui.last_selected = "keyframe".to_string();
                    let kf = &armature.sel_anim(&sel).unwrap().keyframes[i];
                    let el = &(kf.element.clone() as usize);
                    let context_id =
                        &format!("keyframe_{}_{}_{}_{}", el, &kf.bone_id, &kf.frame, &i);
                    shared_ui.context_menu.show(context_id);
                }

                let cursor = get_cursor(ui);
                if response.dragged() {
                    shared_ui.cursor_icon = egui::CursorIcon::Grabbing;
                    shared_ui.dragged_keyframe = Keyframe {
                        frame,
                        bone_id: -1,
                        ..Default::default()
                    };

                    // draw diamond following mouse
                    let color = if cursor.y < 0. {
                        // red diamond if dragged above keyframe bar (being removed)
                        egui::Color32::RED
                    } else {
                        diamond_color.into()
                    };
                    let pos = cursor + ui.min_rect().left_top().into();
                    draw_diamond(&ui.ctx().debug_painter(), pos, color, 5.);
                }

                let kf = &shared_ui.dragged_keyframe;
                let not_dragging = kf.frame != frame || kf.bone_id != -1;
                if not_dragging {
                    // draw regular stationary diamond
                    draw_diamond(ui.painter(), pos, diamond_color.into(), diamond_size);
                } else {
                    // draw stationary diamond with lower opacity when dragging
                    let dc = diamond_color;
                    let white = egui::Color32::from_rgba_unmultiplied(dc.r, dc.g, dc.b, 30);
                    draw_diamond(ui.painter(), pos, white, 5.);
                }

                if !response.drag_stopped() {
                    continue;
                }

                let anim = armature.sel_anim(&sel).unwrap().clone();
                shared_ui.cursor_icon = egui::CursorIcon::Grabbing;

                // remove keyframe if dragged out
                if cursor.y < 0. {
                    events.delete_keyframes_by_frame(anim.keyframes[i].frame);
                    // break loop to prevent OOB errors
                    break;
                }

                // move all keyframes under this one over
                for j in 0..shared_ui.lines_x.len() {
                    let x = shared_ui.lines_x[j];
                    if cursor.x < x + hitbox && cursor.x > x - hitbox {
                        events.move_selected_keyframes(j as i32);
                        return;
                    }
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
            let area = egui::ScrollArea::both()
                .scroll_offset(shared_ui.timeline_offset.into())
                .id_salt("test");
            let response = area.show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(ui.available_height());

                let mut cursor = get_cursor(ui);
                // keep cursor on the frame
                cursor.y -= shared_ui.timeline_offset.y;

                // render darkened background after last keyframe
                let sel = selections.clone();
                let lkf = armature.sel_anim(&sel).unwrap().keyframes.last();
                if lkf != None && (lkf.unwrap().frame as usize) < shared_ui.lines_x.len() {
                    let left_top = egui::vec2(shared_ui.lines_x[lkf.unwrap().frame as usize], -3.);
                    let right_bot =
                        egui::vec2(0., shared_ui.bone_tops.tops.last().unwrap().height + 999.);

                    let rect_to_fill = egui::Rect::from_min_size(
                        ui.min_rect().left_top() + left_top,
                        ui.min_rect().size() + right_bot,
                    );
                    ui.painter()
                        .rect_filled(rect_to_fill, 0., config.colors.dark_accent);
                }

                draw_frame_lines(
                    ui, shared_ui, armature, config, input, selections, events, hitbox, cursor,
                );
            });
            if ui.ui_contains_pointer() {
                shared_ui.timeline_offset = response.state.offset.into();
            }
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

                let mut pressed = ui.input_mut(|i| i.consume_shortcut(&config.keys.play_animation));
                if button.clicked() {
                    pressed = true;
                }
                if !pressed || armature.sel_anim(&sel).unwrap().keyframes.len() == 0 {
                    return;
                }

                let anim = armature.sel_anim(&sel).unwrap();
                events.toggle_anim_playing(selections.anim, anim.elapsed == None);
            });

            let desc = shared_ui.loc("top_bar.view.zoom_in");
            let kb_tip = shared_ui
                .loc("keyframe_editor.zoom_kb_tip")
                .replace("$kb", &config.keys.timeline_zoom_mode.display());
            if ui.skf_button("+").on_hover_text(desc + &kb_tip).clicked() {
                shared_ui.timeline_zoom -= 0.1;
                shared_ui.timeline_zoom = shared_ui.timeline_zoom.max(0.1);
            }
            let desc = shared_ui.loc("top_bar.view.zoom_out");
            if ui.skf_button("-").on_hover_text(desc + &kb_tip).clicked() {
                shared_ui.timeline_zoom += 0.1;
                shared_ui.timeline_zoom = shared_ui.timeline_zoom.min(3.);
            }

            ui.add_space(5.);

            ui.label(&shared_ui.loc("keyframe_editor.frame"));
            ui.add(
                egui::DragValue::new(&mut selections.anim_frame)
                    .speed(0.1)
                    .update_while_editing(false),
            );

            let fps = armature.sel_anim(&sel).unwrap().fps;

            ui.label(&shared_ui.loc("keyframe_editor.fps"))
                .on_hover_text(&shared_ui.loc("keyframe_editor.frames_per_second"));
            let (edited, value, _) =
                ui.float_input("fps".to_string(), shared_ui, fps as f32, 1., None);
            if edited {
                events.adjust_keyframes_by_fps(value as usize);
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
    shared_ui.lines_x = vec![];
    let mut selected_line_x = 0.;
    let mut hovered_line_x = -1.;
    let panel = shared_ui.keyframe_panel_rect;
    let mut hovering = false;

    let range = egui::Rangef { min: 0., max: 999. };
    let painter = ui.painter();

    let mut x = 0.;
    let mut i = 0;
    while x < ui.min_rect().width() {
        x = i as f32 * hitbox * 2. + LINE_OFFSET;
        shared_ui.lines_x.push(x);

        let mut color: egui::Color32 = config.colors.frameline.into();
        let last_keyframe = armature.animations[selections.anim].keyframes.last();
        if last_keyframe != None && i > last_keyframe.unwrap().frame {
            color = config.colors.dark_accent.into();
        }
        let anim = &armature.animations[selections.anim];
        if i == anim.get_frame() && anim.elapsed != None {
            color = color + egui::Color32::from_rgb(60, 60, 60);
        }

        let below_top_bar = cursor.y < ui.min_rect().height() - 13.;
        let in_ui = cursor.y > -25.;
        let can_hover =
            in_ui && !shared_ui.modal && !shared_ui.settings_modal && !shared_ui.export_modal;
        let cur = cursor;
        let is_in = can_hover && cur.x < x + hitbox && cur.x > x - hitbox && below_top_bar;

        if selections.anim_frame == i {
            selected_line_x = ui.min_rect().left() + x;
        } else if i != 0 && i % anim.fps == 0 {
            color = color + egui::Color32::from_rgb(20, 20, 20);
        }
        if is_in && !shared_ui.hovering_diamond {
            color = egui::Color32::from_rgb(175, 175, 175);
            shared_ui.hovering_frame = i;
            hovering = true;
            hovered_line_x = ui.min_rect().left() + x;

            // select this frame if clicked
            if input.left_clicked && shared_ui.context_menu.id == "" {
                if !input.holding_mod {
                    shared_ui.selected_keyframes = vec![];
                }
                events.select_anim_frame(i as usize, false, false);
                shared_ui.last_selected = "keyframe".to_string();
            }
        }

        if !shared_ui.hovering_diamond && is_in && input.right_clicked {
            let context_id = format!("kfline_{}", i.to_string());
            shared_ui.context_menu.show(&context_id);
        }

        // draw the line!
        painter.vline(ui.min_rect().left() + x, range, Stroke { width: 2., color });

        i += 1;
    }

    if !hovering {
        shared_ui.hovering_frame = -1;
    }

    let mut last_bone = -1;
    for top in &shared_ui.bone_tops.tops {
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

    // draw hovered line
    if hovered_line_x != -1. {
        let color = egui::Color32::from_rgb(175, 175, 175);
        painter.vline(hovered_line_x, range, Stroke { width: 2., color });
    }

    // draw selected line
    let color = (config.colors.frameline + egui::Color32::from_rgb(100, 100, 100).into()).into();
    painter.vline(selected_line_x, range, Stroke { width: 2., color });

    shared_ui.hovering_diamond = false;

    // draw per-change icons
    let sel_anim = &armature.animations[selections.anim];
    let mut last_unique_frame = -1;
    let mut base_color = Color::new(255, 255, 255, 0);
    base_color -= Color::new(40, 40, 40, 0);
    for i in 0..sel_anim.keyframes.len() {
        let kf = sel_anim.keyframes[i].clone();
        let size = Vec2::new(17., 17.);

        // alternate colors between unique frames
        if last_unique_frame != kf.frame {
            last_unique_frame = kf.frame;
            if base_color == Color::new(255, 255, 255, base_color.a) {
                base_color = Color::new(255, 255, 255, 0);
                base_color -= Color::new(40, 40, 40, 0)
            } else {
                base_color = Color::new(255, 255, 255, 0);
            }
        }

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
        let x = shared_ui.lines_x[kf.frame as usize] + ui.min_rect().left();
        let pos = Vec2::new(x, top + size.y / 2.);
        if panel != None && pos.y > panel.unwrap().bottom() {
            continue;
        }

        let mut idx = kf.element.clone().clone() as usize;
        if idx > shared::ANIM_ICON_ID.len() - 1 {
            idx = shared::ANIM_ICON_ID.len() - 1;
        }

        let dkf = &shared_ui.dragged_keyframe;

        // make icon translucent if being dragged
        if *dkf == kf {
            base_color.a = 30;
        }

        let rect = egui::Rect::from_center_size(pos.into(), (size * 0.5).into());
        let response: egui::Response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

        let mut icon_size: egui::Vec2 = [20., 20.].into();
        let mut final_color = base_color;
        // expand icon if hovered on
        if response.hovered() || shared_ui.selected_keyframes.contains(&kf) {
            icon_size += [6., 6.].into();
            if shared_ui.selected_keyframes.contains(&kf) {
                final_color = Color::new(100, 255, 100, 0);
            }
            if response.hovered() {
                shared_ui.hovering_diamond = true;
                shared_ui.cursor_icon = egui::CursorIcon::PointingHand;
            }
        }

        // draw icon
        let offset: Vec2 = (icon_size / 2.).into();
        let img_rect = egui::Rect::from_min_size((pos - offset).into(), icon_size.into());
        egui::Image::new(&shared_ui.icon_images[shared::ANIM_ICON_ID[idx]])
            .tint(final_color)
            .paint_at(ui, img_rect);

        // select this frame if icon is clicked
        if response.clicked() {
            shared_ui.last_selected = "keyframe".to_string();
            events.select_anim_frame(kf.frame as usize, false, false);
            if input.holding_mod {
                add_selected_keyframes(&mut shared_ui.selected_keyframes, &kf);
            } else {
                shared_ui.selected_keyframes = vec![kf.clone()];
            }
        }

        // put this keyframe in selected if it's going to be dragged
        if response.drag_started() {
            if input.holding_mod || shared_ui.selected_keyframes.contains(&kf) {
                add_selected_keyframes(&mut shared_ui.selected_keyframes, &kf);
            } else {
                shared_ui.selected_keyframes = vec![kf.clone()];
            }
        }

        if response.dragged() {
            shared_ui.dragged_keyframe = kf.clone();
            shared_ui.cursor_icon = egui::CursorIcon::Grabbing;
            if let Some(cursor) = ui.ctx().pointer_latest_pos() {
                let pos = egui::Pos2::new(cursor.x - offset.x, cursor.y - offset.y);
                let drag_rect = egui::Rect::from_min_size(pos, size.into());
                egui::Image::new(&shared_ui.icon_images[shared::ANIM_ICON_ID[idx]])
                    .paint_at(ui, drag_rect);
            }
        }

        if response.secondary_clicked() {
            let context_id = &format!(
                "keyframe_{}_{}_{}_{}",
                &(kf.element.clone() as usize),
                &kf.bone_id,
                &kf.frame,
                &i
            );
            if input.holding_mod || shared_ui.selected_keyframes.contains(&kf) {
                add_selected_keyframes(&mut shared_ui.selected_keyframes, &kf);
            } else {
                shared_ui.selected_keyframes = vec![kf.clone()];
            }
            shared_ui.context_menu.show(context_id);
        }

        if response.drag_stopped() {
            // delete keyframes if cursor is above keyframe editor
            if cursor.y < 0. {
                events.delete_selected_keyframes();
                // break the loop to prevent OOB errors
                break;
            }

            // get the frame that cursor is on
            let mut dropped_frame = 0;
            for j in 0..shared_ui.lines_x.len() {
                let x = shared_ui.lines_x[j];
                if cursor.x < x + hitbox && cursor.x > x - hitbox {
                    dropped_frame = j;
                    break;
                }
            }

            events.move_selected_keyframes(dropped_frame as i32);
            break;
        }
    }

    // create extra space at the bottom
    if shared_ui.bone_tops.tops.len() > 0 {
        let height = shared_ui.bone_tops.tops.last().unwrap().height;
        let rect = egui::Rect::from_min_size(egui::pos2(0., height), egui::Vec2::new(1., 40.));
        ui.add_space(40.);
        ui.allocate_rect(rect, egui::Sense::empty());
    }
}

pub fn draw_diamond(painter: &egui::Painter, pos: Vec2, color: egui::Color32, size: f32) {
    let points = vec![
        egui::Pos2::new(pos.x, pos.y - size), // Top
        egui::Pos2::new(pos.x + size, pos.y), // Right
        egui::Pos2::new(pos.x, pos.y + size), // Bottom
        egui::Pos2::new(pos.x - size, pos.y), // Left
    ];

    painter.add(egui::Shape::convex_polygon(
        points,
        color,
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

fn add_selected_keyframes(selected_keyframes: &mut Vec<Keyframe>, kf: &Keyframe) {
    if !selected_keyframes.contains(&kf) {
        selected_keyframes.push(kf.clone());
    }
}
