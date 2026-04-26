//! UI Armature window.

use egui::*;

use crate::{
    shared::Vec2,
    ui::{self, EguiUi, TextInputOptions},
    utils,
};

use crate::shared::*;

const HIGHLIGHT: Color = Color::new(50, 50, 50, 0);

pub fn draw(
    egui_ctx: &Context,
    events: &mut EventState,
    config: &Config,
    armature: &Armature,
    selections: &SelectionState,
    edit_mode: &EditMode,
    shared_ui: &mut crate::Ui,
    camera: &Camera,
) {
    let min_default_size = 175.;
    let panel_id = "Armature";
    let side_panel: egui::SidePanel;
    match config.layout {
        UiLayout::Split | UiLayout::Left => {
            side_panel = egui::SidePanel::left(panel_id)
                .default_width(min_default_size)
                .min_width(min_default_size)
                .max_width(min_default_size + 100.)
                .resizable(true);
        }
        UiLayout::Right => {
            side_panel = egui::SidePanel::right(panel_id)
                .default_width(min_default_size)
                .min_width(min_default_size)
                .max_width(min_default_size + 100.)
                .resizable(true);
        }
    }

    let panel = side_panel.resizable(true).show(egui_ctx, |ui| {
        if shared_ui.startup_window {
            shared_ui.armature_panel_rect = Some(ui.min_rect());
            return;
        }
        let gradient = config.colors.gradient.into();
        ui.gradient(ui.ctx().content_rect(), Color32::TRANSPARENT, gradient);
        ui.horizontal(|ui| {
            ui.heading(&shared_ui.loc("armature_panel.heading"));
        });

        ui.separator();

        ui.horizontal(|ui| {
            let button = ui.skf_button(shared_ui.loc("armature_panel.new_bone_button"));
            if button.clicked() {
                events.new_bone();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if armature.bones.len() == 0 {
                    return;
                }
                let id = format!("styles{}", armature.styles.len().to_string());
                let dropdown = egui::ComboBox::new(id, "")
                    .selected_text(&shared_ui.loc("armature_panel.styles"))
                    .width(80.)
                    .height(500.)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .show_ui(ui, |ui| {
                        for s in 0..armature.styles.len() {
                            ui.set_width(80.);
                            let active = armature.styles[s].active;
                            let tick = if active { " 👁" } else { "" };
                            let mut name = armature.styles[s].name.to_string();
                            name = utils::trunc_str(ui, &name, ui.min_rect().width() - 20.);
                            let label = ui.selectable_value(&mut -1, s as i32, name);
                            #[rustfmt::skip]
                            ui.painter().text(
                                label.rect.right_center(),
                                egui::Align2::RIGHT_CENTER,
                                tick,
                                egui::FontId::default(),
                                config.colors.text.into(),
                            );
                            if label.clicked() {
                                events.toggle_style_active(s, !armature.styles[s].active);
                            }
                        }
                        let label = ui.selectable_value(&mut -1, -2, "[Setup]");
                        if label.clicked() {
                            shared_ui.styles_modal = true;
                            ui.close();
                        }
                    })
                    .response
                    .on_hover_text(&shared_ui.loc("armature_panel.styles_desc"));

                if shared_ui.focus_style_dropdown {
                    dropdown.request_focus();
                    shared_ui.focus_style_dropdown = false;
                }
            });
        });
        ui.add_space(3.);
        let scroll_area = egui::ScrollArea::both().max_height(ui.available_height() - 10.);
        scroll_area.show(ui, |ui| {
            // hierarchy
            let frame = Frame::default().inner_margin(5.);
            ui.dnd_drop_zone::<i32, _>(frame, |ui| {
                ui.set_min_height(ui.available_height());
                ui.set_width(ui.available_width());

                // The empty armature text should have blue hyperlinks to attract the user's
                // attention. The blue makes it clear of being a hyperlink, while also sticking
                // out (without being too jarring).
                ui.style_mut().visuals.hyperlink_color = egui::Color32::from_rgb(94, 156, 255);

                if armature.bones.len() != 0 {
                    #[rustfmt::skip]
                    draw_hierarchy(ui, shared_ui, &selections, &armature, &config, &edit_mode, events);
                } else {
                    ui.add_space(5.);
                    ui::empty_armature_starters(shared_ui, config, ui);
                }
                ui.add_space(4.);
            });
        });
        shared_ui.armature_panel_rect = Some(ui.min_rect());
    });

    ui::draw_resizable_panel(panel_id, panel, events, &egui_ctx, camera);
}

