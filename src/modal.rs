use crate::{armature_window, ui::EguiUi, utils, Action, ActionType, Config, PolarId, Shared};

pub fn modal_template<T: FnOnce(&mut egui::Ui), E: FnOnce(&mut egui::Ui)>(
    ctx: &egui::Context,
    id: String,
    config: &Config,
    content: T,
    buttons: E,
) {
    egui::Modal::new(id.into())
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
        "polar".to_string(),
        &shared.config,
        |ui| {
            ui.label(headline);
        },
        |ui| {
            let pressed_no = ui.input_mut(|i| i.consume_shortcut(&shared.config.keys.cancel));
            if ui.skf_button("No").clicked() || pressed_no {
                shared.ui.polar_modal = false;
            }
            if ui.skf_button("Yes").clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                yes = true;
            }
        },
    );

    if !yes {
        return;
    }

    shared.ui.polar_modal = false;
    match shared.ui.polar_id {
        PolarId::DeleteBone => {
            shared.new_undo_bones();

            shared.ui.selected_bone_idx = usize::MAX;

            let id = shared.ui.context_menu.id;
            let bone = shared.armature.bones.iter().find(|b| b.id == id);
            if bone == None {
                return;
            }

            let bone_id = bone.unwrap().id;

            // remove all children of this bone as well
            let mut children = vec![bone.unwrap().clone()];
            armature_window::get_all_children(&shared.armature.bones, &mut children, bone.unwrap());
            children.reverse();
            for bone in &children {
                let idx = shared.armature.bones.iter().position(|b| b.id == bone.id);
                shared.armature.bones.remove(idx.unwrap());
            }

            // remove all references to this bone and it's children from all animations
            for bone in &children {
                for anim in &mut shared.armature.animations {
                    anim.keyframes.retain(|kf| kf.bone_id != bone.id);
                }
            }

            // remove this bone from binds
            for bone in &mut shared.armature.bones {
                for b in 0..bone.binds.len() {
                    if bone.binds[b].bone_id == bone_id {
                        bone.binds.remove(b);
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
        PolarId::Exiting => {
            if shared.config.ignore_donate {
                shared.ui.confirmed_exit = true;
            } else {
                shared.ui.donating_modal = true;
            }
        }
        PolarId::DeleteAnim => {
            shared.ui.anim.selected = usize::MAX;
            shared.new_undo_anims();
            let id = shared.ui.context_menu.id as usize;
            shared.armature.animations.remove(id);
            shared.ui.context_menu.close();
        }
        PolarId::DeleteFile => {
            std::fs::remove_file(&shared.ui.selected_path).unwrap();
        }
        PolarId::DeleteTex => {
            let id = shared.ui.context_menu.id as usize;
            shared.selected_set_mut().unwrap().textures.remove(id);
        }
    }
}

pub fn modal(shared: &mut Shared, ctx: &egui::Context) {
    let headline = shared.ui.headline.to_string();
    modal_template(
        ctx,
        "modal".to_string(),
        &shared.config,
        |ui| {
            let mut cache = egui_commonmark::CommonMarkCache::default();
            let str = utils::markdown(headline, shared.local_doc_url.to_string());
            egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
        },
        |ui| {
            if shared.ui.forced_modal || !ui.button("OK").clicked() {
                return;
            }

            shared.ui.modal = false;
            shared.ui.headline = "".to_string();
        },
    )
}

pub fn donating_modal(shared: &mut Shared, ctx: &egui::Context) {
    let headline = shared.loc("donating");
    let config = shared.config.clone();
    modal_template(
        ctx,
        "donate".to_string(),
        &config,
        |ui| {
            let mut cache = egui_commonmark::CommonMarkCache::default();
            let str = utils::markdown(headline, "".to_string());
            egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
        },
        |ui| {
            let mut pressed = false;
            if ui.skf_button("Donate").clicked() {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    _ = open::that("https://ko-fi.com/retropaintt");
                }

                #[cfg(target_arch = "wasm32")]
                {
                    crate::openLink("https://ko-fi.com/retropaintt".to_string());
                }

                // wait a second before closing
                // oddly specific but it's for those with 'selector' default
                // browsers like browserosaurus
                std::thread::sleep(std::time::Duration::from_secs(1));

                pressed = true;
            }
            if ui.skf_button("Later").clicked() {
                pressed = true;
            }
            if ui.skf_button("Never").clicked() {
                shared.config.ignore_donate = true;
                utils::save_config(&shared.config);
                pressed = true;
            }

            if !pressed {
                return;
            }

            shared.ui.modal = false;
            shared.ui.confirmed_exit = true;
            shared.ui.headline = "".to_string();
        },
    )
}

// top-right X label for modals
pub fn modal_x<T: FnOnce()>(ui: &mut egui::Ui, offset: egui::Vec2, after_close: T) {
    let x_rect = egui::Rect::from_min_size(ui.min_rect().right_top() + offset, egui::Vec2::ZERO);
    if ui
        .put(x_rect, egui::Label::new(egui::RichText::new("X").size(18.)))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        after_close();
    }
}
