use std::{
    fs::OpenOptions,
    io::{Read, Write},
};

#[cfg(all(not(target_os = "windows"), not(target_arch = "wasm32")))]
use std::os::unix::fs::PermissionsExt;

use egui::IntoAtoms;
use zip::ZipArchive;

use crate::{ui::EguiUi, Armature, Config, EditMode, EventState, ExportImgFormat, SettingsState};

#[cfg(target_arch = "wasm32")]
mod web {
    pub use web_time::Instant;
}
#[cfg(target_arch = "wasm32")]
pub use web::*;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use crate::utils;
}
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

pub fn draw(
    ctx: &egui::Context,
    shared_ui: &mut crate::Ui,
    edit_mode: &EditMode,
    config: &Config,
    events: &mut EventState,
    armature: &Armature,
) {
    let mut pressed_export = false;
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

                    let str_armature = shared_ui.loc("export_modal.header_armature").clone();
                    let str_image = shared_ui.loc("export_modal.header_image").clone();
                    let str_video = shared_ui.loc("export_modal.header_video").clone();
                    tab!(str_armature, crate::SettingsState::Ui);
                    tab!(str_image, crate::SettingsState::Animation);
                    tab!(str_video, crate::SettingsState::Keyboard);

                    if !is_hovered {
                        shared_ui.hovering_setting = None;
                    }
                });
            });

            egui::Frame::new().show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let layout = egui::Layout::top_down(egui::Align::Min);
                    ui.with_layout(layout, |ui| match shared_ui.settings_state {
                        SettingsState::Ui => {
                            armature_export(ui, shared_ui, edit_mode, events, config)
                        }
                        SettingsState::Animation => image_export(ui, shared_ui, config, armature),
                        SettingsState::Keyboard => video_export(ui, shared_ui, config, armature),
                        _ => {}
                    });
                });
            });

            let image_or_vid = shared_ui.settings_state == SettingsState::Animation
                || shared_ui.settings_state == SettingsState::Keyboard;
            if image_or_vid && armature.animations.len() == 0 {
                return;
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let str = &shared_ui.loc("export_modal.save_button");
                        //let pressed_enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if ui.skf_button(str).clicked() {
                            pressed_export = true;
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        if shared_ui.settings_state == SettingsState::Keyboard {
                            ui.checkbox(&mut shared_ui.open_after_export, "".into_atoms());
                            ui.label(shared_ui.loc("export_modal.video.open_after_export"));
                        }
                    });
                });
            });
        });
    });

    if !pressed_export {
        return;
    }
    match shared_ui.settings_state {
        SettingsState::Ui => {
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
        SettingsState::Animation => {
            shared_ui.exporting_video_type = crate::ExportVideoType::None;
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
        SettingsState::Keyboard => {
            for anim in &mut shared_ui.exporting_anims {
                *anim = false;
            }
            shared_ui.exporting_anims[shared_ui.exporting_video_anim] = true;
            #[cfg(target_arch = "wasm32")]
            {
                *shared_ui.saving.lock().unwrap() = crate::Saving::Video;
                shared_ui.spritesheet_elapsed = Some(Instant::now());
            }
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_save_dialog(
                &shared_ui.file_path,
                &shared_ui.saving,
                crate::Saving::Video,
            );
        }
        _ => {}
    }
}

pub fn armature_export(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    edit_mode: &EditMode,
    events: &mut EventState,
    config: &Config,
) {
    ui.heading(shared_ui.loc("export_modal.armature.header"));
    ui.add_space(10.);
    let width = ui.available_width() - 20.;
    egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5))
        .show(ui, |ui| {
            ui.set_width(width);
            let ik_str = shared_ui.loc("export_modal.armature.inverse_kinematics");
            let text = egui::RichText::new(ik_str).size(15.);
            ui.label(text);
        });

    ui.add_space(2.);

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.armature.bake_ik"))
            .on_hover_text(shared_ui.loc("export_modal.armature.bake_ik_desc"));
        let mut bake_ik = edit_mode.export_bake_ik;
        ui.checkbox(&mut bake_ik, "".into_atoms());
        if bake_ik != edit_mode.export_bake_ik {
            events.toggle_baking_ik(if bake_ik { 1 } else { 0 });
        }
    });

    ui.add_enabled_ui(edit_mode.export_bake_ik, |ui| {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("export_modal.armature.exclude_ik"))
                .on_hover_text(shared_ui.loc("export_modal.armature.exclude_ik_desc"));
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
            let text =
                egui::RichText::new(shared_ui.loc("export_modal.armature.tex_atlas")).size(15.);
            ui.label(text);
        });
    ui.add_space(5.);

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.armature.img_format"));
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
            ui.label(shared_ui.loc("export_modal.armature.clear_color"))
                .on_hover_text(shared_ui.loc("export_modal.armature.clear_color_desc"));
            let cc = &edit_mode.export_clear_color;
            let mut col: [f32; 3] = [cc.r as f32 / 255., cc.g as f32 / 255., cc.b as f32 / 255.];
            ui.color_edit_button_rgb(&mut col);
            events.set_export_clear_color(col[0], col[1], col[2]);
        });
    });
}

