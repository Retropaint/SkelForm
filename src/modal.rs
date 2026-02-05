use crate::{ui::EguiUi, utils, Config, PolarId};

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

pub fn polar_modal(
    ctx: &egui::Context,
    config: &Config,
    shared_ui: &mut crate::Ui,
    events: &mut crate::EventState,
) {
    let headline = shared_ui.headline.to_string();
    modal_template(
        ctx,
        "polar".to_string(),
        &config,
        |ui| _ = ui.label(headline),
        |ui| {
            let mut yes = false;

            // Proceeding with kb shortcut will only emulate 'yes' if modal isn't for discarding changes.
            // This is to prevent users with muscle memory from accidentally exiting
            // upon pressing 'enter' upon seeing a modal.
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if shared_ui.polar_id != PolarId::Exiting {
                    shared_ui.polar_modal = false;
                    yes = true;
                }
            }

            let pressed_no = ui.input_mut(|i| i.consume_shortcut(&config.keys.cancel));
            if ui.skf_button("No").clicked() || pressed_no {
                shared_ui.polar_modal = false;
            }
            if ui.skf_button("Yes").clicked() {
                shared_ui.polar_modal = false;
                yes = true
            }

            if !yes {
                return;
            }

            match shared_ui.polar_id {
                PolarId::DeleteBone => {
                    println!("{}", shared_ui.context_menu.id);
                    events.delete_bone(shared_ui.context_id_parsed() as usize);
                }
                PolarId::Exiting => {
                    if config.ignore_donate {
                        shared_ui.confirmed_exit = true;
                    } else {
                        shared_ui.donating_modal = true;
                    }
                }
                PolarId::DeleteAnim => {
                    events.delete_anim(shared_ui.context_id_parsed() as usize);
                    shared_ui.context_menu.close();
                }
                PolarId::DeleteFile => {
                    std::fs::remove_file(&shared_ui.selected_path).unwrap();
                }
                PolarId::DeleteTex => {
                    events.delete_tex(shared_ui.context_id_parsed() as usize);
                }
                PolarId::DeleteStyle => {
                    events.delete_style(shared_ui.context_id_parsed() as usize);
                }
                PolarId::NewUpdate => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let base_url = "https://github.com/Retropaint/SkelForm/releases/tag/v";
                        _ = open::that(base_url.to_owned() + &shared_ui.new_version.to_string());
                    }
                }
                PolarId::OpenCrashlog => {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        _ = open::that(utils::crashlog_file());
                    }
                }
                PolarId::DeleteKeyframeLine => {
                    events.delete_keyframe_line(
                        shared_ui.anim.deleting_line_bone_id as usize,
                        &shared_ui.anim.deleting_line_element,
                    );
                }
            }
        },
    );
}

pub fn modal(ctx: &egui::Context, shared_ui: &mut crate::Ui, config: &Config) {
    let headline = shared_ui.headline.to_string();
    modal_template(
        ctx,
        "modal".to_string(),
        &config,
        |ui| {
            let mut cache = egui_commonmark::CommonMarkCache::default();
            let str = utils::markdown(headline, shared_ui.local_doc_url.to_string());
            egui_commonmark::CommonMarkViewer::new().show(ui, &mut cache, &str);
        },
        |ui| {
            if shared_ui.forced_modal || !ui.button("OK").clicked() {
                return;
            }

            shared_ui.modal = false;
            shared_ui.headline = "".to_string();
        },
    )
}

pub fn donating_modal(ctx: &egui::Context, shared_ui: &mut crate::Ui, config: &Config) {
    let headline = shared_ui.loc("donating");
    let config = config.clone();
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
                shared_ui.never_donate = true;
                pressed = true;
            }

            if !pressed {
                return;
            }

            shared_ui.modal = false;
            shared_ui.confirmed_exit = true;
            shared_ui.headline = "".to_string();
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
