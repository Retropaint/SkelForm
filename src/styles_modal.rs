use ui::TextInputOptions;

use crate::{ui::EguiUi, *};

pub fn draw(shared: &mut Shared, ctx: &egui::Context) {
    let modal_size = Vec2::new(500., 500.);
    let center = egui::Pos2::new(
        (shared.window.x / shared.ui.scale / 2. - shared.ui.styles_modal_size.x) / 2.,
        (shared.window.y / shared.ui.scale / 2. - shared.ui.styles_modal_size.y) / 2.,
    );
    egui::Modal::new("styles_modal".into())
        // set modal render order so that tex idx dropdown can be rendered above
        .area(
            egui::Area::new("styles_modal_area".into())
                .fixed_pos(center)
                .order(egui::Order::Middle),
        )
        .frame(egui::Frame {
            corner_radius: 0.into(),
            fill: shared.config.colors.main.into(),
            inner_margin: egui::Margin::same(5),
            stroke: egui::Stroke::new(1., shared.config.colors.light_accent),
            ..Default::default()
        })
        .show(ctx, |ui| {
            ui.set_width(modal_size.x);
            ui.set_height(modal_size.y);
            let str_desc = shared.loc("styles_modal.heading_desc");
            let str_heading = shared.loc(&("styles_modal.heading")).to_owned();
            ui.heading(str_heading).on_hover_text(str_desc);

            ui.add_space(5.);

            let frame_padding = 10.;

            let height = ui.available_height();

            ui.horizontal(|ui| {
                ui.set_height(height);
                let modal_width = ui.max_rect().width();
                let height = ui.available_height();
                draw_styles_list(ui, shared, modal_width, height, frame_padding);
                draw_textures_list(ui, shared, modal_width, height, frame_padding);
                draw_bones_list(ui, shared, modal_width, height);
                draw_assigned_list(ui, shared, height);
            });

            modal::modal_x(ui, egui::Vec2::new(-5., 0.), || {
                shared.ui.styles_modal = false;
            });

            shared.ui.styles_modal_size = ui.min_rect().size().into();
        });
}

