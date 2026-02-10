use egui::IntoAtoms;

use crate::{
    modal::modal_x, ui::EguiUi, utils, Armature, Camera, Config, EditMode, EventState,
    ExportImgFormat, ExportState, SelectionState, SettingsState, Vec2,
};

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

pub fn draw(
    ctx: &egui::Context,
    shared_ui: &mut crate::Ui,
    edit_mode: &EditMode,
    config: &Config,
    events: &mut EventState,
    armature: &Armature,
    camera: &Camera,
    selections: &SelectionState,
) {
    egui::Modal::new("export_modal".into()).show(ctx, |ui| {
        ui.set_width(400.);
        ui.set_height(350.);

        ui.horizontal(|ui| {
            let col: egui::Color32 = config.colors.dark_accent.into();
            let frame = egui::Frame::new()
                .fill(col)
                .inner_margin(egui::Margin::same(5));
            frame.show(ui, |ui| {
                ui.set_width(100.);
                ui.set_height(400.);
                let width = ui.min_rect().width();
                ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                    let mut is_hovered = false;

                    macro_rules! tab {
                        ($name:expr, $state:expr) => {
                            crate::settings_modal::settings_button(
                                $name,
                                $state,
                                ui,
                                shared_ui,
                                &config,
                                width,
                                &mut is_hovered,
                            )
                        };
                    }

                    let str_armature = shared_ui.loc("export_modal.armature").clone();
                    let str_spritesheet = shared_ui.loc("export_modal.spritesheet").clone();
                    tab!(str_armature, crate::SettingsState::Ui);
                    tab!(str_spritesheet, crate::SettingsState::Animation);

                    if !is_hovered {
                        shared_ui.hovering_setting = None;
                    }
                });
            });

            egui::Frame::new().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let layout = egui::Layout::top_down(egui::Align::Min);
                    ui.with_layout(layout, |ui| match shared_ui.settings_state {
                        SettingsState::Ui => armature_export(
                            ui, shared_ui, edit_mode, events, config, armature, camera, selections,
                        ),
                        SettingsState::Animation => spritesheet_export(
                            ui, shared_ui, edit_mode, events, config, armature, camera, selections,
                        ),
                        SettingsState::Keyboard => spritesheet_export(
                            ui, shared_ui, edit_mode, events, config, armature, camera, selections,
                        ),
                        _ => {}
                    });
                });
            })
        });
    });
}

pub fn armature_export(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    edit_mode: &EditMode,
    events: &mut EventState,
    config: &Config,
    armature: &Armature,
    camera: &Camera,
    selections: &SelectionState,
) {
    ui.heading("Export Armature");
    ui.add_space(10.);
    let width = ui.available_width();
    egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5))
        .show(ui, |ui| {
            ui.set_width(width);
            let text =
                egui::RichText::new(shared_ui.loc("export_modal.inverse_kinematics")).size(15.);
            ui.label(text);
        });

    ui.add_space(2.);

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.bake_ik"))
            .on_hover_text(shared_ui.loc("export_modal.bake_ik_desc"));
        let mut bake_ik = edit_mode.export_bake_ik;
        ui.checkbox(&mut bake_ik, "".into_atoms());
        if bake_ik != edit_mode.export_bake_ik {
            events.toggle_baking_ik(if bake_ik { 1 } else { 0 });
        }
    });

    ui.add_enabled_ui(edit_mode.export_bake_ik, |ui| {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("export_modal.exclude_ik"))
                .on_hover_text(shared_ui.loc("export_modal.exclude_ik_desc"));
            let mut exclude_ik = edit_mode.export_exclude_ik;
            ui.checkbox(&mut exclude_ik, "".into_atoms());
            if exclude_ik != edit_mode.export_exclude_ik {
                events.toggle_exclude_ik(if exclude_ik { 1 } else { 0 });
            }
        });
    });

    ui.add_space(30.);

    egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5))
        .show(ui, |ui| {
            ui.set_width(width);
            let text = egui::RichText::new(shared_ui.loc("export_modal.tex_atlas")).size(15.);
            ui.label(text);
        });
    ui.add_space(5.);

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.img_format"));
        let dropdown = egui::ComboBox::new("img_format", "")
            .selected_text(&edit_mode.export_img_format.to_string())
            .width(80.);
        dropdown.show_ui(ui, |ui| {
            let mut selected = edit_mode.export_img_format.clone();
            ui.selectable_value(&mut selected, ExportImgFormat::PNG, "PNG");
            ui.selectable_value(&mut selected, ExportImgFormat::JPG, "JPG");
            if selected != edit_mode.export_img_format {
                events.set_export_img_format(selected as usize);
            }
        });
    });

    ui.add_enabled_ui(edit_mode.export_img_format == ExportImgFormat::JPG, |ui| {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("export_modal.clear_color"))
                .on_hover_text(shared_ui.loc("export_modal.clear_color_desc"));
            let cc = &edit_mode.export_clear_color;
            let mut col: [f32; 3] = [cc.r as f32 / 255., cc.g as f32 / 255., cc.b as f32 / 255.];
            ui.color_edit_button_rgb(&mut col);
            events.set_export_clear_color(col[0], col[1], col[2]);
        });
    });

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let str = &shared_ui.loc("export_modal.save_button");
                if ui.skf_button(str).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        *shared_ui.saving.lock().unwrap() = crate::Saving::Spritesheet;
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    utils::open_save_dialog(
                        &shared_ui.file_path,
                        &shared_ui.saving,
                        crate::Saving::Exporting,
                    );
                    shared_ui.export_modal = false;
                }
            });
        });
    });
}

