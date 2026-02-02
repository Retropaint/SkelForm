use ui::TextInputOptions;

use crate::{ui::EguiUi, *};

pub fn draw(
    ctx: &egui::Context,
    shared_ui: &mut crate::Ui,
    config: &Config,
    camera: &Camera,
    armature: &Armature,
    selections: &SelectionState,
    events: &mut EventState,
) {
    let modal_size = Vec2::new(500., 500.);
    let frame = egui::Frame {
        corner_radius: 0.into(),
        fill: config.colors.main.into(),
        inner_margin: egui::Margin::same(5),
        stroke: egui::Stroke::new(1., config.colors.light_accent),
        ..Default::default()
    };

    let modal;
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    {
        let center = egui::Pos2::new(
            (camera.window.x / shared_ui.scale - shared_ui.styles_modal_size.x) / 2.,
            (camera.window.y / shared_ui.scale - shared_ui.styles_modal_size.y) / 2.,
        );
        modal = egui::Modal::new("styles_modal".into())
            // set modal render order so that tex idx dropdown can be rendered above
            .area(
                egui::Area::new("styles_modal_area".into())
                    .fixed_pos(center)
                    .order(egui::Order::Middle),
            )
            .frame(frame);
    }
    #[cfg(all(not(target_os = "macos"), not(target_arch = "wasm32")))]
    {
        modal = egui::Modal::new("styles_modal".into()).frame(frame);
    }

    modal.show(ctx, |ui| {
        ui.set_width(modal_size.x);
        ui.set_height(modal_size.y);
        let str_desc = shared_ui.loc("styles_modal.heading_desc");
        let str_heading = shared_ui.loc(&("styles_modal.heading")).to_owned();
        ui.heading(str_heading).on_hover_text(str_desc);

        ui.add_space(5.);

        let frame_padding = 10.;

        let height = ui.available_height();

        ui.horizontal(|ui| {
            ui.set_height(height);
            let modal_width = ui.max_rect().width();
            let height = ui.available_height();
            #[rustfmt::skip]
            draw_styles_list(ui, armature, shared_ui, config, selections, events, modal_width, height, frame_padding);
            draw_textures_list(ui, modal_width, height, frame_padding, shared_ui, events, armature, selections, config);
            draw_bones_list(ui, modal_width, height, armature, config, shared_ui, selections, events);
            draw_assigned_list(ui, height, shared_ui, config, selections, armature, events);
        });

        modal::modal_x(ui, egui::Vec2::new(-5., 0.), || {
            shared_ui.styles_modal = false;
        });

        shared_ui.styles_modal_size = ui.min_rect().size().into();
    });
}

