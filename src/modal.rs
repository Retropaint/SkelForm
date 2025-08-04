use crate::{
    armature_window, bone_panel,
    ui::{self, button, job_text, selection_button},
    utils, Action, ActionEnum, Config, PolarId, Shared, UiState, Vec2,
};

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
            if button("No", ui).clicked() || pressed_no {
                shared.ui.set_state(UiState::PolarModal, false);
            }
            if button("Yes", ui).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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
            let selected_id = shared.selected_bone().unwrap().id;
            let bone = shared
                .armature
                .find_bone(shared.ui.context_menu.id)
                .unwrap();
            // remove all children of this bone as well
            let mut children = vec![bone.clone()];
            armature_window::get_all_children(&shared.armature.bones, &mut children, &bone);
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
            if let Some(bone) = shared.armature.find_bone_idx(selected_id) {
                shared.ui.selected_bone_idx = bone;
            } else {
                shared.ui.selected_bone_idx = usize::MAX;
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
            ui.set_width(250.);
            ui.set_height(250.);
            ui.heading("Select Image");

            modal_x(ui, || {
                shared.ui.set_state(UiState::ImageModal, false);
            });

            ui.horizontal(|ui| {
                if selection_button("Import", shared.ui.has_state(UiState::RemovingTexture), ui)
                    .clicked()
                {
                    #[cfg(not(target_arch = "wasm32"))]
                    bone_panel::open_file_dialog(shared.temp_path.img.clone());

                    #[cfg(target_arch = "wasm32")]
                    crate::toggleElement(true, "image-dialog".to_string());
                }

                let label = if shared.ui.has_state(UiState::RemovingTexture) {
                    "Pick"
                } else {
                    "Remove"
                };
                if button(label, ui).clicked() {
                    shared.ui.set_state(
                        UiState::RemovingTexture,
                        shared.ui.has_state(UiState::RemovingTexture),
                    );
                }
            });

            let mut offset = Vec2::new(0., 0.);
            let mut height = 0.;
            let mut tex_idx = -1;
            let max_width = 250.;
            for i in 0..shared.ui.texture_images.len() {
                // limit size
                let mut size = shared.armature.textures[i].size;
                let max = 50.;
                let mut mult = 1.;
                if size.x > max {
                    mult = max / size.x
                }
                if size.y > max {
                    mult = max / size.y
                }
                size.x *= mult;
                size.y *= mult;

                let padding = 2.;
                // go to next row if there's no space
                if size.x + offset.x + padding > max_width {
                    offset.x = 0.;
                    offset.y += height + 2.;
                    height = 0.;
                }

                // record tallest texture of this row
                if height < size.y {
                    height = size.y;
                }

                let pos = egui::pos2(
                    ui.min_rect().left() + offset.x,
                    ui.min_rect().top() + 50. + offset.y,
                );

                let rect = egui::Rect::from_min_size(pos, size.into());
                let response: egui::Response = ui
                    .allocate_rect(rect, egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                // draw image
                ui.painter().rect_filled(
                    rect,
                    egui::CornerRadius::ZERO,
                    shared.config.ui_colors.dark_accent,
                );

                if response.hovered() {
                    tex_idx = i as i32;
                    ui.painter_at(ui.min_rect()).rect_filled(
                        rect,
                        egui::CornerRadius::ZERO,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 60),
                    );
                }

                egui::Image::new(&shared.ui.texture_images[i]).paint_at(ui, rect);

                if response.clicked() {
                    if shared.ui.has_state(UiState::RemovingTexture) {
                        shared.remove_texture(i as i32);
                        shared.ui.set_state(UiState::RemovingTexture, false);
                        // stop the loop to prevent index errors
                        break;
                    } else {
                        let mut anim_id = shared.ui.anim.selected;
                        if !shared.ui.is_animating() {
                            anim_id = usize::MAX;
                            shared.undo_actions.push(Action {
                                action: ActionEnum::Bone,
                                id: shared.selected_bone().unwrap().id,
                                bones: vec![shared.selected_bone().unwrap().clone()],
                                ..Default::default()
                            });
                        } else if shared.ui.is_animating() {
                            shared.undo_actions.push(Action {
                                action: ActionEnum::Animation,
                                id: shared.selected_animation().unwrap().id,
                                animations: vec![shared.selected_animation().unwrap().clone()],
                                ..Default::default()
                            });
                        }

                        // set texture
                        shared.armature.set_bone_tex(
                            shared.selected_bone().unwrap().id,
                            i,
                            anim_id,
                            shared.ui.anim.selected_frame,
                        );
                        shared.ui.set_state(UiState::ImageModal, false);
                    }
                }

                offset.x += size.x + padding;
            }

            ui.add_space(50.);

            let labels = 2;
            let label_heights = (20 * labels) as f32;
            let label_gaps = (2 * labels) as f32;
            ui.add_space(ui.available_height() - label_heights + label_gaps);

            if tex_idx == -1 {
                return;
            }

            // show image info at bottom left of modal

            let tex = &shared.armature.textures[tex_idx as usize];

            let mut name = egui::text::LayoutJob::default();
            job_text("Name: ", Some(Color32::WHITE), &mut name);
            job_text(&tex.name, None, &mut name);
            let mut size = egui::text::LayoutJob::default();
            job_text("Size: ", Some(Color32::WHITE), &mut size);
            job_text(
                &(tex.size.x.to_string() + " x " + &tex.size.y.to_string()),
                None,
                &mut size,
            );
            ui.label(name);
            ui.label(size);
        });
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
                if ui::button("sample", ui).clicked() {
                    #[cfg(target_arch = "wasm32")]
                    crate::downloadSample();
                    #[cfg(not(target_arch = "wasm32"))]
                    crate::file_reader::create_temp_file(
                        &shared.temp_path.import,
                        &(utils::bin_path() + "/samples/skellington.skf"),
                    );
                    shared.ui.set_state(UiState::FirstTimeModal, false);
                }
            });

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("- Be guided from scratch with a ");
                if ui::button("help light", ui).clicked() {
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
