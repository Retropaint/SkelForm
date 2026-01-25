use ui::{EguiUi, TextInputOptions};

use crate::*;

pub fn draw(
    ctx: &egui::Context,
    config: &Config,
    selections: &SelectionState,
    armature: &Armature,
    shared_ui: &mut crate::Ui,
    input: &InputStates,
    events: &mut EventState,
) {
    let frame = egui::Frame {
        corner_radius: 0.into(),
        fill: config.colors.main.into(),
        inner_margin: egui::Margin::same(5),
        stroke: egui::Stroke::new(1., config.colors.light_accent),
        ..Default::default()
    };
    let id = "atlas_modal".into();
    egui::Modal::new(id).frame(frame).show(ctx, |ui| {
        let height = 400.;
        ui.set_height(height);
        ui.set_width(475.);
        ui.heading("Importing Texture(s)");
        ui.add_space(10.);
        let sel = &selections;
        let style = &armature.sel_style(sel).unwrap();
        let atlas = style.textures.last().unwrap().clone();
        ui.horizontal(|ui| {
            // draw atlas
            let frame = egui::Frame::new();
            let image = frame.show(ui, |ui| {
                ui.set_width(300.);
                ui.set_height(300.);
                let data = armature.tex_data(&atlas).unwrap();
                let img_size = Vec2::new(data.image.width() as f32, data.image.height() as f32);
                let size = styles_modal::resize_tex_img(img_size, 300);
                let rect = egui::Rect::from_min_size(ui.min_rect().left_top(), size.into());
                egui::Image::new(data.ui_img.as_ref().unwrap()).paint_at(ui, rect);
                for t in 0..shared_ui.pending_textures.len() {
                    let tex = &shared_ui.pending_textures[t];
                    let interp = tex.offset / atlas.size;
                    let size_interp = tex.size / atlas.size;
                    let size: Vec2 = ui.min_rect().size().into();
                    let left_top = ui.min_rect().left_top() + (size * interp).into();
                    let rb = size * size_interp;
                    let right_bottom: egui::Vec2 = [rb.x, rb.y].into();
                    let rect = egui::Rect::from_min_size(left_top, right_bottom);
                    ui.painter().rect_stroke(
                        rect,
                        egui::CornerRadius::ZERO,
                        egui::Stroke::new(1., egui::Color32::GREEN),
                        egui::StrokeKind::Outside,
                    );
                }
            });

            // adding texture by dragging on atlas
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let interp = Vec2::new(
                    (pointer.x - image.response.rect.min.x) / image.response.rect.size().x,
                    (pointer.y - image.response.rect.min.y) / image.response.rect.size().y,
                );

                // drag rects if mouse is on them
                let mut dragging = false;
                for tex in &mut shared_ui.pending_textures {
                    let pixel = atlas.size * interp;
                    if !(pixel.x > tex.offset.x
                        && pixel.y > tex.offset.y
                        && pixel.x < tex.offset.x + tex.size.x
                        && pixel.y < tex.offset.y + tex.size.y)
                        || shared_ui.is_dragging_pending
                    {
                        continue;
                    }

                    shared_ui.cursor_icon = egui::CursorIcon::Grab;
                    if input.left_down {
                        dragging = true;
                        tex.offset += atlas.size * (interp - shared_ui.prev_pending_interp);
                        if tex.offset.x < 0. {
                            tex.offset.x = 0.;
                        }
                        if tex.offset.y < 0. {
                            tex.offset.y = 0.;
                        }

                        if tex.offset.x + tex.size.x > atlas.size.x {
                            tex.offset.x = atlas.size.x - tex.size.x;
                        }
                        if tex.offset.y + tex.size.y > atlas.size.y {
                            tex.offset.y = atlas.size.y - tex.size.y;
                        }
                    }

                    break;
                }

                if !dragging {
                    // if left clicked, initiate new texture
                    if input.left_pressed && image.response.contains_pointer() {
                        shared_ui.init_pending_mouse = pointer.into();
                        shared_ui.pending_textures.push(Texture {
                            offset: (atlas.size * interp).floor(),
                            ..Default::default()
                        });
                    }

                    if input.left_down && image.response.contains_pointer() {
                        shared_ui.is_dragging_pending = true;
                        let tex = &mut shared_ui.pending_textures.last_mut().unwrap();
                        let init_pending = shared_ui.init_pending_mouse;
                        let init_interp = Vec2::new(
                            (init_pending.x - image.response.rect.min.x)
                                / image.response.rect.size().x,
                            (init_pending.y - image.response.rect.min.y)
                                / image.response.rect.size().y,
                        );

                        // dragging (horizontal)
                        if pointer.x < shared_ui.init_pending_mouse.x {
                            tex.offset.x = (atlas.size.x * interp.x).floor().max(0.);
                            tex.size.x = (atlas.size.x * init_interp.x - tex.offset.x).floor();
                        } else {
                            tex.offset.x = (atlas.size.x * init_interp.x).floor();
                            tex.size.x = (atlas.size.x * interp.x - tex.offset.x).floor();
                        }

                        // dragging (vertical)
                        if pointer.y < shared_ui.init_pending_mouse.y {
                            tex.offset.y = (atlas.size.y * interp.y).floor().max(0.);
                            tex.size.y = (atlas.size.y * init_interp.y - tex.offset.y).floor();
                        } else {
                            tex.offset.y = (atlas.size.y * init_interp.y).floor();
                            tex.size.y = (atlas.size.y * interp.y - tex.offset.y).floor();
                        }
                    }
                }

                shared_ui.prev_pending_interp = interp;
            }

            if !input.left_down {
                shared_ui.is_dragging_pending = false;
            }

            ui.add_space(40.);

            ui.vertical(|ui| {
                ui.set_height(height);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                        if ui.skf_button("Add Texture").clicked() {
                            shared_ui.pending_textures.push(Texture::default());
                        };
                    });
                });
                if shared_ui.pending_textures.len() == 0 {
                    ui.label(shared_ui.loc("atlas_modal.no_pending"));
                } else {
                    textures_list(ui, atlas, height, shared_ui);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                        if ui.skf_button("Done").clicked() {
                            for (t, tex) in shared_ui.pending_textures.iter_mut().enumerate() {
                                if tex.name == "" {
                                    tex.name = "Texture ".to_owned() + &t.to_string();
                                }
                            }
                            shared_ui.done_pending = shared_ui.pending_textures.len() > 0;
                            shared_ui.atlas_modal = false;
                        }
                        if ui.skf_button("Cancel").clicked() {
                            shared_ui.pending_textures = vec![];
                            shared_ui.atlas_modal = false;
                            events.cancel_pending_texture();
                        }
                    });
                });
            })
        });
    });
}