pub fn draw_styles_list(
    ui: &mut egui::Ui,
    shared: &mut Shared,
    width: f32,
    height: f32,
    padding: f32,
) {
    let smaller = 25.;
    let frame = egui::Frame::default().inner_margin(5.);
    ui.vertical(|ui| {
        ui.set_height(height);
        ui.set_width((width / 3.) - padding - smaller);

        ui.horizontal(|ui| {
            if shared.ui.hovering_tex != -1 {
                ui.label(&shared.loc("styles_modal.texture_preview"));
                return;
            }
            ui.label(&shared.loc("styles_modal.sets"));
            if !ui.skf_button(&shared.loc("new")).clicked() {
                return;
            }
            let ids = shared.armature.styles.iter().map(|set| set.id).collect();
            shared.armature.styles.push(crate::Style {
                id: generate_id(ids),
                name: "".to_string(),
                textures: vec![],
                active: true,
            });
            shared.ui.rename_id =
                "tex_set ".to_string() + &(shared.armature.styles.len() - 1).to_string();
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
            let mut idx = -1;

            for s in 0..shared.armature.styles.len() {
                idx += 1;

                if shared.ui.rename_id == "tex_set ".to_string() + &s.to_string() {
                    let (edited, value, _) = ui.text_input(
                        shared.ui.rename_id.clone(),
                        shared,
                        shared.armature.styles[s].name.clone(),
                        Some(crate::ui::TextInputOptions {
                            size: Vec2::new(ui.available_width(), 20.),
                            focus: true,
                            placeholder: shared.loc("styles_modal.new_style").to_string(),
                            default: shared.loc("styles_modal.new_style").to_string(),
                            ..Default::default()
                        }),
                    );
                    if edited {
                        shared.armature.styles[s].name = value;
                        shared.ui.selected_style = shared.armature.styles[s].id;
                    }
                    continue;
                }

                let mut col = shared.config.colors.dark_accent;
                if shared.armature.styles[s].id == shared.ui.selected_style {
                    col += crate::Color::new(20, 20, 20, 0);
                }
                if s == shared.ui.hovering_set as usize {
                    col += crate::Color::new(20, 20, 20, 0);
                }
                let cursor_icon = if shared.ui.selected_style != shared.armature.styles[s].id {
                    egui::CursorIcon::PointingHand
                } else {
                    egui::CursorIcon::Default
                };
                let width = ui.available_width();
                let checkbox_width = 30.;
                ui.horizontal(|ui| {
                    let button = ui
                        .dnd_drag_source(egui::Id::new(("style", idx, 0)), idx, |ui| {
                            egui::Frame::new().fill(col.into()).show(ui, |ui| {
                                ui.set_width(width - checkbox_width);
                                ui.set_height(21.);
                                let mut name = shared.armature.styles[s].name.clone();
                                name = utils::trunc_str(ui, &name, ui.min_rect().width());
                                ui.label(
                                    egui::RichText::new(name).color(shared.config.colors.text),
                                );
                            });
                        })
                        .response
                        .on_hover_cursor(cursor_icon)
                        .interact(egui::Sense::click());
                    if button.contains_pointer() && !shared.ui.dragging_tex {
                        shared.ui.hovering_set = s as i32;
                        hovered = true;
                    }
                    if button.clicked() {
                        if shared.ui.selected_style == shared.armature.styles[s].id {
                            shared.ui.rename_id = "tex_set ".to_string() + &s.to_string()
                        }
                        shared.ui.selected_style = shared.armature.styles[s].id;
                    }

                    let str_style_active_desc = &shared.loc("styles_modal.active_desc");
                    let visible_checkbox = ui
                        .allocate_rect(
                            egui::Rect::from_min_size(ui.cursor().left_top(), [20., 20.].into()),
                            egui::Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(str_style_active_desc);
                    let mut visible_col = shared.config.colors.text;
                    if visible_checkbox.contains_pointer() {
                        visible_col += Color::new(60, 60, 60, 0);
                    }
                    let visible = if shared.armature.styles[s].active {
                        "👁"
                    } else {
                        "---"
                    };
                    ui.painter().text(
                        visible_checkbox.rect.left_top(),
                        egui::Align2::LEFT_TOP,
                        visible,
                        egui::FontId::new(20., egui::FontFamily::default()),
                        visible_col.into(),
                    );
                    if visible_checkbox.clicked() {
                        shared.armature.styles[s].active = !shared.armature.styles[s].active;
                        for b in 0..shared.armature.bones.len() {
                            shared.armature.set_bone_tex(
                                shared.armature.bones[b].id,
                                shared.armature.bones[b].tex.clone(),
                                shared.ui.anim.selected,
                                shared.ui.anim.selected_frame,
                            );
                        }
                    }

                    let pointer = ui.input(|i| i.pointer.interact_pos());
                    let hovered_payload = button.dnd_hover_payload::<i32>();
                    let dragged_payload = button.dnd_release_payload::<i32>();

                    if pointer == None || hovered_payload == None {
                        return;
                    }

                    let rect = button.rect;
                    let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

                    ui.painter().hline(rect.x_range(), rect.top(), stroke);
                    if shared.ui.dragging_tex {
                        ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                        ui.painter().vline(rect.right(), rect.y_range(), stroke);
                        ui.painter().vline(rect.left(), rect.y_range(), stroke);
                    }

                    if dragged_payload == None {
                        return;
                    }

                    let hov = *hovered_payload.clone().unwrap() as usize;
                    if !shared.ui.dragging_tex {
                        shared.armature.styles.swap(hov, idx as usize);
                    } else if idx != shared.ui.selected_style {
                        let style = &mut shared.armature.styles[shared.ui.selected_style as usize];
                        let tex = style.textures[hov].clone();
                        style.textures.remove(hov);
                        shared.armature.styles[idx as usize].textures.push(tex);
                        shared.ui.dragging_tex = false;
                    }
                });
            }

            if !hovered {
                shared.ui.hovering_set = -1;
            }
        });
    });
}

pub fn draw_textures_list(
    ui: &mut egui::Ui,
    shared: &mut Shared,
    modal_width: f32,
    height: f32,
    padding: f32,
) {
    let frame = egui::Frame::default().inner_margin(5.);
    let mut set_idx: usize = usize::MAX;
    let styles = &shared.armature.styles;
    let tex_id = shared.ui.selected_style;
    if let Some(idx) = styles.iter().position(|set| set.id == tex_id) {
        set_idx = idx;
    }
    let is_selected = set_idx == shared.ui.hovering_set as usize;
    let smaller = 25.;
    ui.vertical(|ui| {
        ui.set_width((modal_width / 3.) - padding - smaller);
        ui.set_height(height);

        ui.horizontal(|ui| {
            if shared.ui.hovering_set != -1 && !is_selected {
                ui.label(&shared.loc("styles_modal.style_preview"));
                return;
            }
            ui.label(&shared.loc("styles_modal.textures"));

            // don't show import button if first created style is still being named
            let naming_first_style =
                shared.armature.styles.len() == 1 && shared.ui.rename_id == "tex_set 0";

            if naming_first_style
                || set_idx == usize::MAX
                || shared.ui.selected_style == -1
                || !ui.skf_button(&shared.loc("styles_modal.import")).clicked()
            {
                return;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let file_name = Arc::clone(&shared.file_name);
                let file_contents = Arc::clone(&shared.img_contents);
                bone_panel::open_file_dialog(file_name, file_contents);
            }
            #[cfg(target_arch = "wasm32")]
            crate::toggleElement(true, "image-dialog".to_string());
        });

        let size = ui.available_size();
        let tex_frame_padding = Vec2::new(10., 15.);

        if shared.ui.selected_style == -1 {
            let mut darker = shared.config.colors.dark_accent;
            darker -= Color::new(5, 5, 5, 0);
            egui::Frame::new()
                .inner_margin(5.)
                .fill(darker.into())
                .show(ui, |ui| {
                    ui.set_width(size.x - tex_frame_padding.x + 5.);
                    ui.set_height(size.y - tex_frame_padding.y + 5.);
                });
            return;
        }

        ui.dnd_drop_zone::<i32, _>(frame, |ui| {
            let mut darker = shared.config.colors.dark_accent;
            if shared.ui.selected_style == -1 {
                darker -= Color::new(5, 5, 5, 0);
            }
            egui::Frame::new()
                .fill(darker.into())
                .inner_margin(2.)
                .show(ui, |ui| {
                    ui.set_width(size.x - tex_frame_padding.x);
                    ui.set_height(size.y - tex_frame_padding.y);

                    if set_idx == usize::MAX {
                        return;
                    }

                    if shared.ui.hovering_set == -1 || is_selected {
                        if shared.ui.selected_style != -1 {
                            egui::ScrollArea::vertical()
                                .id_salt("tex_list")
                                .show(ui, |ui| {
                                    draw_tex_buttons(shared, ui);
                                });
                        }
                        return;
                    }

                    let is_empty = shared.armature.styles[shared.ui.hovering_set as usize]
                        .textures
                        .len()
                        == 0;
                    if is_empty {
                        let str_empty = &shared.loc("styles_modal.style_preview_empty");
                        ui.label(str_empty);
                        return;
                    }

                    let mut offset = Vec2::new(0., 0.);
                    let mut row_height = 0.;
                    for tex in &shared.armature.styles[shared.ui.hovering_set as usize].textures {
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
                        let data = shared.armature.tex_data(tex).unwrap();
                        egui::Image::new(data.ui_img.as_ref().unwrap()).paint_at(ui, rect);
                        offset.x += size.x;
                    }
                });
        });
    });
}

fn draw_bones_list(ui: &mut egui::Ui, shared: &mut Shared, modal_width: f32, height: f32) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let str_heading = &shared.loc("styles_modal.bones");
            ui.label(str_heading.to_owned())
        });

        if shared.ui.selected_style == -1 {
            let size = ui.available_size();
            let mut darker = shared.config.colors.dark_accent;
            darker -= Color::new(5, 5, 5, 0);
            egui::Frame::new()
                .inner_margin(5.)
                .fill(darker.into())
                .show(ui, |ui| {
                    ui.set_width(size.x - 50.);
                    ui.set_height(size.y - 10.);
                });
            return;
        }

        let frame = egui::Frame::default();
        ui.dnd_drop_zone::<i32, _>(frame, |ui| {
            egui::Frame::new()
                .fill(shared.config.colors.dark_accent.into())
                .inner_margin(6.)
                .show(ui, |ui| {
                    let padding = Vec2::new(0., 33.);
                    ui.set_width((modal_width / 3.) - padding.x);
                    ui.set_height(height - padding.y);

                    if shared.ui.selected_style == -1 {
                        return;
                    }

                    let styles = &shared.armature.styles;
                    let tex_id = shared.ui.selected_style;

                    let set = styles.iter().find(|style| style.id == tex_id).unwrap();

                    if set.textures.len() == 0 {
                        return;
                    }

                    let scroll = egui::ScrollArea::both()
                        .vertical_scroll_offset(shared.ui.bones_assigned_scroll)
                        .show(ui, |ui| {
                            draw_bone_buttons(ui, shared);
                        });
                    shared.ui.bones_assigned_scroll = scroll.state.offset.y;
                })
        });
    });
}

