use crate::{
    armature_window, generate_id,
    ui::{job_text, selection_button, EguiUi},
    utils, Action, ActionEnum, Config, PolarId, Shared, UiState, Vec2,
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
            fill: config.ui_colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., config.ui_colors.light_accent),
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
        }
        PolarId::Exiting => shared.ui.set_state(UiState::Exiting, true),
        PolarId::FirstTime => shared.ui.start_tutorial(&shared.armature),
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
            fill: shared.config.ui_colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., shared.config.ui_colors.light_accent),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(300.);
            ui.set_height(400.);
            ui.heading("Select Texture");
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
                ui.vertical(|ui| {
                    ui.set_height(height);
                    ui.set_width((modal_width / 2.) - 10.);

                    ui.horizontal(|ui| {
                        ui.label("Sets");
                        if !ui.skf_button("New").clicked() {
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
                            let tex = &shared.armature.texture_sets
                                [shared.ui.selected_tex_set_idx as usize]
                                .textures[shared.ui.hovering_tex as usize];
                            let max = ui.available_width();
                            let mut size = tex.size;
                            let mut mult = 1.;
                            if size.x > max {
                                mult = max / size.x;
                            }
                            size.x *= mult;
                            size.y *= mult;

                            mult = 1.;
                            if size.y > max {
                                mult = max / size.y
                            }
                            size.x *= mult;
                            size.y *= mult;
                            let rect =
                                egui::Rect::from_min_size(ui.min_rect().left_top(), size.into());
                            egui::Image::new(tex.ui_img.as_ref().unwrap()).paint_at(ui, rect);
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
                                        size: Vec2::new(60., 20.),
                                        focus: true,
                                        placeholder: "New Set".to_string(),
                                        default: "New Set".to_string(),
                                        ..Default::default()
                                    }),
                                );
                                if edited {
                                    set!().name = value;
                                    shared.ui.selected_tex_set_idx = s as i32;
                                }
                                continue;
                            }

                            let mut col = shared.config.ui_colors.light_accent;
                            if s as i32 == shared.ui.selected_tex_set_idx {
                                col += crate::Color::new(20, 20, 20, 0);
                            }
                            let button =
                                ui.add(egui::Button::new(set!().name.to_string()).fill(col));
                            if button.clicked() {
                                if shared.ui.selected_tex_set_idx == s as i32 {
                                    shared.ui.rename_id = "tex_set ".to_string() + &s.to_string()
                                }
                                shared.ui.selected_tex_set_idx = s as i32;
                            }
                        }
                    });
                });

                if shared.ui.selected_tex_set_idx == -1 {
                    return;
                }

                let frame = egui::Frame::default().inner_margin(5.);
                ui.vertical(|ui| {
                    ui.set_width((modal_width / 2.) - 20.);
                    ui.set_height(height);
                    ui.horizontal(|ui| {
                        ui.label("Textures");
                        if !ui.skf_button("Import").clicked() {
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
                        draw_tex_buttons(shared, ui);
                    });
                });
            });
        });
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
                shared.remove_texture(i as i32);
                shared.ui.set_state(UiState::RemovingTexture, false);
                // stop the loop to prevent index errors
                break;
            } else {
                let mut anim_id = shared.ui.anim.selected;
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

        if pointer == None || hovered_payload == None {
            continue;
        }

        let rect = button.rect;
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

        let mut old_name_order: Vec<String> = vec![];
        for tex in &shared.armature.textures {
            old_name_order.push(tex.name.clone());
        }

        let new_idx = idx as usize + is_below as usize;

        let tex = shared.armature.textures[dragged_payload].clone();
        shared.armature.textures.remove(dragged_payload);
        shared.armature.textures.insert(new_idx, tex);

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
            bone!().tex_idx = shared
                .armature
                .textures
                .iter()
                .position(|tex| tex.name == *old_name)
                .unwrap() as i32;
        }
    }
}

pub fn first_time_modal(shared: &mut Shared, ctx: &egui::Context) {
    modal_template(
        ctx,
        &shared.config,
        |ui| {
            ui.heading("Welcome!");
            modal_x(ui, || {
                shared.ui.set_state(UiState::FirstTimeModal, false);
            });
            ui.label("\nA few resources to get you started:\n");
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("- Check out the ");
                let mut link = ui
                    .label(
                        egui::RichText::new("User Documentation")
                            .color(egui::Color32::from_hex("#659adf").unwrap()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                link.sense = egui::Sense::click();
                if link.clicked() {
                    utils::open_docs(false, "");
                }
            });

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("- Try out the ");

                if !ui.skf_button("sample").clicked() {
                    return;
                }

                #[cfg(target_arch = "wasm32")]
                crate::downloadSample();
                #[cfg(not(target_arch = "wasm32"))]
                crate::file_reader::create_temp_file(
                    &shared.temp_path.import,
                    &(utils::bin_path() + "/samples/skellington.skf"),
                );
                shared.ui.set_state(UiState::FirstTimeModal, false);
            });

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("- Be guided from scratch with a ");
                if ui.skf_button("help light").clicked() {
                    shared.ui.start_tutorial(&shared.armature);
                    shared.ui.set_state(UiState::FirstTimeModal, false);
                }
            });
        },
        |_| {},
    );
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
