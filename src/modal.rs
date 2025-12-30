use crate::{armature_window, ui::EguiUi, utils, Config, PolarId, Shared};

pub fn modal_template<T: FnOnce(&mut egui::Ui), E: FnOnce(&mut egui::Ui)>(
    ctx: &egui::Context,
    id: String,
    config: &Config,
    content: T,
    buttons: E,
) {
    let modal = egui::Modal::new(id.into()).frame(egui::Frame {
        corner_radius: 0.into(),
        fill: config.colors.main.into(),
        inner_margin: egui::Margin::same(5),
        stroke: egui::Stroke::new(1., config.colors.light_accent),
        ..Default::default()
    });
    modal.show(ctx, |ui| {
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
            if ui.skf_button("Yes").clicked() {
                yes = true;
            }

            // Proceeding with kb shortcut will only emulate 'yes' if modal isn't for discarding changes.
            // This is to prevent users with muscle memory from accidentally exiting
            // upon pressing 'enter' upon seeing a modal.
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if shared.ui.polar_id != PolarId::Exiting {
                    yes = true;
                }
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

            let parsed_id = shared.context_id_parsed();
            let bone = &shared.armature.bones[parsed_id as usize];

            let bone_id = bone.id;

            // remove all children of this bone as well
            let mut children = vec![bone.clone()];
            armature_window::get_all_children(&shared.armature.bones, &mut children, &bone);
            children.reverse();
            for bone in &children {
                let idx = shared.armature.bones.iter().position(|b| b.id == bone.id);
                shared.armature.bones.remove(idx.unwrap());
            }

            // remove all references to this bone and it's children from all animations
            let mut set_undo_bone_continued = false;
            for bone in &children {
                for a in 0..shared.armature.animations.len() {
                    let anim = &mut shared.armature.animations[a];
                    let last_len = anim.keyframes.len();

                    // if an animation has this bone, save it in undo data
                    let mut temp_kfs = anim.keyframes.clone();
                    temp_kfs.retain(|kf| kf.bone_id != bone.id);
                    if last_len != temp_kfs.len() && !set_undo_bone_continued {
                        shared.new_undo_anims();
                        shared.undo_actions.last_mut().unwrap().continued = true;
                        set_undo_bone_continued = true;
                    }

                    shared.armature.animations[a].keyframes = temp_kfs;
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
            let context_id = shared.context_id_parsed();
            let bones = &mut shared.armature.bones;
            let targeters = bones.iter_mut().filter(|b| b.ik_target_id == context_id);
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
            let id = shared.context_id_parsed() as usize;
            shared.armature.animations.remove(id);
            shared.ui.context_menu.close();
        }
        PolarId::DeleteFile => {
            std::fs::remove_file(&shared.ui.selected_path).unwrap();
        }
        PolarId::DeleteTex => {
            shared.new_undo_sel_style();
            let id = shared.context_id_parsed() as usize;
            shared.selected_set_mut().unwrap().textures.remove(id);
        }
        PolarId::DeleteStyle => {
            shared.new_undo_styles();
            let context_id = shared.context_id_parsed();
            let styles = &mut shared.armature.styles;
            let idx = styles.iter().position(|s| s.id == context_id).unwrap();
            if shared.ui.selected_style == context_id {
                shared.ui.selected_style = -1;
            }
            styles.remove(idx);
        }
        PolarId::NewUpdate => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let base_url = "https://github.com/Retropaint/SkelForm/releases/tag/v";
                _ = open::that(base_url.to_owned() + &shared.ui.new_version.to_string());
            }
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
    let label = egui::Label::new(egui::RichText::new("X").size(18.));
    let hand = egui::CursorIcon::PointingHand;
    if ui.put(x_rect, label).on_hover_cursor(hand).clicked() {
        after_close();
    }
}