pub fn draw_hierarchy(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    selections: &SelectionState,
    armature: &Armature,
    config: &Config,
    edit_mode: &EditMode,
    events: &mut EventState,
) {
    ui.set_min_width(ui.available_width());
    let mut idx: i32 = -1;
    let mut is_hovering = false;
    let sel = selections.clone();
    let panel = shared_ui.armature_panel_rect;
    let mut def_line_col = config.colors.dark_accent;
    def_line_col += Color::new(20, 20, 20, 0);

    // setup propagated group colors
    let mut group_colors: std::collections::HashMap<i32, Color> = Default::default();
    for bone in &armature.bones {
        group_colors.insert(bone.id, bone.group_color);
        if bone.group_color.a != 0 {
            continue;
        }
        let parent_id = &bone.parent_id;
        let parent = armature.bones.iter().find(|b| b.id == *parent_id);
        if let Some(parent) = parent {
            *group_colors.get_mut(&bone.id).unwrap() = *group_colors.get(&parent.id).unwrap();
        }
    }

    let mut cached_children: std::collections::HashMap<i32, Vec<Bone>> = Default::default();

    for b in 0..armature.bones.len() {
        // stop rendering if bones go below this panel
        if panel != None && ui.cursor().top() > panel.unwrap().bottom() {
            break;
        }

        idx += 1;
        if armature.is_bone_folded(armature.bones[b].id) {
            continue;
        }
        let bone_id = armature.bones[b].id;

        let mut dragged = false;
        let this_group_color = *group_colors.get(&bone_id).unwrap();

        let parents = armature.get_all_parents(false, armature.bones[b].id);
        let sel_bone = armature.sel_bone(&sel);
        let selected_bone_id = if sel_bone != None {
            sel_bone.unwrap().id
        } else {
            -1
        };

        // disable selected bone and it's children from armature if setting IK target,
        // since IK target cannot be itself
        let setting_ik_target = edit_mode.setting_ik_target
            && (bone_id == selected_bone_id
                || parents.iter().find(|bone| bone.id == selected_bone_id) != None);

        ui.add_enabled_ui(!setting_ik_target, |ui| {
            ui.horizontal(|ui| {
                let id = format!("bone_hidden{}", b.to_string());
                let hidden = armature.is_bone_hidden(true, config.propagate_visibility, bone_id);
                let mut col = config.colors.text;
                if hidden {
                    col -= Color::new(80, 80, 80, 0);
                }
                let desc = shared_ui.loc("hidden_desc");
                if bone_label("👁", true, ui, id, Vec2::new(-2., 18.), &desc, col).clicked() {
                    let hidden_f32 = if !hidden { 1. } else { 0. };
                    let sel = selections.anim;
                    let frame = selections.anim_frame;
                    events.save_edited_bone(b);
                    events.edit_bone(bone_id, &AnimElement::Hidden, hidden_f32, "", sel, frame);
                }
                let locked = armature.bones[b].locked;
                let mut col = config.colors.text;
                if !locked {
                    col -= Color::new(80, 80, 80, 0);
                }
                let offset = ui.cursor().min + [16., 3.].into();
                let rect = egui::Rect::from_min_size(offset, [15., 15.].into());
                let img = shared_ui.lock_img.as_ref().unwrap();
                egui::Image::new(img).tint(col).paint_at(ui, rect);
                let response: egui::Response = ui
                    .allocate_rect(rect, egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(shared_ui.loc("locked_desc"));
                if response.hovered() || response.has_focus() {
                    egui::Image::new(img)
                        .tint(col + HIGHLIGHT)
                        .paint_at(ui, rect);
                }
                if response.clicked() {
                    let locked_f32 = if !locked { 1. } else { 0. };
                    events.save_edited_bone(b);
                    let locked = &AnimElement::Locked;
                    events.edit_bone(bone_id, locked, locked_f32, "", usize::MAX, -1);
                }

                // add space to the left if this is a child
                for p in (0..parents.len()).rev() {
                    // don't add vertical line, if there are no more direct children to this parent beyond this bone
                    if let Some(_) = cached_children.get(&parents[p].id) {
                    } else {
                        let id = parents[p].id;
                        cached_children.insert(id, vec![]);
                        get_all_children(
                            &armature.bones,
                            &mut cached_children.get_mut(&parents[p].id).unwrap(),
                            &parents[p],
                        );
                    };
                    let children = cached_children.get(&parents[p].id).unwrap();
                    let this_child_idx = children.iter().position(|b| b.id == bone_id).unwrap();
                    let direct_child_idx =
                        children.iter().rposition(|b| b.parent_id == parents[p].id);
                    if direct_child_idx.unwrap() < this_child_idx {
                        ui.add_space(15.);
                        continue;
                    }

                    // adjust vertical line on first children, so the parent's fold arrow isn't blocked
                    let mut size = None;
                    let mut offset = Vec2::new(0., -12.);
                    if this_child_idx == 0 {
                        size = Some(Vec2::new(2., 20.));
                        offset = Vec2::new(0., -8.);
                    }

                    // get appropriate color for this vertical line, based on the current parent
                    let mut line_col = def_line_col;
                    if group_colors.get(&parents[p].id).unwrap().a != 0 {
                        line_col = *group_colors.get(&parents[p].id).unwrap();
                    }

                    vert_line(offset, size, ui, line_col);
                    ui.add_space(15.);
                }

                // horizontal line connecting to vertical line, for children
                if parents.len() != 0 {
                    let mut parent_col = def_line_col;
                    if parents.len() > 0 && group_colors.get(&parents[0].id).unwrap().a != 0 {
                        parent_col = *group_colors.get(&parents[0].id).unwrap();
                    }
                    hor_line(Vec2::new(-8., 10.), ui, parent_col);
                }

                let mut children = vec![];
                let bone = &armature.bones[b];
                get_all_children(&armature.bones, &mut children, bone);

                // show folding button if this bone has children
                if children.len() == 0 {
                    let mut col = def_line_col;
                    if this_group_color.a != 0 {
                        col = this_group_color;
                    }
                    hor_line(Vec2::new(0., 10.), ui, col);
                } else {
                    // render arrow border
                    let mut border_col = def_line_col;
                    if group_colors.get(&bone.id).unwrap().a != 0 {
                        border_col = *group_colors.get(&bone.id).unwrap();
                    }
                    let border_id = "bone_fold_border".to_string() + &b.to_string();
                    let ball_offset = Vec2::new(-2., 18.);
                    let border = bone_label("⏺", true, ui, border_id, ball_offset, "", border_col);

                    // render folding arrow

                    // change arrow color to contrast better with border
                    let mut arrow_col = config.colors.text;
                    if constrast_between(border_col, arrow_col) < 4.5 {
                        arrow_col = config.colors.dark_accent;
                    }

                    let folded = armature.bones[b].folded;
                    let fold_icon = if folded { "⏵" } else { "⏷" };
                    let id = format!("bone_fold{}", b.to_string());
                    //let desc = shared_ui.loc("armature_panel.fold_desc");
                    let desc = "";
                    let arrow_offset = Vec2::new(-2., 18.5);
                    bone_label(fold_icon, false, ui, id, arrow_offset, &desc, arrow_col);
                    if border.clicked() {
                        events.toggle_bone_folded(idx as usize, !armature.bones[b].folded);
                    }
                }
                ui.add_space(13.);

                let mut selected_col = config.colors.dark_accent;
                let mut cursor = egui::CursorIcon::PointingHand;

                if hidden {
                    selected_col = config.colors.dark_accent;
                }

                if shared_ui.hovering_bone == idx {
                    selected_col += Color::new(20, 20, 20, 0);
                }

                let id = &armature.bones[idx as usize].id;
                let is_multi_selected = selections.bone_ids.contains(id);
                if selections.bone_idx == idx as usize || is_multi_selected {
                    selected_col += Color::new(20, 20, 20, 0);
                    cursor = egui::CursorIcon::Default;
                }

                let width = ui.available_width();
                let context_id = "bone_".to_string() + &idx.to_string();
                if shared_ui.rename_id == context_id {
                    let bone_name = shared_ui.loc("armature_panel.new_bone_name").to_string();
                    let bone = armature.bones[b].name.clone();
                    let options = Some(TextInputOptions {
                        size: Vec2::new(ui.available_width(), 21.),
                        focus: true,
                        placeholder: bone_name.clone(),
                        default: bone_name,
                        ..Default::default()
                    });
                    let (edited, value, _) = ui.text_input(context_id, shared_ui, bone, options);
                    if edited {
                        events.save_bone(idx as usize);
                        events.rename_bone(idx as usize, value);
                    }
                    return;
                }

                let id = Id::new(("bone", idx, 0));
                let button = ui
                    .dnd_drag_source(id, idx, |ui| {
                        ui.set_width(width);
                        let name = armature.bones[b].name.to_string();
                        let mut text_col = config.colors.text;
                        if hidden {
                            text_col = config.colors.dark_accent;
                            text_col += Color::new(40, 40, 40, 0)
                        }

                        // flash bone button if flash timer has been activated
                        let timer = &mut shared_ui.flash_armature_timer;
                        if *timer != None {
                            let dur = 500.;
                            let max_flash = 50.;

                            let flash_percent = dur - timer.unwrap().elapsed().as_millis() as f32;
                            let flash = max_flash * (flash_percent / dur);
                            if flash <= 0. {
                                *timer = None;
                            } else {
                                selected_col +=
                                    Color::new(flash as u8, flash as u8, flash as u8, 0);
                            }
                        }

                        egui::Frame::new().fill(selected_col.into()).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.set_width(width);
                                ui.set_height(21.);
                                ui.add_space(5.);
                                ui.label(egui::RichText::new(name).color(text_col));

                                let mut icon_col = config.colors.dark_accent;
                                icon_col += Color::new(40, 40, 40, 0);

                                let is_target = armature
                                    .bones
                                    .iter()
                                    .find(|b| b.ik_family_id != -1 && b.ik_target_id == bone.id);

                                ui.spacing_mut().item_spacing.x = 5.;
                                ui.style_mut().visuals.override_text_color = Some(icon_col.into());
                                ui.style_mut()
                                    .text_styles
                                    .insert(egui::TextStyle::Body, egui::FontId::monospace(14.0));

                                let mut offset = Vec2::new(0., 18.);
                                let colors = &config.colors;
                                if armature.anim_tex_of(bone.id) != None {
                                    let str = shared_ui
                                        .loc("armature_panel.icons.tex")
                                        .replace("$tex", &bone.tex);
                                    let id = format!("{}tex", b.to_string());
                                    bone_label("🖻", false, ui, id, offset, &str, colors.texture);
                                    offset.x += 18.;
                                } else if bone.tex != "" {
                                    let str = shared_ui
                                        .loc("armature_panel.icons.tex_inactive")
                                        .replace("$tex", &bone.tex);
                                    let id = format!("{}tex_in", b.to_string());
                                    let tex_col = config.colors.texture;
                                    bone_label("🗋", false, ui, id, offset, &str, tex_col);
                                    offset.x += 18.;
                                };
                                if bone.verts_edited {
                                    let str = shared_ui.loc("armature_panel.icons.mesh");
                                    let id = format!("{}mesh", b.to_string());
                                    let off = Vec2::new(offset.x, 19.);
                                    bone_label("⬟", false, ui, id, off, &str, colors.meshdef);
                                    offset.x += 18.;
                                }
                                if bone.ik_family_id != -1 {
                                    let color = config.colors.inverse_kinematics;
                                    let desc = shared_ui
                                        .loc("armature_panel.icons.ik_family")
                                        .replace("$family_id", &bone.ik_family_id.to_string());
                                    let img_offset = ui.cursor().min + [-1., 5.].into();
                                    let rect = egui::Rect::from_min_size(
                                        img_offset + [offset.x, 0.].into(),
                                        [13., 10.].into(),
                                    );
                                    let img = shared_ui.ik_img.as_ref().unwrap();
                                    egui::Image::new(img).tint(color).paint_at(ui, rect);
                                    let response = ui.allocate_rect(rect, egui::Sense::hover());
                                    if response.contains_pointer() {
                                        response.show_tooltip_text(&desc);
                                    }
                                    let family_id = bone.ik_family_id.to_string();
                                    let id = format!("{}ik", b.to_string());
                                    let color = colors.inverse_kinematics;
                                    let id_offset = Vec2::new(offset.x - 18., 18.);
                                    bone_label(&family_id, false, ui, id, id_offset, &desc, color);
                                    offset.x += 18.;
                                }
                                if is_target != None {
                                    let family_id = is_target.unwrap().ik_family_id.to_string();
                                    let icon = format!("⌖{}", family_id);
                                    let desc = shared_ui
                                        .loc("armature_panel.icons.ik_target")
                                        .replace("$family_id", &family_id);
                                    let inc = (20 as u8)
                                        .saturating_mul(is_target.unwrap().ik_family_id as u8);
                                    let mut color = config.colors.ik_target;
                                    color += Color::new(0, inc, inc, 0);
                                    let id = format!("⌖{}", family_id);
                                    bone_label(&icon, false, ui, id, offset, &desc, color);
                                    offset.x += 18.;
                                }
                            });
                        });
                    })
                    .response
                    .interact(Sense::click())
                    .on_hover_cursor(cursor);

                if button.contains_pointer() || button.has_focus() {
                    is_hovering = true;
                    shared_ui.hovering_bone = idx;
                }

                if button.clicked() {
                    events.select_bone(idx as usize, false);
                }
                if button.secondary_clicked() {
                    shared_ui.context_menu.show(&context_id);
                }

                if check_bone_dragging(events, &armature, ui, button, idx as usize) {
                    dragged = true;
                }
            });
        });

        if dragged {
            break;
        }
    }

    if !is_hovering {
        shared_ui.hovering_bone = -1;
    }
}

