//! Animation keyframe editor. Very early and only proof-of-concept.

use egui::Stroke;

use ui::{EguiUi, TextInputOptions};

use crate::*;

const LINE_OFFSET: f32 = 30.;

const HELP_DONE: &str = "Congratulations, you've made your first animation!\n\nYou may proceed with fleshing out the armature and animation, or read the user docs to learn more.\n\nHave fun with SkelForm!";

pub fn draw(egui_ctx: &egui::Context, shared: &mut Shared) {
    if shared.input.mouse_left == -1 {
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
                ui.gradient(
                    ui.ctx().screen_rect(),
                    egui::Color32::TRANSPARENT,
                    shared.config.ui_colors.gradient.into(),
                );
                shared.ui.camera_bar_pos.y = ui.min_rect().top();

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
                                ui.vertical(|ui| {
                                    draw_animations_list(ui, shared);
                                })
                            });
                        });

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
    ui.horizontal(|ui| {
        ui.heading("Animation");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
            let button = ui.skf_button("New");
            ui::draw_tutorial_rect(TutorialStep::CreateAnim, button.rect, shared, ui);

            if !button.clicked() {
                return;
            }

            shared.undo_actions.push(Action {
                action: ActionEnum::Animations,
                animations: shared.armature.animations.clone(),
                ..Default::default()
            });

            shared.armature.new_animation();
            let idx = shared.armature.animations.len() - 1;
            shared.ui.original_name = "".to_string();
            shared.ui.rename_id = "animation ".to_owned() + &idx.to_string();
            shared.ui.edit_value = Some("".to_string());
        });
    });
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.set_width(ui.available_width());
        for i in 0..shared.armature.animations.len() {
            let name = &mut shared.armature.animations[i].name.clone();

            // show input field if renaming
            if shared.ui.rename_id == "animation ".to_string() + &i.to_string() {
                let (edited, value, _) = ui.text_input(
                    "animation ".to_string() + &i.to_string(),
                    shared,
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
                    shared
                        .ui
                        .start_next_tutorial_step(TutorialStep::SelectKeyframe, &shared.armature);
                }
                continue;
            }

            macro_rules! activate_renaming {
                () => {
                    shared.ui.rename_id = "animation ".to_string() + &i.to_string();
                    shared.ui.edit_value = Some(name.to_string());
                };
            }

            ui.horizontal(|ui| {
                let button = ui::selection_button(&name, i == shared.ui.anim.selected, ui);
                if button.clicked() {
                    if shared.ui.anim.selected != i {
                        shared.ui.anim.selected = i;
                        shared.ui.select_anim_frame(0);
                    } else {
                        activate_renaming!();
                    }
                }
                if button.secondary_clicked() {
                    shared
                        .ui
                        .context_menu
                        .show(ContextType::Animation, i as i32);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    let anim = &mut shared.armature.animations[i];
                    let icon = if anim.elapsed == None { "⏵" } else { "⏹" };
                    if ui.skf_button(icon).clicked() {
                        anim.elapsed = if anim.elapsed == None {
                            Some(Instant::now())
                        } else {
                            None
                        };
                    }
                });

                if shared.ui.context_menu.is(ContextType::Animation, i as i32) {
                    button.show_tooltip_ui(|ui| {
                        if ui.clickable_label("Rename").clicked() {
                            activate_renaming!();
                            shared.ui.context_menu.close();
                        };
                        if ui.clickable_label("Delete").clicked() {
                            shared.ui.open_polar_modal(
                                PolarId::DeleteAnim,
                                "Are you sure to delete this animation?",
                            );

                            // only hide the menu, as anim id is still needed for modal
                            shared.ui.context_menu.hide = true;
                        }
                        if ui.ui_contains_pointer() {
                            shared.ui.context_menu.keep = true;
                        }
                    });
                }
            });
        }
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
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::ZERO,
                    shared.config.ui_colors.light_accent,
                );

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

            // keep track of elements, to prevent showing multiple of the same
            let mut added_elements: Vec<AnimElement> = vec![];

            for i in 0..keyframes.len() {
                let kf = &keyframes[i];

                if last_bone_id != kf.bone_id {
                    let label = ui
                        .label(shared.armature.find_bone(kf.bone_id).unwrap().name.clone())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .interact(egui::Sense::click());
                    if label.clicked() {
                        shared.ui.selected_bone_idx = shared
                            .armature
                            .bones
                            .iter()
                            .position(|b| b.id == kf.bone_id)
                            .unwrap();

                        shared.armature.unfold_to_bone(kf.bone_id);
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
                        draw_diamond(
                            &ui.ctx().debug_painter(),
                            cursor + ui.min_rect().left_top().into(),
                            color,
                        );
                    }

                    let kf = &shared.ui.anim.dragged_keyframe;
                    if kf.frame != frame || kf.bone_id != -1 {
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

                    add_anim_action(shared);

                    shared.cursor_icon = egui::CursorIcon::Grabbing;

                    // remove keyframe if dragged out
                    if cursor.y < 0. {
                        let frame = shared.selected_animation_mut().unwrap().keyframes[i].frame;
                        shared
                            .selected_animation_mut()
                            .unwrap()
                            .remove_all_keyframes_of_frame(frame);
                        // break loop to prevent OOB errors
                        break;
                    }

                    // move all keyframes under this one over
                    for j in 0..shared.ui.anim.lines_x.len() {
                        let x = shared.ui.anim.lines_x[j];
                        if !(cursor.x < x + hitbox && cursor.x > x - hitbox) {
                            continue;
                        }
                        shared
                            .selected_animation_mut()
                            .unwrap()
                            .remove_all_keyframes_of_frame(j as i32);
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
    let graph = ui
        .with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            egui::Frame::new()
                .fill(shared.config.ui_colors.light_accent.into())
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
                                shared.ui.anim.lines_x
                                    [shared.last_keyframe().unwrap().frame as usize],
                                -3.,
                            );
                            let right_bottom_rect = egui::vec2(0., 999.);

                            let rect_to_fill = egui::Rect::from_min_size(
                                ui.min_rect().left_top() + left_top_rect,
                                ui.min_rect().size() + right_bottom_rect,
                            );

                            ui.painter().rect_filled(
                                rect_to_fill,
                                0.,
                                shared.config.ui_colors.dark_accent,
                            );
                        }

                        draw_frame_lines(ui, shared, &bone_tops, hitbox, cursor);
                    });
                    shared.ui.anim.timeline_offset = response.state.offset.into();
                    shared.ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;
                });
        })
        .response;
    ui::draw_tutorial_rect(TutorialStep::SelectKeyframe, graph.rect, shared, ui);
}

