use crate::{
    bone_panel,
    ui::{EguiUi, TextInputOptions},
    utils, Armature, Config, EventState, InputStates, SelectionState, Ui,
};

pub fn draw(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::shared::Ui,
    config: &Config,
    selections: &SelectionState,
    armature: &Armature,
    events: &mut EventState,
    input: &InputStates,
    camera: &crate::Camera,
) {
    ui.horizontal(|ui| {
        ui.heading(&shared_ui.loc("style_panel.heading"));
    });

    ui.add_space(2.);

    // style name field
    ui.horizontal(|ui| {
        ui.label(&shared_ui.loc("bone_panel.name"));
        let style_name = armature.sel_style(&selections).unwrap().name.clone();
        let (edited, value, _) = ui.text_input("Name".to_string(), shared_ui, style_name, None);
        if edited {
            events.rename_style(0, value);
        }
    });

    ui.add_space(10.);

    // dropdown for new texture types
    ui.horizontal(|ui| {
        ui.label(&shared_ui.loc("style_panel.textures"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
            let combo_box = egui::ComboBox::new("new_tex", "")
                .selected_text(shared_ui.loc("new"))
                .width(40.);
            combo_box.show_ui(ui, |ui| {
                let mut selected = -1;
                ui.selectable_value(&mut selected, 0, shared_ui.loc("style_panel.import"));
                ui.selectable_value(&mut selected, 1, shared_ui.loc("style_panel.empty"));
                if selected == 0 {
                    // import texture (via file dialog)
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        bone_panel::open_file_dialog(&shared_ui.file_paths, &shared_ui.file_type);
                    }
                    #[cfg(target_arch = "wasm32")]
                    crate::clickFileInput(true);
                } else if selected == 1 {
                    events.create_empty_texture();

                    // immediately start renaming empty texture when created
                    let len = armature.sel_style(selections).unwrap().textures.len();
                    shared_ui.rename_id = "texture_".to_owned() + &len.to_string();
                }
            });
        });
    });

    let frame = egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(5.);
    frame.show(ui, |ui| {
        let scroll_area = egui::ScrollArea::vertical().id_salt("tex_list");
        scroll_area.show(ui, |ui| {
            let mut idx: i32 = -1;
            let sel = &selections;
            let mut hovered = false;
            let mut dragged = false;
            for i in 0..armature.sel_style(&sel).unwrap().textures.len() {
                idx += 1;
                let og_name = armature.sel_style(&sel).unwrap().textures[i].name.clone();
                let trimmed_name = utils::trunc_str(ui, &og_name, ui.available_width());
                let context_id = format!("texture_{}", &i.to_string());
                let str_desc = &shared_ui.loc("style_panel.texture_desc").clone();
                let tex = &armature.sel_style(&sel).unwrap().textures[i];

                let mut col = config.colors.dark_accent;
                if i == shared_ui.hovering_tex as usize {
                    col += crate::Color::new(20, 20, 20, 0);
                }
                if selections.tex_ids.contains(&(i as i32)) {
                    col += crate::Color::new(20, 20, 20, 0);
                }

                ui.horizontal(|ui| {
                    // show renaming input field if this texture is being renamed
                    if shared_ui.rename_id == context_id {
                        let input = Some(TextInputOptions {
                            focus: true,
                            placeholder: "Texture".to_string(),
                            default: "Texture".to_string(),
                            ..Default::default()
                        });
                        let (edited, value, _) = ui.text_input(
                            context_id.clone(),
                            shared_ui,
                            og_name.to_string(),
                            input,
                        );
                        if edited {
                            events.rename_texture(i, value);
                        }
                        return;
                    }

                    // render texture button (as draggable element)
                    let bin_width = 13.;
                    let button_id = egui::Id::new(("tex", idx));

                    let drag_id = ui.ctx().dragged_id();
                    let mut opacity = 1.;
                    if drag_id != None && drag_id.unwrap() == button_id && !camera.on_ui {
                        opacity = 0.;
                    }
                    let button = ui
                        .dnd_drag_source(button_id, idx, |ui| {
                            let frame = egui::Frame::new()
                                .fill(col.into())
                                .multiply_with_opacity(opacity);
                            frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.set_width(ui.available_width());
                                    ui.set_height(21.);
                                    ui.add_space(5.);
                                    let text = if opacity == 0. {
                                        egui::RichText::new("")
                                    } else {
                                        egui::RichText::new(&trimmed_name).color(config.colors.text)
                                    };
                                    ui.label(text);
                                });
                            });
                        })
                        .response
                        .on_hover_text(str_desc)
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .interact(egui::Sense::click_and_drag());

                    if button.clicked() {
                        if selections.tex_ids.contains(&(i as i32)) {
                            shared_ui.rename_id = context_id.clone();
                        }
                        if !input.holding_shift && !input.holding_mod {
                            // unselect every other texture if no modifiers are being held
                            for id in &selections.tex_ids {
                                events.toggle_sel_tex(*id, false);
                            }
                        } else if input.holding_shift && selections.tex_ids.len() > 0 {
                            // select all textures between top-most and this one
                            let top = selections.tex_ids[0].min(i as i32);
                            let bottom = selections.tex_ids[0].max(i as i32);
                            for i in 0..(bottom - top) {
                                events.toggle_sel_tex(i as i32 + top, true);
                            }
                        }

                        // select this texture
                        events.toggle_sel_tex(i as i32, true);
                    }
                    if button.secondary_clicked() {
                        shared_ui.context_menu.show(&context_id);

                        // unselect other textures if this wasn't selected prior to being right-clicked
                        if !selections.tex_ids.contains(&(i as i32)) {
                            for id in &selections.tex_ids {
                                events.toggle_sel_tex(*id, false);
                            }
                        }
                        events.toggle_sel_tex(i as i32, true);
                    }
                    if button.contains_pointer() || button.has_focus() {
                        shared_ui.hovering_tex = i as i32;
                        hovered = true;
                    }
                    if button.hovered() && shared_ui.context_menu.id == "" {
                        let area_pos = egui::Pos2::new(
                            shared_ui.bone_panel_rect.unwrap().left() - 175.,
                            shared_ui.bone_panel_rect.unwrap().top(),
                        );
                        let area = egui::Area::new("texture_preview".into())
                            .fixed_pos(area_pos)
                            .order(egui::Order::Foreground);
                        area.show(ui.ctx(), |ui| {
                            let frame = egui::Frame::popup(ui.style());
                            frame.show(ui, |ui| {
                                ui.set_min_size(egui::Vec2::new(150., 200.));
                                ui.set_max_size(egui::Vec2::new(150., 200.));
                                let pos =
                                    egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top());
                                let rect = &egui::Rect::from_min_size(pos, [150., 150.].into());
                                let tex_id = shared_ui.hovering_tex as usize;
                                draw_tex_preview(
                                    tex_id, armature, shared_ui, selections, ui, rect, &config,
                                );
                                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                    let col = Some(config.colors.text.into());
                                    let mut name = egui::text::LayoutJob::default();
                                    let img_name_str = &shared_ui.loc("style_panel.img_name");
                                    crate::ui::job_text(img_name_str, col, &mut name);
                                    crate::ui::job_text(&tex.name, None, &mut name);
                                    let mut size = egui::text::LayoutJob::default();
                                    let img_size_header_str =
                                        &shared_ui.loc("style_panel.img_size");
                                    crate::ui::job_text(img_size_header_str, col, &mut size);
                                    let img_size_str =
                                        &(tex.size.x.to_string() + " x " + &tex.size.y.to_string());
                                    crate::ui::job_text(img_size_str, None, &mut size);
                                    ui.label(name);
                                    ui.label(size);
                                });
                            });
                        });
                    }

                    // set dragging_tex true later, so styles will know that the element being
                    // dropped is a texture (migration)
                    let drag_id = ui.ctx().dragged_id();
                    if drag_id != None && drag_id.unwrap() == button_id {
                        shared_ui.dragging_element = "texture".to_string();
                        shared_ui.dragging_id = idx as u32;
                    }

                    // dragging stuff

                    let rect = button.rect;
                    let pointer = ui.input(|i| i.pointer.interact_pos());
                    let hovered_payload = button.dnd_hover_payload::<i32>();
                    let dragged_payload = button.dnd_release_payload::<i32>();

                    if hovered_payload == None
                        || pointer == None
                        || hovered_payload.unwrap() == idx.into()
                    {
                        return;
                    };

                    let stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                    if pointer.unwrap().y < rect.center().y {
                        ui.painter().hline(rect.x_range(), rect.top(), stroke);
                    } else {
                        //ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
                        //is_below = true;
                    };

                    if let Some(dp) = dragged_payload {
                        if shared_ui.dragging_element != "texture" {
                            return;
                        }
                        // if the dragged texture was dropped on another, move the dragged one above or below
                        let mut old_name_order: Vec<String> = vec![];
                        let sel = selections.clone();
                        for tex in &armature.sel_style(&sel).unwrap().textures {
                            old_name_order.push(tex.name.clone());
                        }
                        events.move_texture(*dp as usize, idx as usize);
                    }
                });
            }

            if !hovered || dragged {
                shared_ui.hovering_tex = -1;
            }
        });
    });
}