fn textures_list(ui: &mut egui::Ui, atlas: Texture, height: f32, shared_ui: &mut crate::Ui) {
    let h = height - 45.;
    egui::ScrollArea::vertical().max_height(h).show(ui, |ui| {
        for t in 0..shared_ui.pending_textures.len() {
            let mut tex = shared_ui.pending_textures[t].clone();
            macro_rules! input {
                ($id:expr, $field:expr, $ui:expr) => {
                    let (edited, value, _) = $ui.float_input($id, shared_ui, $field, 1., None);
                    if edited {
                        $field = value;
                    }
                };
            }

            // loop will have to break if tex is removed, to prevent OoB
            let mut removed = false;

            // name input
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.add_space(10.);
                    let trash = ui
                        .label("🗑")
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    if trash.clicked() {
                        shared_ui.pending_textures.remove(t);
                        removed = true;
                        return;
                    }
                    let (edited, value, _) = ui.text_input(
                        t.to_string() + "name",
                        shared_ui,
                        tex.name.clone(),
                        Some(TextInputOptions {
                            placeholder: "Texture Name...".to_string(),
                            ..Default::default()
                        }),
                    );
                    if edited {
                        tex.name = value;
                    }
                });
            });

            if removed {
                break;
            }

            // offset inputs
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("L:").monospace());
                input!(t.to_string() + "texox", tex.offset.x, ui);
                ui.label(egui::RichText::new("T:").monospace());
                input!(t.to_string() + "texoy", tex.offset.y, ui);
                tex.offset.x = tex.offset.x.min(atlas.size.x);
                tex.offset.y = tex.offset.y.min(atlas.size.y);
            });

            // size inputs
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("R:").monospace());
                input!(t.to_string() + "texsx", tex.size.x, ui);
                ui.label(egui::RichText::new("B:").monospace());
                input!(t.to_string() + "texsy", tex.size.y, ui);
                tex.size.x = tex.size.x.min(atlas.size.x - tex.offset.x);
                tex.size.y = tex.size.y.min(atlas.size.y - tex.offset.y);
            });
            ui.add_space(10.);
            shared_ui.pending_textures[t] = tex;
        }
    });
}
