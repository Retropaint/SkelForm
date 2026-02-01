use egui::IntoAtoms;

use crate::{
    modal::modal_x, ui::EguiUi, utils, Armature, Camera, EditMode, EventState, Saving,
    SelectionState,
};

pub fn draw(
    ctx: &egui::Context,
    shared_ui: &mut crate::Ui,
    edit_mode: &EditMode,
    events: &mut EventState,
    _armature: &Armature,
    _camera: &Camera,
    _selections: &SelectionState,
) {
    egui::Modal::new("export_modal".into()).show(ctx, |ui| {
        ui.set_width(250.);
        ui.set_height(250.);
        ui.heading(shared_ui.loc("export_modal.heading"));
        modal_x(ui, [0., 0.].into(), || {
            shared_ui.export_modal = false;
        });

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

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let str = &shared_ui.loc("export_modal.save_button");
                    if ui.skf_button(str).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        #[cfg(target_arch = "wasm32")]
                        utils::save_web(_armature, _camera, _selections, edit_mode);
                        #[cfg(not(target_arch = "wasm32"))]
                        utils::open_save_dialog(
                            &shared_ui.file_path,
                            &shared_ui.saving,
                            Saving::Exporting,
                        );
                        shared_ui.export_modal = false;
                    }
                });
            });
        });
    });
}