pub fn draw_styles_list(
    ui: &mut egui::Ui,
    armature: &Armature,
    shared_ui: &mut crate::Ui,
    config: &crate::Config,
    selections: &crate::SelectionState,
    events: &mut crate::EventState,
    width: f32,
    height: f32,
    padding: f32,
) {
    let smaller = 25.;
    ui.vertical(|ui| {
        ui.set_height(height);
        ui.set_width((width / 3.) - padding - smaller);

        ui.horizontal(|ui| {
            if shared_ui.hovering_tex != -1 {
                ui.label(&shared_ui.loc("styles_modal.texture_preview"));
                return;
            }
            ui.label(&shared_ui.loc("styles_modal.sets"));
            if !ui.skf_button(&shared_ui.loc("new")).clicked() {
                return;
            }
            events.new_style();
        });

        let size = ui.available_size();
        let frame = egui::Frame::default().inner_margin(5.);
        ui.dnd_drop_zone::<i32, _>(frame, |ui| {
            ui.set_width(size.x);
            ui.set_height(size.y - 10.);

            let mut hovered = false;
            let mut idx = -1;

            if shared_ui.hovering_tex != -1 {
                draw_tex_preview(armature, shared_ui, selections, ui);
                return;
            }

            if armature.styles.len() == 0 {
                let mut cache = egui_commonmark::CommonMarkCache::default();
                let loc = shared_ui.loc("styles_modal.styles_empty").to_string();
                let str = utils::markdown(loc.to_string(), shared_ui.local_doc_url.to_string());
                egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
            }

            for s in 0..armature.styles.len() {
                idx += 1;
                let context_id = "style_".to_owned() + &s.to_string();

                if shared_ui.rename_id == context_id {
                    let (edited, value, _) = ui.text_input(
                        context_id,
                        shared_ui,
                        armature.styles[s].name.clone(),
                        Some(crate::ui::TextInputOptions {
                            size: Vec2::new(ui.available_width(), 20.),
                            focus: true,
                            placeholder: shared_ui.loc("styles_modal.new_style").to_string(),
                            default: shared_ui.loc("styles_modal.new_style").to_string(),
                            ..Default::default()
                        }),
                    );
                    if edited {
                        events.select_style(armature.styles[s].id as usize);
                        events.rename_style(armature.styles[s].id as usize, value);
                    }
                    continue;
                }

                let mut col = config.colors.dark_accent;
                if armature.styles[s].id == selections.style {
                    col += crate::Color::new(20, 20, 20, 0);
                }
                if s == shared_ui.hovering_set as usize {
                    col += crate::Color::new(20, 20, 20, 0);
                }
                let cursor_icon = if selections.style != armature.styles[s].id {
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
                                let mut name = armature.styles[s].name.clone();
                                name = utils::trunc_str(ui, &name, ui.min_rect().width());
                                ui.label(egui::RichText::new(name).color(config.colors.text));
                            });
                        })
                        .response
                        .on_hover_cursor(cursor_icon)
                        .interact(egui::Sense::click());
                    if (button.contains_pointer() || button.has_focus()) && !shared_ui.dragging_tex
                    {
                        shared_ui.hovering_set = s as i32;
                        hovered = true;
                    }

                    if button.clicked() {
                        shared_ui.rename_id = "".to_string();
                        if selections.style == armature.styles[s].id {
                            shared_ui.rename_id = context_id.clone();
                        }
                        events.select_style(armature.styles[s].id as usize);
                    }

                    context_menu!(button, shared_ui, context_id, |ui: &mut egui::Ui| {
                        ui.context_rename(shared_ui, config, context_id);
                        let str = "delete_style";
                        ui.context_delete(shared_ui, config, events, str, PolarId::DeleteStyle);
                    });

                    let str_style_active_desc = &shared_ui.loc("styles_modal.active_desc");
                    let visible_checkbox = ui
                        .allocate_rect(
                            egui::Rect::from_min_size(ui.cursor().left_top(), [20., 20.].into()),
                            egui::Sense::click(),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(str_style_active_desc);
                    let mut visible_col = config.colors.text;
                    if visible_checkbox.contains_pointer() || visible_checkbox.has_focus() {
                        visible_col += Color::new(60, 60, 60, 0);
                    }
                    let visible = if armature.styles[s].active {
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
                        events.toggle_style_active(s, false);
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
                    if shared_ui.dragging_tex {
                        ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                        ui.painter().vline(rect.right(), rect.y_range(), stroke);
                        ui.painter().vline(rect.left(), rect.y_range(), stroke);
                    }

                    if dragged_payload == None {
                        return;
                    }

                    let hov = *hovered_payload.clone().unwrap() as usize;
                    if !shared_ui.dragging_tex {
                        events.move_style(hov as usize, idx as usize);
                    } else if armature.styles[s].id != selections.style {
                        events.migrate_texture(hov, idx as usize);
                    }
                });
            }

            if !hovered {
                shared_ui.hovering_set = -1;
            }
        });
    });
}

