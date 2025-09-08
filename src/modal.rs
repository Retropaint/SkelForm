use crate::{
    armature_window,
    ui::{job_text, EguiUi},
    Action, ActionEnum, Config, PolarId, Shared, UiState, Vec2,
};

#[cfg(not(target_arch = "wasm32"))]
use crate::bone_panel;

use egui::Color32;

pub fn modal_template<T: FnOnce(&mut egui::Ui), E: FnOnce(&mut egui::Ui)>(
    ctx: &egui::Context,
    config: &Config,
    content: T,
    buttons: E,
) {
    egui::Modal::new("test".into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: config.colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., config.colors.light_accent),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(250.);
            content(ui);
            ui.add_space(20.);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                buttons(ui);
            })
        });
}

pub fn polar_modal(shared: &mut Shared, ctx: &egui::Context) {
    let mut yes = false;

    let headline = shared.ui.headline.to_string();

    modal_template(
        ctx,
        &shared.config,
        |ui| {
            ui.label(headline);
        },
        |ui| {
            let pressed_no = ui.input_mut(|i| i.consume_shortcut(&shared.config.keys.cancel));
            if ui.skf_button("No").clicked() || pressed_no {
                shared.ui.set_state(UiState::PolarModal, false);
            }
            if ui.skf_button("Yes").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                yes = true;
            }
        },
    );

    if !yes {
        return;
    }

    shared.ui.set_state(UiState::PolarModal, false);
    match shared.ui.polar_id {
        PolarId::DeleteBone => {
            shared.undo_actions.push(Action {
                action: ActionEnum::Bones,
                bones: shared.armature.bones.clone(),
                ..Default::default()
            });

            shared.ui.selected_bone_idx = usize::MAX;

            if shared.armature.find_bone(shared.ui.context_menu.id) == None {
                return;
            }

            let bone = shared
                .armature
                .find_bone(shared.ui.context_menu.id)
                .unwrap();

            // remove all children of this bone as well
            let mut children = vec![bone.clone()];
            armature_window::get_all_children(&shared.armature.bones, &mut children, bone);
            children.reverse();
            for bone in &children {
                shared.armature.delete_bone(bone.id);
            }

            // remove all references to this bone and it's children from all animations
            for bone in &children {
                for anim in &mut shared.armature.animations {
                    for i in 0..anim.keyframes.len() {
                        if anim.keyframes[i].bone_id == bone.id {
                            anim.keyframes.remove(i);
                        }
                    }
                }
            }

            // IK bones that target this are now -1
            let targeters = shared
                .armature
                .bones
                .iter_mut()
                .filter(|bone| bone.ik_target_id == shared.ui.context_menu.id);
            for bone in targeters {
                bone.ik_target_id = -1;
            }
        }
        PolarId::Exiting => shared.ui.set_state(UiState::Exiting, true),
        PolarId::DeleteAnim => {
            shared.ui.anim.selected = usize::MAX;
            shared.undo_actions.push(Action {
                action: ActionEnum::Animations,
                animations: shared.armature.animations.clone(),
                ..Default::default()
            });
            shared
                .armature
                .animations
                .remove(shared.ui.context_menu.id as usize);
            shared.ui.context_menu.close();
        }
        PolarId::DeleteFile => {
            std::fs::remove_file(&shared.ui.selected_path).unwrap();
        }
    }
}

pub fn modal(shared: &mut Shared, ctx: &egui::Context) {
    let headline = shared.ui.headline.to_string();
    modal_template(
        ctx,
        &shared.config,
        |ui| {
            ui.label(headline);
        },
        |ui| {
            if shared.ui.has_state(UiState::ForcedModal) || !ui.button("OK").clicked() {
                return;
            }

            shared.ui.set_state(UiState::Modal, false);
            shared.ui.headline = "".to_string();
        },
    )
}