pub fn draw_bone_buttons(ui: &mut egui::Ui, shared: &mut Shared) {
    let mut hovered = false;
    let mut idx = -1;
    for b in 0..shared.armature.bones.len() {
        idx += 1;
        if shared.armature.is_bone_folded(shared.armature.bones[b].id) {
            continue;
        }
        ui.horizontal(|ui| {
            let parents = shared.armature.get_all_parents(shared.armature.bones[b].id);
            // add space to the left if this is a child
            for _ in 0..parents.len() {
                armature_window::vert_line(0., ui, shared);
                ui.add_space(15.);
            }

            // show folding button if this bone has children
            let mut children = vec![];
            let bone = &shared.armature.bones[b];
            armature_window::get_all_children(&shared.armature.bones, &mut children, bone);
            if children.len() == 0 {
                armature_window::hor_line(11., ui, shared);
            } else {
                let fold_icon = if shared.armature.bones[b].folded {
                    "⏵"
                } else {
                    "⏷"
                };
                let id = "bone_style_fold".to_owned() + &b.to_string();
                if armature_window::bone_label(fold_icon, ui, id, shared, Vec2::new(-2., 18.))
                    .clicked()
                {
                    shared.armature.bones[b].folded = !shared.armature.bones[b].folded;
                }
            }
            ui.add_space(13.);

            let mut selected_col = shared.config.colors.dark_accent;

            if shared.ui.hovering_style_bone == b as i32 {
                selected_col += crate::Color::new(20, 20, 20, 0);
            }

            let width = ui.available_width();

            let idx_input_width = 15.;

            let name = shared.armature.bones[b].name.to_string();
            let mut text_col = shared.config.colors.text;
            if shared.armature.bones[b].is_hidden == 1 {
                text_col = shared.config.colors.dark_accent;
                text_col += crate::Color::new(40, 40, 40, 0)
            }
            let button = egui::Frame::new()
                .fill(selected_col.into())
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_width((width - idx_input_width).max(0.));
                        ui.set_height(21.);
                        ui.add_space(5.);
                        ui.label(egui::RichText::new(name).color(text_col));
                    });
                })
                .response
                .interact(egui::Sense::click());

            if button.contains_pointer() {
                shared.ui.hovering_style_bone = b as i32;
                hovered = true;
            }

            let pointer = ui.input(|i| i.pointer.interact_pos());
            let hovered_payload = button.dnd_hover_payload::<i32>();
            let dragged_payload = button.dnd_release_payload::<i32>();

            ui.add_space(7.);

            if pointer == None || hovered_payload == None {
                return;
            }

            let rect = button.rect;
            let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);

            ui.painter().hline(rect.x_range(), rect.top(), stroke);
            ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
            ui.painter().vline(rect.right(), rect.y_range(), stroke);
            ui.painter().vline(rect.left(), rect.y_range(), stroke);

            if dragged_payload == None {
                return;
            }

            let bone = &mut shared.armature.bones[idx as usize];
            let id = bone.id;
            let tex_str = shared.armature.styles[shared.ui.selected_style as usize].textures
                [*dragged_payload.unwrap() as usize]
                .name
                .clone();
            shared.armature.set_bone_tex(
                id,
                tex_str,
                shared.ui.anim.selected,
                shared.ui.anim.selected_frame,
            );
        });
    }
    if !hovered {
        shared.ui.hovering_style_bone = -1;
    }
}