pub fn bone_label(
    icon: &str,
    interactable: bool,
    ui: &mut egui::Ui,
    id: String,
    offset: Vec2,
    desc: &str,
    color: Color,
) -> egui::Response {
    // draw icon
    let rect = ui.painter().text(
        ui.cursor().min + Vec2::new(offset.x, offset.y).into(),
        egui::Align2::LEFT_BOTTOM,
        icon,
        egui::FontId::default(),
        color.into(),
    );

    // set sense based on whether it should be interactable
    let sense = if interactable {
        egui::Sense::click()
    } else {
        egui::Sense::empty()
    };
    let rect = ui
        .interact(rect, id.into(), sense)
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    // show tooltip if hovered
    if rect.contains_pointer() && desc != "" {
        rect.show_tooltip_text(desc);
    }

    // highlight if hovered/focused
    if interactable && (rect.hovered() || rect.has_focus()) {
        ui.painter().text(
            ui.cursor().min + Vec2::new(offset.x, offset.y).into(),
            egui::Align2::LEFT_BOTTOM,
            icon,
            egui::FontId::default(),
            (color + HIGHLIGHT).into(),
        );
    }
    rect
}

fn check_bone_dragging(
    events: &mut EventState,
    armature: &Armature,
    ui: &mut egui::Ui,
    drag: Response,
    idx: usize,
) -> bool {
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let hovered_payload = drag.dnd_hover_payload::<i32>();
    let rect = drag.rect;
    let stroke = egui::Stroke::new(1.0, Color32::WHITE);

    if pointer == None || hovered_payload == None {
        return false;
    }

    // prevent dragging bone onto itself
    if *hovered_payload.unwrap() == idx as i32 {
        return false;
    }

    let mut is_above = false;

    if pointer.unwrap().y < rect.center().y {
        // above bone (move dragged bone above it)
        ui.painter().hline(rect.x_range(), rect.top(), stroke);
        is_above = true;
    } else {
        // in bone (turn dragged bone to child)
        ui.painter().hline(rect.x_range(), rect.top(), stroke);
        ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
        ui.painter().vline(rect.right(), rect.y_range(), stroke);
        ui.painter().vline(rect.left(), rect.y_range(), stroke);
    };

    let drag_payload = drag.dnd_release_payload::<i32>();
    if drag_payload == None {
        return false;
    };

    events.drag_bone(
        is_above,
        armature.bones[idx].id as usize,
        armature.bones[*drag_payload.unwrap() as usize].id as usize,
    );
    return true;
}