pub fn image_modal(shared: &mut Shared, ctx: &egui::Context) {
    egui::Modal::new("test".into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: shared.config.colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., shared.config.colors.light_accent),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(300.);
            ui.set_height(400.);
            ui.heading(shared.loc("texture_modal.heading"));
            modal_x(ui, || {
                shared.ui.set_state(UiState::ImageModal, false);
            });

            ui.add_space(5.);

            let height = ui.available_height();

            ui.horizontal(|ui| {
                ui.set_height(height);
                let frame = egui::Frame::default().inner_margin(5.);
                let modal_width = ui.max_rect().width();
                let height = ui.available_height();
                let mut hovered_set: i32 = -1;
                ui.vertical(|ui| {
                    ui.set_height(height);
                    ui.set_width((modal_width / 2.) - 10.);

                    ui.horizontal(|ui| {
                        if shared.ui.hovering_tex != -1 {
                            ui.label(shared.loc("Texture Preview"));
                            return;
                        }
                        ui.label(shared.loc("texture_modal.sets"));
                        if !ui.skf_button(shared.loc("new")).clicked() {
                            return;
                        }
                        shared.armature.texture_sets.push(crate::TextureSet {
                            name: "".to_string(),
                            textures: vec![],
                        });
                        shared.ui.rename_id = "tex_set ".to_string()
                            + &(shared.armature.texture_sets.len() - 1).to_string();
                    });

                    let size = ui.available_size();
                    ui.dnd_drop_zone::<i32, _>(frame, |ui| {
                        ui.set_width(size.x);
                        ui.set_height(size.y - 10.);

                        if shared.ui.hovering_tex != -1 {
                            draw_tex_preview(shared, ui);
                            return;
                        }
                        for s in 0..shared.armature.texture_sets.len() {
                            macro_rules! set {
                                () => {
                                    shared.armature.texture_sets[s]
                                };
                            }

                            if shared.ui.rename_id == "tex_set ".to_string() + &s.to_string() {
                                let (edited, value, _) = ui.text_input(
                                    shared.ui.rename_id.clone(),
                                    shared,
                                    set!().name.clone(),
                                    Some(crate::ui::TextInputOptions {
                                        size: Vec2::new(ui.available_width(), 20.),
                                        focus: true,
                                        placeholder: shared
                                            .loc("texture_modal.new_set")
                                            .to_string(),
                                        default: shared.loc("texture_modal.new_set").to_string(),
                                        ..Default::default()
                                    }),
                                );
                                if edited {
                                    set!().name = value;
                                    shared.ui.selected_tex_set_idx = s as i32;
                                }
                                continue;
                            }

                            let mut col = shared.config.colors.light_accent;
                            if s as i32 == shared.ui.selected_tex_set_idx {
                                col += crate::Color::new(20, 20, 20, 0);
                            }
                            let cursor_icon = if shared.ui.selected_tex_set_idx != s as i32 {
                                egui::CursorIcon::PointingHand
                            } else {
                                egui::CursorIcon::Default
                            };
                            let button = ui
                                .add(egui::Button::new(set!().name.to_string()).fill(col))
                                .on_hover_cursor(cursor_icon);
                            let tex_len = shared.armature.texture_sets[s].textures.len();
                            if button.hovered() && tex_len > 0 {
                                hovered_set = s as i32;
                            }
                            if button.clicked() {
                                if shared.ui.selected_tex_set_idx == s as i32 {
                                    shared.ui.rename_id = "tex_set ".to_string() + &s.to_string()
                                }
                                shared.ui.selected_tex_set_idx = s as i32;
                            }
                        }
                    });
                });

                if shared.ui.selected_tex_set_idx == -1 && hovered_set == -1 {
                    return;
                }

                let frame = egui::Frame::default().inner_margin(5.);
                ui.vertical(|ui| {
                    ui.set_width((modal_width / 2.) - 20.);
                    ui.set_height(height);
                    ui.horizontal(|ui| {
                        if hovered_set != -1 {
                            ui.label(shared.loc("texture_modal.set_preview"));
                            return;
                        }
                        ui.label(shared.loc("texture_modal.textures"));
                        if !ui.skf_button(shared.loc("texture_modal.import")).clicked() {
                            return;
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        bone_panel::open_file_dialog(shared.temp_path.img.clone());
                        #[cfg(target_arch = "wasm32")]
                        crate::toggleElement(true, "image-dialog".to_string());
                    });
                    let size = ui.available_size();
                    ui.dnd_drop_zone::<i32, _>(frame, |ui| {
                        ui.set_width(size.x);
                        ui.set_height(size.y - 10.);
                        if hovered_set != -1 {
                            let mut offset = Vec2::new(0., 0.);
                            let mut row_height = 0.;
                            for tex in &shared.armature.texture_sets[hovered_set as usize].textures
                            {
                                let size = resize_tex_img(tex.size, 50);

                                if offset.x + size.x > ui.available_width() {
                                    offset.x = 0.;
                                    offset.y += row_height;
                                    row_height = 0.;
                                }

                                if size.y > row_height {
                                    row_height = size.y;
                                }
                                let rect = egui::Rect::from_min_size(
                                    ui.min_rect().left_top() + offset.into(),
                                    size.into(),
                                );
                                egui::Image::new(tex.ui_img.as_ref().unwrap()).paint_at(ui, rect);
                                offset.x += size.x;
                            }
                            return;
                        }
                        draw_tex_buttons(shared, ui);
                    });
                });
            });
        });
}