fn draw_assigned_list(ui: &mut egui::Ui, shared: &mut Shared, height: f32) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let str_heading = &shared.loc("styles_modal.assigned_textures");
            let str_desc = &shared.loc("styles_modal.assigned_textures_desc");
            ui.label(str_heading.to_owned()).on_hover_text(str_desc)
        });

        let default_width = 150.;

        if shared.ui.selected_style == -1 {
            let size = ui.available_size();
            let mut darker = shared.config.colors.dark_accent;
            darker -= Color::new(5, 5, 5, 0);
            egui::Frame::new()
                .inner_margin(5.)
                .fill(darker.into())
                .show(ui, |ui| {
                    ui.set_width(default_width);
                    ui.set_height(size.y - 10.);
                });
            return;
        }

        let frame = egui::Frame::default();
        ui.dnd_drop_zone::<i32, _>(frame, |ui| {
            egui::Frame::new()
                .fill(shared.config.colors.dark_accent.into())
                .inner_margin(6.)
                .show(ui, |ui| {
                    ui.set_width(default_width);
                    let padding = Vec2::new(20., 33.);
                    ui.set_height(height - padding.y);

                    if shared.ui.selected_style == -1 {
                        return;
                    }

                    let styles = &shared.armature.styles;
                    let tex_id = shared.ui.selected_style;

                    let set = styles.iter().find(|style| style.id == tex_id).unwrap();

                    if set.textures.len() == 0 {
                        return;
                    }

                    let scroll = egui::ScrollArea::vertical()
                        .id_salt("assigned")
                        .vertical_scroll_offset(shared.ui.bones_assigned_scroll)
                        .show(ui, |ui| {
                            for b in 0..shared.armature.bones.len() {
                                if shared.armature.is_bone_folded(shared.armature.bones[b].id) {
                                    continue;
                                }
                                let bone = shared.armature.bones[b].clone();
                                let mut raw_str = bone.tex.to_string();
                                let max_letters = 13;
                                if raw_str.len() > max_letters - 1 {
                                    raw_str = raw_str[0..max_letters.min(raw_str.len())]
                                        .to_string()
                                        + "...";
                                }
                                let len: i32 =
                                    (max_letters as i32 - raw_str.len() as i32 + 5).max(0);
                                raw_str += &" ".repeat(len as usize);
                                let str_idx = egui::RichText::new(raw_str).monospace();
                                ui.add_space(1.5);
                                ui.horizontal(|ui| {
                                    egui::containers::menu::MenuButton::new(str_idx).ui(ui, |ui| {
                                        let style_id = shared.ui.selected_style;
                                        let styles = &shared.armature.styles;
                                        let set = styles.iter().find(|s| s.id == style_id).unwrap();
                                        let mut tex_str = bone.tex.clone();
                                        let none_str = shared.loc("none_option");
                                        ui.selectable_value(&mut tex_str, "".to_string(), none_str);
                                        for t in 0..set.textures.len() {
                                            let name = set.textures[t].name.clone().to_string();
                                            ui.selectable_value(&mut tex_str, name.clone(), name);
                                        }
                                        shared.armature.set_bone_tex(
                                            bone.id,
                                            tex_str,
                                            shared.ui.anim.selected,
                                            shared.ui.anim.selected_frame,
                                        );
                                    });
                                    ui.add_space(7.);
                                });
                                ui.add_space(1.5);
                            }
                        });
                    shared.ui.bones_assigned_scroll = scroll.state.offset.y;
                })
        });
    });
}