pub fn image_export(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    config: &Config,
    armature: &Armature,
) {
    ui.heading(shared_ui.loc("export_modal.image.header"));
    let width = ui.available_width() - 20.;

    if armature.animations.len() == 0 {
        ui.label(shared_ui.loc("export_modal.no_anims"));
        return;
    }

    ui.add_space(10.);
    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.image.export_type"));
        let str_sequences = shared_ui.loc("export_modal.image.sequences");
        let str_spritesheets = shared_ui.loc("export_modal.image.spritesheets");
        let selected_str = if shared_ui.image_sequences {
            &str_sequences
        } else {
            &str_spritesheets
        };
        egui::ComboBox::new("transition_dropdown".to_string(), "")
            .selected_text(selected_str.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut shared_ui.image_sequences, false, str_spritesheets);
                ui.selectable_value(&mut shared_ui.image_sequences, true, str_sequences);
            })
            .response;
    });

    ui.add_enabled_ui(!shared_ui.image_sequences, |ui: &mut egui::Ui| {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("export_modal.image.sprites_per_row"));
            let spr = shared_ui.sprites_per_row as f32;
            let (edited, value, _) = ui.float_input("sprite_row".into(), shared_ui, spr, 1., None);
            if edited {
                shared_ui.sprites_per_row = value as i32;
            }
        });
    });

    ui.add_space(10.);
    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.image.size_per_sprite"));
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

    ui.add_space(20.);
    egui::Frame::new()
        .fill(config.colors.dark_accent.into())
        .inner_margin(egui::Margin::same(5))
        .show(ui, |ui| {
            ui.set_width(width);
            let text =
                egui::RichText::new(shared_ui.loc("export_modal.image.animations")).size(15.);
            ui.label(text);
        });
    ui.add_space(5.);
    for a in 0..armature.animations.len() {
        #[rustfmt::skip]
        let col = if a % 2 == 1 { config.colors.dark_accent } else { config.colors.main };

        let anim = &armature.animations[a];
        egui::Frame::new().fill(col.into()).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_width(width + 10.);
                ui.label(anim.name.to_string());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut meta_col = config.colors.text;
                    meta_col -= crate::Color::new(40, 40, 40, 0);
                    ui.checkbox(&mut shared_ui.exporting_anims[a], "".into_atoms())
                        .on_hover_text(shared_ui.loc("export_modal.image.animations_check_desc"));
                    ui.add_space(10.);
                    let total_frames = anim.keyframes.last();
                    if total_frames == None {
                        return;
                    }
                    let str = anim.fps.to_string()
                        + &" FPS  -  ".to_string()
                        + &total_frames.unwrap().frame.to_string()
                        + &shared_ui.loc("export_modal.iamge.frames");
                    ui.label(egui::RichText::new(str).color(meta_col));
                });
            });
        });
    }
}

