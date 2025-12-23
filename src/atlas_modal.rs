use ui::{EguiUi, TextInputOptions};

use crate::*;

pub fn draw(shared: &mut Shared, ctx: &egui::Context) {
    let frame = egui::Frame {
        corner_radius: 0.into(),
        fill: shared.config.colors.main.into(),
        inner_margin: egui::Margin::same(5),
        stroke: egui::Stroke::new(1., shared.config.colors.light_accent),
        ..Default::default()
    };
    let id = "atlas_modal".into();
    egui::Modal::new(id).frame(frame).show(ctx, |ui| {
        let height = 400.;
        ui.set_height(height);
        ui.set_width(475.);
        ui.heading("Importing Texture(s)");
        ui.add_space(10.);
        let style = &shared.armature.styles[shared.ui.selected_style as usize];
        let atlas = style.textures.last().unwrap().clone();
        ui.horizontal(|ui| {
            // draw atlas
            let frame = egui::Frame::new();
            let image = frame.show(ui, |ui| {
                ui.set_width(300.);
                ui.set_height(300.);
                let data = shared.armature.tex_data(&atlas).unwrap();
                egui::Image::new(data.ui_img.as_ref().unwrap())
                    .uv(egui::Rect::from_min_size([0., 0.].into(), [1., 1.].into()))
                    .paint_at(ui, ui.min_rect());
                for t in 0..shared.ui.pending_textures.len() {
                    let tex = &shared.ui.pending_textures[t];
                    let interp = tex.offset / atlas.size;
                    let size_interp = tex.size / atlas.size;
                    let size: Vec2 = ui.min_rect().size().into();
                    let left_top = ui.min_rect().left_top() + (size * interp).into();
                    let right_bot = size * size_interp;
                    let rb: egui::Vec2 = [right_bot.x, right_bot.y].into();
                    let rect = egui::Rect::from_min_size(left_top, rb);
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

                // if left clicked, initiate new texture
                if shared.input.left_pressed && image.response.contains_pointer() {
                    shared.ui.init_pending_mouse = pointer.into();
                    shared.ui.pending_textures.push(Texture {
                        offset: (atlas.size * interp).floor(),
                        ..Default::default()
                    });
                }

                if shared.input.left_down && shared.ui.pending_textures.len() > 0 {
                    let tex = &mut shared.ui.pending_textures.last_mut().unwrap();
                    let init_pending = shared.ui.init_pending_mouse;
                    let init_interp = Vec2::new(
                        (init_pending.x - image.response.rect.min.x) / image.response.rect.size().x,
                        (init_pending.y - image.response.rect.min.y) / image.response.rect.size().y,
                    );

                    // dragging (horizontal)
                    if pointer.x < shared.ui.init_pending_mouse.x {
                        tex.offset.x = (atlas.size.x * interp.x).floor().max(0.);
                        tex.size.x = (atlas.size.x * init_interp.x - tex.offset.x).floor();
                    } else {
                        tex.offset.x = (atlas.size.x * init_interp.x).floor();
                        tex.size.x = (atlas.size.x * interp.x - tex.offset.x).floor();
                    }

                    // dragging (vertical)
                    if pointer.y < shared.ui.init_pending_mouse.y {
                        tex.offset.y = (atlas.size.y * interp.y).floor().max(0.);
                        tex.size.y = (atlas.size.y * init_interp.y - tex.offset.y).floor();
                    } else {
                        tex.offset.y = (atlas.size.y * init_interp.y).floor();
                        tex.size.y = (atlas.size.y * interp.y - tex.offset.y).floor();
                    }
                }
            }

            ui.add_space(40.);

            ui.vertical(|ui| {
                ui.set_height(height);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                        if ui.skf_button("Add Texture").clicked() {
                            shared.ui.pending_textures.push(Texture::default());
                        };
                    });
                });
                if shared.ui.pending_textures.len() == 0 {
                    ui.label(shared.loc("atlas_modal.no_pending"));
                } else {
                    textures_list(shared, ui, atlas, height);
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                        if ui.skf_button("Done").clicked() {
                            shared.ui.done_pending = shared.ui.pending_textures.len() > 0;
                            shared.ui.atlas_modal = false;
                        }
                    });
                });
            })
        });
    });
}

fn textures_list(shared: &mut Shared, ui: &mut egui::Ui, atlas: Texture, height: f32) {
    let h = height - 45.;
    egui::ScrollArea::vertical().max_height(h).show(ui, |ui| {
        for t in 0..shared.ui.pending_textures.len() {
            let mut tex = shared.ui.pending_textures[t].clone();
            macro_rules! input {
                ($id:expr, $field:expr, $ui:expr) => {
                    let (edited, value, _) = $ui.float_input($id, shared, $field, 1., None);
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
                        shared.ui.pending_textures.remove(t);
                        removed = true;
                        return;
                    }
                    let (edited, value, _) = ui.text_input(
                        t.to_string() + "name",
                        shared,
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
            shared.ui.pending_textures[t] = tex;
        }
    });
}