pub fn draw_tex_preview(
    tex_id: usize,
    armature: &Armature,
    shared_ui: &mut crate::Ui,
    selections: &crate::SelectionState,
    ui: &mut egui::Ui,
    display: &egui::Rect,
    config: &Config,
) {
    let tex = &armature.sel_style(selections).unwrap().textures[tex_id];
    let size = resize_tex_img(tex.size, display.width().min(display.height()) as usize);
    let left_top = egui::Pos2::new(
        display.left() + (display.width() / 2. - size.x / 2.),
        display.top() + (display.height() / 2. - size.y / 2.),
    );

    if tex.size == crate::Vec2::new(1., 1.) {
        // empty texture
        ui.painter().text(
            left_top,
            egui::Align2::CENTER_CENTER,
            "[Empty texture]",
            egui::FontId::proportional(14.),
            config.colors.text.into(),
        );
    } else {
        // draw texture
        let data = armature.tex_data(tex).unwrap();
        if data.ui_img == None {
            return;
        }
        let texture_rect = egui::Rect::from_min_size(left_top, size.into());
        egui::Image::new(data.ui_img.as_ref().unwrap()).paint_at(ui, texture_rect);
    }
}

pub fn resize_tex_img(mut size: crate::Vec2, max: usize) -> crate::Vec2 {
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
