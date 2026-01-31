use egui::IntoAtoms;

use crate::{modal::modal_x, ui::EguiUi, utils, Vec2};

pub fn draw(ctx: &egui::Context, shared_ui: &mut crate::Ui) {
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
            ui.checkbox(&mut shared_ui.export_bake_ik, "".into_atoms());
        });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.skf_button("Export").clicked() {
                        #[cfg(target_arch = "wasm32")]
                        utils::save_web(armature, camera);
                        #[cfg(not(target_arch = "wasm32"))]
                        utils::open_save_dialog(&shared_ui.file_path, &shared_ui.saving);
                    }
                });
            });
        });
    });
}