pub fn vert_line(offset: Vec2, mut size: Option<Vec2>, ui: &mut egui::Ui, color: Color) {
    if size == None {
        size = Some(Vec2::new(2., 24.));
    }
    let rect = egui::Rect::from_min_size(
        ui.cursor().left_top() + [3. + offset.x, -1.5 + offset.y].into(),
        size.unwrap().into(),
    );
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, color);
}

pub fn hor_line(offset: Vec2, ui: &mut egui::Ui, color: Color) {
    let rect = egui::Rect::from_min_size(
        ui.cursor().left_top() + [-2. + offset.x, -1.5 + offset.y].into(),
        [13., 2.].into(),
    );
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, color);
}

/// Retrieve all children of this bone (recursive)
pub fn get_all_children(bones: &Vec<Bone>, children_vec: &mut Vec<Bone>, parent: &Bone) {
    let idx = bones.iter().position(|b| b.id == parent.id).unwrap();

    for j in 1..(bones.len() - idx) {
        if bones[idx + j].parent_id != parent.id {
            continue;
        }
        children_vec.push(bones[idx + j].clone());
        get_all_children(bones, children_vec, &bones[idx + j]);
    }
}

fn constrast_between(color1: Color, color2: Color) -> f32 {
    // https://www.w3.org/TR/WCAG20/#contrast-ratiodef
    let lumm_1 = luminance(srgb_to_linear(color1));
    let lumm_2 = luminance(srgb_to_linear(color2));

    let max = lumm_1.max(lumm_2);
    let min = lumm_1.min(lumm_2);

    (max + 0.05) / (min + 0.05)
}

fn srgb_to_linear(color: Color) -> Color {
    let r = (color.r as f32 / 255.).powf(2.2);
    let g = (color.g as f32 / 255.).powf(2.2);
    let b = (color.b as f32 / 255.).powf(2.2);

    Color::new((r * 255.) as u8, (g * 255.) as u8, (b * 255.) as u8, 0)
}

fn luminance(color: Color) -> f32 {
    // https://en.wikipedia.org/wiki/Relative_luminance
    let y = (0.2125, 0.7154, 0.0721);
    (color.r as f32 * 255.) * y.0 + (color.g as f32 * 255.) * y.1 + (color.b as f32 * 255.) * y.2
}