pub fn draw_tex_preview(shared: &Shared, ui: &mut egui::Ui) {
    let tex = &shared.armature.texture_sets[shared.ui.selected_tex_set_idx as usize].textures
        [shared.ui.hovering_tex as usize];
    let size = resize_tex_img(tex.size, ui.available_width() as usize);
    let left_top = egui::Pos2::new(
        ui.min_rect().center().x - size.x / 2.,
        ui.min_rect().center().y - size.y / 2. - 40.,
    );
    let rect = egui::Rect::from_min_size(left_top, size.into());
    egui::Image::new(tex.ui_img.as_ref().unwrap()).paint_at(ui, rect);

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        let mut name = egui::text::LayoutJob::default();
        job_text(
            shared.loc("texture_modal.img_name"),
            Some(Color32::WHITE),
            &mut name,
        );
        job_text(&tex.name, None, &mut name);
        let mut size = egui::text::LayoutJob::default();
        job_text(
            shared.loc("texture_modal.img_size"),
            Some(Color32::WHITE),
            &mut size,
        );
        job_text(
            &(tex.size.x.to_string() + " x " + &tex.size.y.to_string()),
            None,
            &mut size,
        );
        ui.label(name);
        ui.label(size);
    });
}

fn resize_tex_img(mut size: Vec2, max: usize) -> Vec2 {
    let mut mult = 1.;
    if size.x > max as f32 {
        mult = max as f32 / size.x;
    }
    size.x *= mult;
    size.y *= mult;

    mult = 1.;
    if size.y > max as f32 {
        mult = max as f32 / size.y
    }
    size.x *= mult;
    size.y *= mult;
    size
}