pub fn spritesheet_export(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    edit_mode: &EditMode,
    events: &mut EventState,
    config: &Config,
    armature: &Armature,
    camera: &Camera,
    selections: &SelectionState,
) {
    ui.heading("Export Spritesheet");
    ui.add_space(10.);
    ui.horizontal(|ui| {
        ui.label("Size per sprite: ");
        let x = shared_ui.sprite_size.x;
        let (edited, value, _) = ui.float_input("sprite_size_x".into(), shared_ui, x, 1., None);
        if edited {
            shared_ui.sprite_size.x = value;
        }
        ui.label("x");
        let y = shared_ui.sprite_size.y;
        let (edited, value, _) = ui.float_input("sprite_size_y".into(), shared_ui, y, 1., None);
        if edited {
            shared_ui.sprite_size.y = value;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Sprites per row: ");
        let spr = shared_ui.sprites_per_row as f32;
        let (edited, value, _) = ui.float_input("sprite_row".into(), shared_ui, spr, 1., None);
        if edited {
            shared_ui.sprites_per_row = value as i32;
        }
    });

    ui.add_space(10.);
    ui.heading(shared_ui.loc("export_modal.animations"));

    for a in 0..armature.animations.len() {
        #[rustfmt::skip]
        let col = if a % 2 == 0 { config.colors.dark_accent } else { config.colors.main };

        let anim = &armature.animations[a];

        egui::Frame::new().fill(col.into()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(anim.name.to_string());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut meta_col = config.colors.text;
                    meta_col -= crate::Color::new(40, 40, 40, 0);
                    ui.checkbox(&mut shared_ui.exporting_anims[a], "".into_atoms())
                        .on_hover_text(shared_ui.loc("export_modal.animations_check_desc"));
                    ui.add_space(10.);
                    let total_frames = anim.keyframes.last().unwrap().frame;
                    ui.label(
                        egui::RichText::new(
                            anim.fps.to_string()
                                + &" FPS  -  ".to_string()
                                + &total_frames.to_string()
                                + &" frames".to_string(),
                        )
                        .color(meta_col),
                    );
                });
            });
        });
    }

    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let str = &shared_ui.loc("export_modal.save_button");
                if ui.skf_button(str).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    #[cfg(target_arch = "wasm32")]
                    {
                        *shared_ui.saving.lock().unwrap() = crate::Saving::Spritesheet;
                        shared_ui.spritesheet_elapsed = Some(Instant::now());
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    utils::open_save_dialog(
                        &shared_ui.file_path,
                        &shared_ui.saving,
                        crate::Saving::Spritesheet,
                    );
                }
            });
        });
    });
}
