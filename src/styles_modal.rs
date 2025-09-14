use crate::{ui::EguiUi, *};

pub fn draw(shared: &mut Shared, ctx: &egui::Context) {
    egui::Modal::new("test".into())
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: shared.config.colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., shared.config.colors.light_accent),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(450.);
            ui.set_height(400.);
            let str_desc = shared.loc("texture_modal.heading_desc");
            let str_heading = shared.loc(&("texture_modal.heading")).to_owned();
            ui.heading(str_heading + " " + crate::ICON_INFO)
                .on_hover_text(str_desc);
            modal::modal_x(ui, || {
                shared.ui.set_state(UiState::ImageModal, false);
            });

            ui.add_space(5.);

            let frame_padding = 10.;
            let frame_count = 3.;

            let height = ui.available_height();

            ui.horizontal(|ui| {
                ui.set_height(height);
                let frame = egui::Frame::default().inner_margin(5.);
                let modal_width = ui.max_rect().width();
                let height = ui.available_height();
                ui.vertical(|ui| {
                    ui.set_height(height);
                    ui.set_width((modal_width / frame_count) - frame_padding);

                    ui.horizontal(|ui| {
                        if shared.ui.hovering_tex != -1 {
                            ui.label(shared.loc("texture_modal.texture_preview"));
                            return;
                        }
                        ui.label(shared.loc("texture_modal.sets"));
                        if !ui.skf_button(shared.loc("new")).clicked() {
                            return;
                        }
                        shared.armature.texture_sets.push(crate::TextureSet {
                            name: "".to_string(),
                            textures: vec![],
                            active: false,
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

                        let mut hovered = false;

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

                            let mut col = shared.config.colors.dark_accent;
                            if s == shared.ui.selected_tex_set_idx as usize {
                                col += crate::Color::new(20, 20, 20, 0);
                            }
                            if s == shared.ui.hovering_set as usize {
                                col += crate::Color::new(20, 20, 20, 0);
                            }
                            let cursor_icon = if shared.ui.selected_tex_set_idx != s as i32 {
                                egui::CursorIcon::PointingHand
                            } else {
                                egui::CursorIcon::Default
                            };
                            let width = ui.available_width();
                            let button = egui::Frame::new()
                                .fill(col.into())
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.set_width(width);
                                        ui.set_height(21.);
                                        ui.add_space(5.);
                                        ui.label(
                                            egui::RichText::new(set!().name.clone())
                                                .color(shared.config.colors.text),
                                        );
                                    });
                                })
                                .response
                                .interact(egui::Sense::click())
                                .on_hover_cursor(cursor_icon);
                            if button.contains_pointer() {
                                shared.ui.hovering_set = s as i32;
                                hovered = true;
                            }
                            if button.clicked() {
                                if shared.ui.selected_tex_set_idx == s as i32 {
                                    shared.ui.rename_id = "tex_set ".to_string() + &s.to_string()
                                }
                                shared.ui.selected_tex_set_idx = s as i32;
                            }
                        }

                        if !hovered {
                            shared.ui.hovering_set = -1;
                        }
                    });
                });

                let frame = egui::Frame::default().inner_margin(5.);
                let is_selected = shared.ui.selected_tex_set_idx == shared.ui.hovering_set;
                ui.vertical(|ui| {
                    ui.set_width((modal_width / frame_count) - frame_padding);
                    ui.set_height(height);
                    ui.horizontal(|ui| {
                        if shared.ui.hovering_set != -1 && !is_selected {
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

                        if shared.ui.hovering_set == -1 || is_selected {
                            if shared.ui.selected_tex_set_idx != -1 {
                                draw_tex_buttons(shared, ui);
                            }
                        } else {
                            let is_empty = shared.armature.texture_sets
                                [shared.ui.hovering_set as usize]
                                .textures
                                .len()
                                == 0;
                            if is_empty {
                                let str_empty = shared.loc("texture_modal.set_preview_empty");
                                ui.label(str_empty);
                            } else {
                                let mut offset = Vec2::new(0., 0.);
                                let mut row_height = 0.;
                                for tex in &shared.armature.texture_sets
                                    [shared.ui.hovering_set as usize]
                                    .textures
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
                                    egui::Image::new(tex.ui_img.as_ref().unwrap())
                                        .paint_at(ui, rect);
                                    offset.x += size.x;
                                }
                            }
                        }
                    });
                });

                draw_bones_list(ui, shared, modal_width, height);
            });
        });
}