pub fn draw_tex_buttons(shared: &mut Shared, ui: &mut egui::Ui) {
    let mut idx: i32 = -1;
    macro_rules! set {
        () => {
            shared.armature.texture_sets[shared.ui.selected_tex_set_idx as usize]
        };
    }

    shared.ui.hovering_tex = -1;
    for i in 0..set!().textures.len() {
        idx += 1;
        let name = set!().textures[i].name.clone();
        let button = ui
            .dnd_drag_source(egui::Id::new(("tex", idx, 0)), idx, |ui| {
                let but = egui::Button::new(name);
                ui.add(but);
            })
            .response
            .interact(egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if button.hovered() {
            shared.ui.hovering_tex = i as i32;
        }

        if button.clicked() {
            if shared.ui.has_state(UiState::RemovingTexture) {
                shared.remove_texture(shared.ui.selected_tex_set_idx, i as i32);
                shared.ui.set_state(UiState::RemovingTexture, false);
                // stop the loop to prevent index errors
                break;
            } else {
                let mut anim_id = shared.ui.anim.selected;
                shared.ui.hovering_tex = -1;
                if !shared.ui.is_animating() && shared.selected_bone() != None {
                    anim_id = usize::MAX;
                    shared.undo_actions.push(Action {
                        action: ActionEnum::Bone,
                        id: shared.selected_bone().unwrap().id,
                        bones: vec![shared.selected_bone().unwrap().clone()],
                        ..Default::default()
                    });
                } else if shared.ui.is_animating() && shared.selected_animation() != None {
                    shared.undo_actions.push(Action {
                        action: ActionEnum::Animation,
                        id: shared.selected_animation().unwrap().id,
                        animations: vec![shared.selected_animation().unwrap().clone()],
                        ..Default::default()
                    });
                }

                if shared.selected_bone() == None {
                    return;
                }

                // set texture
                shared.armature.set_bone_tex(
                    shared.selected_bone().unwrap().id,
                    i,
                    shared.ui.selected_tex_set_idx,
                    anim_id,
                    shared.ui.anim.selected_frame,
                );
                shared.ui.set_state(UiState::ImageModal, false);
            }
        }

        let pointer = ui.input(|i| i.pointer.interact_pos());
        let hovered_payload = button.dnd_hover_payload::<i32>();

        let rect = button.rect;

        if pointer == None || hovered_payload == None {
            let stroke =
                egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(125, 125, 125, 255));
            if button.contains_pointer() {
                ui.painter().hline(rect.x_range(), rect.top(), stroke);
                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                ui.painter().vline(rect.right(), rect.y_range(), stroke);
                ui.painter().vline(rect.left(), rect.y_range(), stroke);
            }
            continue;
        }

        let stroke = egui::Stroke::new(1.0, Color32::WHITE);
        let mut is_below = false;

        if pointer.unwrap().y < rect.center().y {
            ui.painter().hline(rect.x_range(), rect.top(), stroke);
        } else {
            ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
            is_below = true;
        };

        let dp = button.dnd_release_payload::<i32>();
        if dp == None {
            continue;
        }
        let dragged_payload = *dp.unwrap() as usize;
        let selected_idx = shared.ui.selected_tex_set_idx as usize;

        let mut old_name_order: Vec<String> = vec![];
        for tex in &shared.armature.texture_sets[selected_idx].textures {
            old_name_order.push(tex.name.clone());
        }

        let new_idx = idx as usize + is_below as usize;
        let tex = shared.armature.texture_sets[selected_idx].textures[dragged_payload].clone();
        shared.armature.texture_sets[selected_idx]
            .textures
            .remove(dragged_payload);
        shared.armature.texture_sets[selected_idx]
            .textures
            .insert(new_idx, tex);

        // disabled:
        // bones would adjust their tex idx after dragging,
        // but auto-adjusting would be confusing when other tex sets
        // are involved
        return;

        #[allow(unreachable_code)]
        for b in 0..shared.armature.bones.len() {
            macro_rules! bone {
                () => {
                    shared.armature.bones[b]
                };
            }

            if bone!().tex_idx == -1 {
                continue;
            }

            let old_name = &old_name_order[bone!().tex_idx as usize];
            bone!().tex_idx = shared.armature.texture_sets[selected_idx]
                .textures
                .iter()
                .position(|tex| tex.name == *old_name)
                .unwrap() as i32;
        }
    }
}

// top-right X label for modals
pub fn modal_x<T: FnOnce()>(ui: &mut egui::Ui, after_close: T) {
    let x_rect = egui::Rect::from_min_size(ui.min_rect().right_top(), egui::Vec2::ZERO);
    if ui
        .put(x_rect, egui::Label::new(egui::RichText::new("X").size(18.)))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        after_close();
    }
}