pub fn video_export(
    ui: &mut egui::Ui,
    shared_ui: &mut crate::Ui,
    _config: &Config,
    armature: &Armature,
) {
    ui.heading(shared_ui.loc("export_modal.video.header"));
    let _width = ui.available_width() - 20.;

    if armature.animations.len() == 0 {
        ui.label(shared_ui.loc("export_modal.no_anims"));
        return;
    }

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.video.animation"));
        let anim_idx = shared_ui.exporting_video_anim;
        let dropdown = egui::ComboBox::new("animation_to_export", "")
            .selected_text(armature.animations[anim_idx as usize].name.clone())
            .width(80.);
        dropdown.show_ui(ui, |ui| {
            let export = &mut shared_ui.exporting_video_anim;
            for a in 0..armature.animations.len() {
                ui.selectable_value(export, a, armature.animations[a].name.to_string());
            }
        });
    });

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.video.format"));
        let dropdown = egui::ComboBox::new("export_video", "")
            .selected_text(&shared_ui.exporting_video_type.to_string().to_uppercase())
            .width(80.);
        dropdown.show_ui(ui, |ui| {
            let export = &mut shared_ui.exporting_video_type;
            ui.selectable_value(export, crate::ExportVideoType::Mp4, "MP4");
            ui.selectable_value(export, crate::ExportVideoType::Gif, "GIF");
        });
    });

    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("export_modal.video.resolution"));
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

    let is_mp4 = shared_ui.exporting_video_type == crate::ExportVideoType::Mp4;
    ui.add_enabled_ui(is_mp4, |ui| {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("export_modal.video.background_color"));
            let real = &mut shared_ui.video_clear_bg;
            let mut col: [u8; 3] = [real.r, real.g, real.b];
            ui.color_edit_button_srgb(&mut col);
            *real = crate::shared::Color::new(col[0], col[1], col[2], 255);
        });
    });

    if !is_mp4 {
        shared_ui.anim_cycles = 1;
    }
    ui.add_enabled_ui(is_mp4, |ui| {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("export_modal.video.cycles"));
            let cycles = "anim_cycles".to_string();
            let (edited, value, _) =
                ui.float_input(cycles, shared_ui, shared_ui.anim_cycles as f32, 1., None);
            if edited {
                shared_ui.anim_cycles = value as i32;
            }
        });
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        ui.add_space(20.);
        egui::Frame::new()
            .fill(_config.colors.dark_accent.into())
            .inner_margin(egui::Margin::same(5))
            .show(ui, |ui| {
                ui.set_width(_width);
                let text = egui::RichText::new(shared_ui.loc("export_modal.video.compatibility"))
                    .size(15.);
                ui.label(text);
            });

        // disabled: encoder dropdown - default is always used for now
        if false {
            ui.horizontal(|ui| {
                ui.label(shared_ui.loc("export_modal.video.encoder"));
                ui.add_enabled_ui(is_mp4, |ui| {
                    let encoder_str = &shared_ui.exporting_video_encoder.to_string().to_lowercase();
                    let dropdown = egui::ComboBox::new("export_encoder", "")
                        .selected_text(encoder_str)
                        .width(80.);
                    dropdown.show_ui(ui, |ui| {
                        let export = &mut shared_ui.exporting_video_encoder;
                        ui.selectable_value(export, crate::ExportVideoEncoder::Libx264, "libx264");
                        ui.selectable_value(export, crate::ExportVideoEncoder::AV1, "av1");
                    });
                });
            });
        }
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("export_modal.video.use_system_ffmpeg"))
                .on_hover_text(shared_ui.loc("export_modal.video.use_system_ffmpeg_desc"));
            ui.checkbox(&mut shared_ui.use_system_ffmpeg, "".into_atoms());
        });

        // disabled:
        // optional ffmpeg downloads - would need to compress first for reduced download size
        // and then uncompress via zip

        ui.add_space(10.);
        if ui.skf_button("Download ffmpeg").clicked() {
            let base_url =
                "https://github.com/Retropaint/SkelForm/raw/refs/heads/master/ffmpeg/native/";
            let bin_name;
            let final_bin_name;
            #[cfg(target_os = "macos")]
            {
                bin_name = "ffmpeg-mac-arm.zip";
                final_bin_name = "ffmpeg";
            }
            #[cfg(target_os = "windows")]
            {
                bin_name = "ffmpeg-win.zip";
                final_bin_name = "ffmpeg.exe";
            }
            #[cfg(target_os = "linux")]
            {
                bin_name = "ffmpeg-linux.zip";
                final_bin_name = "ffmpeg";
            }

            // get zip file
            let resp = ureq::get(base_url.to_owned() + bin_name).call().unwrap();
            let mut f = std::fs::File::create(utils::bin_path().join("ffmpeg.zip")).unwrap();
            let bytes_result: Result<Vec<u8>, _> = resp.into_body().into_reader().bytes().collect();
            if let Ok(bytes) = bytes_result {
                _ = f.write(&bytes);
            }

            let options = OpenOptions::new()
                .append(true)
                .read(true)
                .open(utils::bin_path().join("ffmpeg.zip").clone());
            match options {
                Ok(file) => {
                    // unzip it
                    let mut zip = ZipArchive::new(file).unwrap();
                    let download = zip.by_index(0).unwrap();
                    let mut ffmpeg_bin =
                        std::fs::File::create(utils::bin_path().join(final_bin_name)).unwrap();
                    let bytes_result: Result<Vec<u8>, _> = download.bytes().collect();
                    if let Ok(bytes) = bytes_result {
                        _ = ffmpeg_bin.write(&bytes);
                    }
                }
                Err(_) => {}
            }

            let f = std::fs::File::create(utils::bin_path().join(final_bin_name)).unwrap();
            let mut perms = f.metadata().unwrap().permissions();
            perms.set_readonly(false);
            #[cfg(not(target_os = "windows"))]
            {
                perms.set_mode(0o755);
            }
            f.set_permissions(perms).unwrap();
        }
        ui.add_space(2.5);
        #[allow(unused_mut)]
        let mut size_warning = "";
        #[cfg(target_os = "windows")]
        {
            size_warning = " (>100mb).\nThe program will freeze during download, do not close it";
        }
        let str = if std::fs::exists(utils::bin_path().join("ffmpeg")).unwrap() {
            "Re-download ffmpeg if problems occur.".to_string()
        } else {
            "ffmpeg is not installed.\nClick the above button to download it".to_owned()
                + &size_warning
                + "."
        };
        ui.label(str);
    }
}