pub fn draw_textures_list(
    ui: &mut egui::Ui,
    modal_width: f32,
    height: f32,
    padding: f32,
    shared_ui: &mut crate::Ui,
    events: &mut EventState,
    armature: &Armature,
    selections: &SelectionState,
    config: &Config,
) {
    let frame = egui::Frame::default().inner_margin(5.);
    let mut set_idx: usize = usize::MAX;
    let styles = &armature.styles;
    if let Some(idx) = styles.iter().position(|set| set.id == selections.style) {
        set_idx = idx;
    }
    let is_selected = set_idx == shared_ui.hovering_set as usize;
    let smaller = 25.;
    ui.vertical(|ui| {
        ui.set_width((modal_width / 3.) - padding - smaller);
        ui.set_height(height);

        ui.horizontal(|ui| {
            if shared_ui.hovering_set != -1 && !is_selected {
                ui.label(&shared_ui.loc("styles_modal.style_preview"));
                return;
            }
            ui.label(&shared_ui.loc("styles_modal.textures"));

            // don't show import button if first created style is still being named
            let naming_first_style =
                armature.styles.len() == 1 && shared_ui.rename_id == "tex_set 0";

            if naming_first_style
                || set_idx == usize::MAX
                || selections.style == -1
                || !ui
                    .skf_button(&shared_ui.loc("styles_modal.import"))
                    .clicked()
            {
                return;
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                bone_panel::open_file_dialog(&shared_ui.file_path, &shared_ui.file_type);
            }
            #[cfg(target_arch = "wasm32")]
            crate::clickFileInput(true);
        });

        let size = ui.available_size();
        let tex_frame_padding = Vec2::new(10., 15.);

        if selections.style == -1 {
            let mut darker = config.colors.dark_accent;
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
            let mut darker = config.colors.dark_accent;
            if selections.style == -1 {
                darker -= Color::new(5, 5, 5, 0);
            }
            let frame = egui::Frame::new().fill(darker.into()).inner_margin(2.);
            frame.show(ui, |ui| {
                ui.set_width(size.x - tex_frame_padding.x);
                ui.set_height(size.y - tex_frame_padding.y);

                if set_idx == usize::MAX {
                    return;
                }

                let style = &armature.sel_style(selections).unwrap();
                if style.textures.len() == 0 {
                    let mut cache = egui_commonmark::CommonMarkCache::default();
                    let loc = shared_ui.loc("styles_modal.textures_empty").to_string();
                    let str = utils::markdown(loc, shared_ui.local_doc_url.to_string());
                    egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
                    return;
                }

                if shared_ui.hovering_set == -1 || is_selected {
                    if selections.style != -1 {
                        egui::ScrollArea::vertical()
                            .id_salt("tex_list")
                            .show(ui, |ui| {
                                draw_tex_buttons(
                                    shared_ui,
                                    &armature,
                                    &selections,
                                    &config,
                                    events,
                                    ui,
                                );
                            });
                    }
                    return;
                }

                let is_empty = armature.styles[shared_ui.hovering_set as usize]
                    .textures
                    .len()
                    == 0;
                if is_empty {
                    let str_empty = &shared_ui.loc("styles_modal.style_preview_empty");
                    ui.label(str_empty);
                    return;
                }

                let mut offset = Vec2::new(0., 0.);
                let mut row_height = 0.;
                for tex in &armature.styles[shared_ui.hovering_set as usize].textures {
                    let data = armature.tex_data(tex).unwrap();

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
                    egui::Image::new(data.ui_img.as_ref().unwrap()).paint_at(ui, rect);
                    offset.x += size.x;
                }
            });
        });
    });
}

fn draw_bones_list(
    ui: &mut egui::Ui,
    modal_width: f32,
    height: f32,
    armature: &Armature,
    config: &Config,
    shared_ui: &mut crate::Ui,
    selections: &SelectionState,
    events: &mut EventState,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let str_heading = &shared_ui.loc("styles_modal.bones");
            ui.label(str_heading.to_owned())
        });

        if selections.style == -1 {
            let size = ui.available_size();
            let mut darker = config.colors.dark_accent;
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
            let frame = egui::Frame::new()
                .fill(config.colors.dark_accent.into())
                .inner_margin(6.);
            frame.show(ui, |ui| {
                let padding = Vec2::new(0., 33.);
                ui.set_width((modal_width / 3.) - padding.x);
                ui.set_height(height - padding.y);

                if selections.style == -1 {
                    return;
                }

                let styles = &armature.styles;
                let tex_id = selections.style;

                let set = styles.iter().find(|style| style.id == tex_id).unwrap();

                if set.textures.len() == 0 {
                    return;
                }

                let scroll = egui::ScrollArea::both()
                    .vertical_scroll_offset(shared_ui.bones_assigned_scroll)
                    .show(ui, |ui| {
                        draw_bone_buttons(ui, armature, &config, shared_ui, &selections, events);
                    });
                shared_ui.bones_assigned_scroll = scroll.state.offset.y;
            })
        });
    });
}

