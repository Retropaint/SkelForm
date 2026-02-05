use egui::IntoAtoms;

use crate::{
    modal::modal_x, ui::EguiUi, utils, Armature, Camera, Config, EditMode, EventState,
    ExportImgFormat, SelectionState,
};

pub fn draw(
    ctx: &egui::Context,
    shared_ui: &mut crate::Ui,
    edit_mode: &EditMode,
    config: &Config,
    events: &mut EventState,
    _armature: &Armature,
    _camera: &Camera,
    _selections: &SelectionState,
) {
    egui::Modal::new("export_modal".into()).show(ctx, |ui| {
        ui.set_width(300.);
        ui.set_height(300.);
        ui.heading(shared_ui.loc("export_modal.heading"));
        modal_x(ui, [0., 0.].into(), || {
            shared_ui.export_modal = false;
        });

        ui.add_space(5.);

        let width = ui.available_width();
        egui::Frame::new()
            .fill(config.colors.dark_accent.into())
            .inner_margin(egui::Margin::same(5))
            .show(ui, |ui| {
                ui.set_width(width);

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
                let mut col: [f32; 3] =
                    [cc.r as f32 / 255., cc.g as f32 / 255., cc.b as f32 / 255.];
                ui.color_edit_button_rgb(&mut col);
                events.set_export_clear_color(col[0], col[1], col[2]);
            });
        });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let str = &shared_ui.loc("export_modal.save_button");
                    if ui.skf_button(str).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        #[cfg(target_arch = "wasm32")]
                        utils::save_web(_armature, _camera, edit_mode, true);
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
    });
}