fn draw_bones_list(ui: &mut egui::Ui, shared: &mut Shared, modal_width: f32, height: f32) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| ui.label("Assigned Bones"));
        egui::Frame::new()
            .fill(shared.config.colors.dark_accent.into())
            .inner_margin(6.)
            .show(ui, |ui| {
                ui.set_width((modal_width / 3.) - 10.);
                ui.set_height(height - 33.);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut hovered = false;
                    for b in 0..shared.armature.bones.len() {
                        // if this bone's parent is folded, skip drawing
                        let mut visible = true;
                        let mut nb = &shared.armature.bones[b];
                        while nb.parent_id != -1 {
                            nb = shared.armature.find_bone(nb.parent_id).unwrap();
                            if nb.folded {
                                visible = false;
                                break;
                            }
                        }
                        if !visible {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            let parents =
                                shared.armature.get_all_parents(shared.armature.bones[b].id);
                            // add space to the left if this is a child
                            for _ in 0..parents.len() {
                                armature_window::vert_line(0., ui, shared);
                                ui.add_space(15.);
                            }

                            // show folding button if this bone has children
                            let mut children = vec![];
                            armature_window::get_all_children(
                                &shared.armature.bones,
                                &mut children,
                                &shared.armature.bones[b],
                            );
                            if children.len() == 0 {
                                armature_window::hor_line(11., ui, shared);
                            } else {
                                let fold_icon = if shared.armature.bones[b].folded {
                                    "⏵"
                                } else {
                                    "⏷"
                                };

                                let id = "style_bone_fold".to_owned() + &b.to_string();
                                if armature_window::bone_label(
                                    fold_icon,
                                    ui,
                                    id,
                                    shared,
                                    b,
                                    Vec2::new(-2., 18.),
                                )
                                .clicked()
                                {
                                    shared.armature.bones[b].folded =
                                        !shared.armature.bones[b].folded;
                                }
                            }
                            ui.add_space(13.);

                            let mut selected_col = shared.config.colors.dark_accent;

                            if shared.armature.bones[b]
                                .style_idxs
                                .contains(&shared.ui.selected_tex_set_idx)
                            {
                                selected_col += crate::Color::new(20, 20, 20, 0);
                            }

                            if shared.ui.hovering_style_bone == b as i32 {
                                selected_col += crate::Color::new(20, 20, 20, 0);
                            }

                            let width = ui.available_width();

                            let has_tex = shared.armature.bones[b].tex_set_idx != -1;

                            let id = egui::Id::new(("styles_bone", b, 0));
                            let button = ui
                                .dnd_drag_source(id, b, |ui| {
                                    ui.set_width(width);

                                    let name = shared.armature.bones[b].name.to_string();
                                    let mut text_col = shared.config.colors.text;
                                    if shared.armature.is_bone_hidden(shared.armature.bones[b].id) {
                                        text_col = shared.config.colors.dark_accent;
                                        text_col += crate::Color::new(40, 40, 40, 0)
                                    }
                                    egui::Frame::new().fill(selected_col.into()).show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.set_width(width);
                                            ui.set_height(21.);
                                            ui.add_space(5.);
                                            ui.label(egui::RichText::new(name).color(text_col));

                                            let pic = if has_tex { "🖻  " } else { "" };
                                            let mut pic_col = shared.config.colors.dark_accent;
                                            pic_col += crate::Color::new(40, 40, 40, 0);
                                            ui.label(egui::RichText::new(pic).color(pic_col))
                                        });
                                    });
                                })
                                .response
                                .interact(egui::Sense::click())
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            if button.contains_pointer() {
                                shared.ui.hovering_style_bone = b as i32;
                                hovered = true;
                            }
                            if button.clicked() {
                                let styles = &mut shared.armature.bones[b].style_idxs;
                                if styles.contains(&shared.ui.selected_tex_set_idx) {
                                    styles.retain(|style| *style != shared.ui.selected_tex_set_idx);
                                } else {
                                    styles.push(shared.ui.selected_tex_set_idx);
                                }
                            }
                        });
                    }
                    if !hovered {
                        shared.ui.hovering_style_bone = -1;
                    }
                });
            })
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
        ui::job_text(
            shared.loc("texture_modal.img_name"),
            Some(egui::Color32::WHITE),
            &mut name,
        );
        ui::job_text(&tex.name, None, &mut name);
        let mut size = egui::text::LayoutJob::default();
        ui::job_text(
            shared.loc("texture_modal.img_size"),
            Some(egui::Color32::WHITE),
            &mut size,
        );
        ui::job_text(
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

    let width = ui.available_width();

    let mut hovered = false;
    for i in 0..set!().textures.len() {
        idx += 1;
        let name = set!().textures[i].name.clone();

        let mut col = shared.config.colors.dark_accent;
        if i == shared.ui.hovering_tex as usize {
            col += crate::Color::new(20, 20, 20, 0);
        }
        let button = ui
            .dnd_drag_source(egui::Id::new(("tex", idx, 0)), idx, |ui| {
                egui::Frame::new().fill(col.into()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_width(width);
                        ui.set_height(21.);
                        ui.add_space(5.);
                        ui.label(
                            egui::RichText::new(i.to_string() + ") " + &name)
                                .color(shared.config.colors.text),
                        );
                    });
                });
            })
            .response;

        if button.contains_pointer() {
            shared.ui.hovering_tex = i as i32;
            hovered = true;
        }

        let pointer = ui.input(|i| i.pointer.interact_pos());
        let hovered_payload = button.dnd_hover_payload::<i32>();

        let rect = button.rect;

        if pointer == None || hovered_payload == None {
            continue;
        }

        let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
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

        shared.undo_actions.push(Action {
            action: ActionType::TextureSet,
            id: selected_idx as i32,
            tex_sets: vec![shared.armature.texture_sets[selected_idx].clone()],
            ..Default::default()
        });

        let new_idx = idx as usize + is_below as usize;
        let tex = shared.armature.texture_sets[selected_idx].textures[dragged_payload].clone();
        shared.armature.texture_sets[selected_idx]
            .textures
            .remove(dragged_payload);
        shared.armature.texture_sets[selected_idx]
            .textures
            .insert(new_idx, tex);

        if shared.config.keep_tex_idx_on_move {
            return;
        }

        // adjust bones to use the new texture indices that matched prior
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

    if !hovered {
        shared.ui.hovering_tex = -1;
    }
}