pub fn draw_bone_buttons(
    ui: &mut egui::Ui,
    armature: &Armature,
    config: &crate::Config,
    shared_ui: &mut crate::Ui,
    selections: &crate::SelectionState,
    events: &mut EventState,
) {
    let mut hovered = false;
    let mut idx = -1;
    for b in 0..armature.bones.len() {
        idx += 1;
        if armature.is_bone_folded(armature.bones[b].id) {
            continue;
        }
        ui.horizontal(|ui| {
            let parents = armature.get_all_parents(armature.bones[b].id);
            // add space to the left if this is a child
            for _ in 0..parents.len() {
                armature_window::vert_line(0., ui, &config);
                ui.add_space(15.);
            }

            // show folding button if this bone has children
            let mut children = vec![];
            let bone = &armature.bones[b];
            armature_window::get_all_children(&armature.bones, &mut children, bone);
            if children.len() == 0 {
                armature_window::hor_line(11., ui, &config);
            } else {
                let fold_icon = if armature.bones[b].folded {
                    "⏵"
                } else {
                    "⏷"
                };
                let id = "bone_style_fold".to_owned() + &b.to_string();
                if armature_window::bone_label(fold_icon, ui, id, Vec2::new(-2., 18.), config)
                    .clicked()
                {
                    events.toggle_bone_folded(b, !armature.bones[b].folded);
                }
            }
            ui.add_space(13.);

            let mut selected_col = config.colors.dark_accent;

            if shared_ui.hovering_style_bone == b as i32 {
                selected_col += crate::Color::new(20, 20, 20, 0);
            }

            let width = ui.available_width();

            let idx_input_width = 15.;

            let name = armature.bones[b].name.to_string();
            let mut text_col = config.colors.text;
            if armature.bones[b].is_hidden {
                text_col = config.colors.dark_accent;
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

            if button.contains_pointer() || button.has_focus() {
                shared_ui.hovering_style_bone = b as i32;
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

            let bone = &armature.bones[idx as usize];
            let id = bone.id;
            let style = &armature.sel_style(selections).unwrap();
            let tex_str = style.textures[*dragged_payload.unwrap() as usize]
                .name
                .clone();
            events.set_bone_texture(id as usize, tex_str);
        });
    }
    if !hovered {
        shared_ui.hovering_style_bone = -1;
    }
}

fn draw_assigned_list(
    ui: &mut egui::Ui,
    height: f32,
    shared_ui: &mut crate::Ui,
    config: &Config,
    selections: &SelectionState,
    armature: &Armature,
    events: &mut EventState,
) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let str_heading = &shared_ui.loc("styles_modal.assigned_textures");
            let str_desc = &shared_ui.loc("styles_modal.assigned_textures_desc");
            ui.label(str_heading.to_owned()).on_hover_text(str_desc)
        });

        let default_width = 150.;

        if selections.style == -1 {
            let size = ui.available_size();
            let mut darker = config.colors.dark_accent;
            darker -= Color::new(5, 5, 5, 0);
            let frame = egui::Frame::new().inner_margin(5.).fill(darker.into());
            frame.show(ui, |ui| {
                ui.set_width(default_width);
                ui.set_height(size.y - 10.);
            });
            return;
        }

        let frame = egui::Frame::default();
        ui.dnd_drop_zone::<i32, _>(frame, |ui| {
            let frame = egui::Frame::new()
                .fill(config.colors.dark_accent.into())
                .inner_margin(6.);
            frame.show(ui, |ui| {
                ui.set_width(default_width);
                let padding = Vec2::new(20., 33.);
                ui.set_height(height - padding.y);

                if selections.style == -1 {
                    return;
                }

                let styles = &armature.styles;
                let tex_id = selections.style;

                let set = styles.iter().find(|style| style.id == tex_id).unwrap();

                if set.textures.len() == 0 {
                    let mut cache = egui_commonmark::CommonMarkCache::default();
                    let loc = shared_ui.loc("styles_modal.assigned_empty").to_string();
                    let str = utils::markdown(loc, shared_ui.local_doc_url.to_string());
                    egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
                    return;
                }

                let scroll_area = egui::ScrollArea::vertical()
                    .id_salt("assigned")
                    .vertical_scroll_offset(shared_ui.bones_assigned_scroll);
                let scroll = scroll_area.show(ui, |ui| {
                    for b in 0..armature.bones.len() {
                        if armature.is_bone_folded(armature.bones[b].id) {
                            continue;
                        }
                        let bone = armature.bones[b].clone();
                        let mut raw_str = bone.tex.to_string();
                        let max_letters = 13;
                        if raw_str.len() > max_letters - 1 {
                            raw_str =
                                raw_str[0..max_letters.min(raw_str.len())].to_string() + "...";
                        }
                        let len: i32 = (max_letters as i32 - raw_str.len() as i32 + 5).max(0);
                        raw_str += &" ".repeat(len as usize);
                        let str_idx = egui::RichText::new(raw_str).monospace();
                        ui.add_space(1.5);
                        ui.horizontal(|ui| {
                            egui::containers::menu::MenuButton::new(str_idx).ui(ui, |ui| {
                                let style_id = selections.style;
                                let styles = &armature.styles;
                                let set = styles.iter().find(|s| s.id == style_id).unwrap();
                                let mut tex_str = bone.tex.clone();
                                let last_tex = tex_str.clone();
                                let none_str = shared_ui.loc("none_option");
                                ui.selectable_value(&mut tex_str, "".to_string(), none_str);
                                for t in 0..set.textures.len() {
                                    let name = set.textures[t].name.clone().to_string();
                                    ui.selectable_value(&mut tex_str, name.clone(), name);
                                }
                                if last_tex != tex_str {
                                    //shared.undo_states.new_undo_bones(&shared.armature.bones);
                                }
                                events.set_bone_texture(bone.id as usize, tex_str.clone());
                            });
                            ui.add_space(7.);
                        });
                        ui.add_space(1.5);
                    }
                });
                shared_ui.bones_assigned_scroll = scroll.state.offset.y;
            })
        });
    });
}