pub fn draw_bottom_bar(ui: &mut egui::Ui, shared: &mut Shared) {
    egui::Frame::new().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.set_height(20.);
        ui.horizontal(|ui| {
            ui.painter_at(ui.min_rect()).rect_filled(
                ui.min_rect(),
                egui::CornerRadius::ZERO,
                shared.config.ui_colors.main,
            );

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                let play_str = if shared.selected_animation().unwrap().elapsed != None {
                    "Stop"
                } else {
                    "Play"
                };

                let play_text = egui::RichText::new(play_str).color(shared.config.ui_colors.text);

                let button = ui
                    .add_sized(
                        [50., 20.],
                        egui::Button::new(play_text)
                            .fill(shared.config.ui_colors.light_accent)
                            .corner_radius(0.),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                ui::draw_tutorial_rect(TutorialStep::PlayAnim, button.rect, shared, ui);
                ui::draw_tutorial_rect(TutorialStep::StopAnim, button.rect, shared, ui);

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
                if shared.ui.tutorial_step_is(TutorialStep::PlayAnim) {
                    shared
                        .ui
                        .start_next_tutorial_step(TutorialStep::StopAnim, &shared.armature);
                } else if !shared.ui.tutorial_step_is(TutorialStep::None) {
                    shared.ui.tutorial_step = TutorialStep::None;
                    shared.ui.open_modal(HELP_DONE.to_string(), false);
                }
            });

            if ui.skf_button("+").clicked() {
                shared.ui.anim.timeline_zoom -= 0.1;
            }
            if ui.skf_button("-").clicked() {
                shared.ui.anim.timeline_zoom += 0.1;
            }

            ui.add_space(20.);

            ui.label("Frame:");
            ui.add(egui::DragValue::new(&mut shared.ui.anim.selected_frame).speed(0.1));

            let fps = shared.selected_animation().unwrap().fps;

            ui.label("FPS:").on_hover_text("Frames Per Second");
            let (edited, value, _) = ui.float_input("fps".to_string(), shared, fps as f32, 1.);
            if edited {
                shared.selected_animation_mut().unwrap().fps = value as i32;
            }
            shared.ui.anim.bottom_bar_top = ui.min_rect().bottom() + 3.;

            if ui.skf_button("Copy").clicked() {
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

            if ui.skf_button("Paste").clicked() {
                add_anim_action(shared);

                let frame = shared.ui.anim.selected_frame;

                shared
                    .selected_animation_mut()
                    .unwrap()
                    .remove_all_keyframes_of_frame(frame);

                for kf in 0..shared.copy_buffer.keyframes.len() {
                    let keyframe = shared.copy_buffer.keyframes[kf].clone();
                    shared
                        .selected_animation_mut()
                        .unwrap()
                        .keyframes
                        .push(Keyframe { frame, ..keyframe })
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

        let mut color: egui::Color32 = shared.config.ui_colors.frameline.into();
        if shared.last_keyframe() != None && i > shared.last_keyframe().unwrap().frame {
            color = shared.config.ui_colors.dark_accent.into();
        }
        let anim = &mut shared.armature.animations[shared.ui.anim.selected];
        if i == anim.get_frame() && anim.elapsed != None {
            color = color + egui::Color32::from_rgb(60, 60, 60);
        }

        let above_scrollbar = cursor.y < ui.min_rect().height() - 13.;
        let in_ui = cursor.y > 0.;
        let in_modal =
            shared.ui.has_state(UiState::Modal) || shared.ui.has_state(UiState::SettingsModal);

        if shared.ui.anim.selected_frame == i {
            color = egui::Color32::WHITE;
        } else if !in_modal
            && in_ui
            && cursor.x < x + hitbox
            && cursor.x > x - hitbox
            && above_scrollbar
        {
            shared.cursor_icon = egui::CursorIcon::PointingHand;
            color = egui::Color32::WHITE;

            // select this frame if clicked
            if shared.input.mouse_left == 0 {
                shared.ui.anim.selected_frame = i;
                shared
                    .ui
                    .start_next_tutorial_step(TutorialStep::EditBoneAnim, &shared.armature);
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
        macro_rules! kf {
            () => {
                shared.selected_animation().unwrap().keyframes[i]
            };
        }
        let size = Vec2::new(17., 17.);

        // the Y position is based on this diamond's respective label
        let top: f32;
        if let Some(bone_top) = bone_tops.find(kf!().bone_id, &kf!().element.clone(), kf!().vert_id)
        {
            top = bone_top.height;
        } else {
            return;
        }
        let x = shared.ui.anim.lines_x[kf!().frame as usize] + ui.min_rect().left();
        let pos = Vec2::new(x, top + size.y / 2.);
        let offset = size / 2.;

        if top > height {
            height = top;
        }

        let rect = egui::Rect::from_min_size((pos - offset).into(), size.into());
        let mut idx = kf!().element.clone().clone() as usize;
        if idx > shared::ANIM_ICON_ID.len() - 1 {
            idx = shared::ANIM_ICON_ID.len() - 1;
        }

        let dkf = &shared.ui.anim.dragged_keyframe;
        let mut color = egui::Color32::WHITE;
        if *dkf == kf!() {
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
            shared.ui.anim.dragged_keyframe = kf!().clone();
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
            let k = shared
                .selected_animation()
                .unwrap()
                .keyframes
                .iter()
                .position(|kf| {
                    kf.bone_id == curr_kf.bone_id
                        && kf.element == curr_kf.element
                        && kf.frame == j as i32
                });
            let mut curr = i;
            if k != None {
                shared
                    .selected_animation_mut()
                    .unwrap()
                    .keyframes
                    .remove(k.unwrap());
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

pub fn add_anim_action(shared: &mut Shared) {
    shared.undo_actions.push(shared::Action {
        action: ActionEnum::Animation,
        id: shared.selected_animation().unwrap().id as i32,
        animations: vec![shared.selected_animation().unwrap().clone()],
        ..Default::default()
    });
}