pub fn draw_tex_preview(shared: &Shared, ui: &mut egui::Ui) {
    let tex = &shared.selected_set().unwrap().textures[shared.ui.hovering_tex as usize];
    let size = resize_tex_img(tex.size, ui.available_width() as usize);
    let left_top = egui::Pos2::new(
        ui.min_rect().center().x - size.x / 2.,
        ui.min_rect().center().y - size.y / 2. - 40.,
    );
    let rect = egui::Rect::from_min_size(left_top, size.into());
    let data = shared.armature.tex_data(tex).unwrap();
    egui::Image::new(data.ui_img.as_ref().unwrap()).paint_at(ui, rect);

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        let mut name = egui::text::LayoutJob::default();
        ui::job_text(
            &shared.loc("styles_modal.img_name"),
            Some(egui::Color32::WHITE),
            &mut name,
        );
        ui::job_text(&tex.name, None, &mut name);
        let mut size = egui::text::LayoutJob::default();
        ui::job_text(
            &shared.loc("styles_modal.img_size"),
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

    let width = ui.available_width();

    let mut hovered = false;
    let mut dragged = false;
    for i in 0..shared.selected_set().unwrap().textures.len() {
        idx += 1;
        let mut name = shared.selected_set().unwrap().textures[i].name.clone();
        name = utils::trunc_str(ui, &name, width - 10.);

        let str_desc = &shared.loc("styles_modal.texture_desc").clone();

        let mut col = shared.config.colors.dark_accent;
        if i == shared.ui.hovering_tex as usize {
            col += crate::Color::new(20, 20, 20, 0);
        }

        ui.horizontal(|ui| {
            let rename_id = "texture_".to_owned() + &i.to_string();
            if shared.ui.rename_id == rename_id {
                let (edited, value, _) = ui.text_input(
                    rename_id.clone(),
                    shared,
                    name.to_string(),
                    Some(TextInputOptions {
                        focus: true,
                        placeholder: "Texture".to_string(),
                        default: "Texture".to_string(),
                        ..Default::default()
                    }),
                );
                if edited {
                    let og_name = shared.selected_set_mut().unwrap().textures[i].name.clone();
                    let trimmed = value.trim_start().trim_end().to_string();
                    shared.selected_set_mut().unwrap().textures[i].name = trimmed.clone();
                    let style = shared.selected_set().unwrap();
                    let tex_names: Vec<String> =
                        style.textures.iter().map(|t| t.name.clone()).collect();

                    let filter = tex_names.iter().filter(|name| **name == trimmed);
                    if filter.count() > 1 {
                        shared.selected_set_mut().unwrap().textures[i].name = og_name.clone();
                        let same_name_str = shared.loc("styles_modal.same_name");
                        shared.ui.open_modal(same_name_str, false);
                    }

                    if !shared.config.keep_tex_str {
                        for bone in &mut shared.armature.bones {
                            if bone.tex == og_name {
                                bone.tex = trimmed.clone();
                            }
                        }
                    }
                }
                return;
            }
            let bin_width = 13.;
            let button_id = egui::Id::new(("tex", idx));
            let button = ui
                .dnd_drag_source(button_id, idx, |ui| {
                    egui::Frame::new().fill(col.into()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.set_width(width - bin_width);
                            ui.set_height(21.);
                            ui.add_space(5.);
                            ui.label(egui::RichText::new(&name).color(shared.config.colors.text));
                        });
                    });
                })
                .response
                .on_hover_text(str_desc)
                .interact(egui::Sense::click());

            let drag_id = ui.ctx().dragged_id();
            if drag_id != None && drag_id.unwrap() == button_id {
                dragged = true;
            }

            if button.secondary_clicked() {
                shared.ui.context_menu.show(ContextType::Texture, i as i32)
            }
            if button.clicked() {
                if shared.ui.selected_tex == i as i32 {
                    shared.ui.rename_id = rename_id.clone();
                }
                shared.ui.selected_tex = i as i32;
            }

            if shared.ui.context_menu.is(ContextType::Texture, i as i32) {
                button.show_tooltip_ui(|ui| {
                    if ui.clickable_label(shared.loc("rename")).clicked() {
                        shared.ui.rename_id = rename_id;
                        shared.ui.context_menu.close();
                    };
                    if ui.clickable_label(shared.loc("delete")).clicked() {
                        let str_del = &shared.loc("polar.delete_tex").clone();
                        shared.ui.open_polar_modal(PolarId::DeleteTex, &str_del);

                        // only hide the menu, as tex id is still needed for modal
                        shared.ui.context_menu.hide = true;
                    }
                    if ui.ui_contains_pointer() {
                        shared.ui.context_menu.keep = true;
                    }
                });
            }
            if button.contains_pointer() {
                shared.ui.hovering_tex = i as i32;
                hovered = true;
            }

            ui.add_space(5.);

            let rect = button.rect;

            let pointer = ui.input(|i| i.pointer.interact_pos());
            let hovered_payload = button.dnd_hover_payload::<i32>();
            let dragged_payload = button.dnd_release_payload::<i32>();

            if hovered_payload == None || pointer == None || hovered_payload.unwrap() == idx.into()
            {
                return;
            };

            let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
            let mut is_below = false;
            if pointer.unwrap().y < rect.center().y {
                ui.painter().hline(rect.x_range(), rect.top(), stroke);
            } else {
                ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                is_below = true;
            };

            let dp = if let Some(dp) = dragged_payload {
                *dp as usize
            } else {
                return;
            };

            let mut old_name_order: Vec<String> = vec![];
            for tex in &shared.selected_set().unwrap().textures {
                old_name_order.push(tex.name.clone());
            }

            shared.undo_actions.push(Action {
                action: ActionType::TextureSet,
                id: shared.ui.selected_style as i32,
                tex_sets: vec![shared.selected_set().unwrap().clone()],
                ..Default::default()
            });

            let new_idx = idx as usize + is_below as usize;

            let textures = &mut shared.selected_set_mut().unwrap().textures;
            let tex = textures[dp].clone();
            textures.remove(dp);
            textures.insert(new_idx, tex);
        });
    }

    if !hovered || dragged {
        shared.ui.hovering_tex = -1;
    }
    shared.ui.dragging_tex = dragged;
}