pub fn draw_tex_preview(
    armature: &Armature,
    shared_ui: &mut crate::Ui,
    selections: &crate::SelectionState,
    ui: &mut egui::Ui,
) {
    let tex = &armature.sel_style(selections).unwrap().textures[shared_ui.hovering_tex as usize];
    let size = resize_tex_img(tex.size, ui.available_width() as usize);
    let left_top = egui::Pos2::new(
        ui.min_rect().center().x - size.x / 2.,
        ui.min_rect().center().y - size.y / 2. - 40.,
    );
    let rect = egui::Rect::from_min_size(left_top, size.into());
    let data = armature.tex_data(tex).unwrap();
    egui::Image::new(data.ui_img.as_ref().unwrap()).paint_at(ui, rect);

    ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
        let mut name = egui::text::LayoutJob::default();
        let img_name_str = &shared_ui.loc("styles_modal.img_name");
        ui::job_text(img_name_str, Some(egui::Color32::WHITE), &mut name);
        ui::job_text(&tex.name, None, &mut name);
        let mut size = egui::text::LayoutJob::default();
        let img_size_str = &shared_ui.loc("styles_modal.img_size");
        ui::job_text(img_size_str, Some(egui::Color32::WHITE), &mut size);
        ui::job_text(
            &(tex.size.x.to_string() + " x " + &tex.size.y.to_string()),
            None,
            &mut size,
        );
        ui.label(name);
        ui.label(size);
    });
}

pub fn resize_tex_img(mut size: Vec2, max: usize) -> Vec2 {
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

pub fn draw_tex_buttons(
    shared_ui: &mut crate::Ui,
    armature: &Armature,
    selections: &crate::SelectionState,
    config: &crate::Config,
    events: &mut crate::EventState,
    ui: &mut egui::Ui,
) {
    let mut idx: i32 = -1;

    let width = ui.available_width();

    let mut hovered = false;
    let mut dragged = false;
    let sel = selections.clone();
    for i in 0..armature.sel_style(&sel).unwrap().textures.len() {
        idx += 1;
        let og_name = armature.sel_style(&sel).unwrap().textures[i].name.clone();
        let trimmed_name = utils::trunc_str(ui, &og_name, width - 10.);
        let context_id = "tex_".to_owned() + &i.to_string();

        let str_desc = &shared_ui.loc("styles_modal.texture_desc").clone();

        let mut col = config.colors.dark_accent;
        if i == shared_ui.hovering_tex as usize {
            col += crate::Color::new(20, 20, 20, 0);
        }

        ui.horizontal(|ui| {
            if shared_ui.rename_id == context_id {
                let (edited, value, _) = ui.text_input(
                    context_id.clone(),
                    shared_ui,
                    og_name.to_string(),
                    Some(TextInputOptions {
                        focus: true,
                        placeholder: "Texture".to_string(),
                        default: "Texture".to_string(),
                        ..Default::default()
                    }),
                );
                if edited {
                    events.rename_texture(i, value);
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
                            ui.label(egui::RichText::new(&trimmed_name).color(config.colors.text));
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

            if button.clicked() {
                if shared_ui.selected_tex == i as i32 {
                    shared_ui.rename_id = context_id.clone();
                }
                shared_ui.selected_tex = i as i32;
            }

            context_menu!(button, shared_ui, context_id, |ui: &mut egui::Ui| {
                ui.context_rename(shared_ui, &config, context_id);
                ui.context_delete(shared_ui, &config, events, "delete_tex", PolarId::DeleteTex);
            });

            if button.contains_pointer() || button.has_focus() {
                shared_ui.hovering_tex = i as i32;
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
            let sel = selections.clone();
            for tex in &armature.sel_style(&sel).unwrap().textures {
                old_name_order.push(tex.name.clone());
            }

            events.move_texture(dp as usize, idx as usize + is_below as usize);
        });
    }

    if !hovered || dragged {
        shared_ui.hovering_tex = -1;
    }
    shared_ui.dragging_tex = dragged;
}
