use ui::EguiUi;

use crate::*;

type W = Warnings;

pub fn check_warnings(armature: &Armature, selections: &SelectionState) -> Vec<Warning> {
    let mut warnings: Vec<Warning> = vec![];

    for b in 0..armature.bones.len() {
        let bone = &armature.bones[b];
        if bone.tex != "" {
            let same_zindex = armature
                .bones
                .iter()
                .find(|b| b.zindex == bone.zindex && b.id != bone.id)
                != None;
            if same_zindex {
                let same_warning = warnings
                    .iter_mut()
                    .find(|w| w.warn_type == W::SameZIndex && w.value == bone.zindex as f32);
                if same_warning != None {
                    same_warning.unwrap().ids.push(bone.id as usize);
                } else {
                    let zindex = bone.zindex as f32;
                    warnings.push(Warning::new(W::SameZIndex, vec![bone.id as usize], zindex));
                }
            }
        }
    }

    warnings
}

pub fn warnings_popup(
    ui: &mut egui::Ui,
    shared_ui: &crate::Ui,
    armature: &Armature,
    events: &mut EventState,
) {
    for w in 0..shared_ui.warnings.len() {
        let warning = &shared_ui.warnings[w];

        egui::Frame::new().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.set_width(ui.available_width().min(300.));
                ui.set_height(21.);
                ui.add_space(5.);
                if warning.warn_type == W::SameZIndex {
                    let str = shared_ui
                        .loc("warnings.SameZIndex")
                        .replace("$b", &warning.ids.len().to_string())
                        .replace("$z", &warning.value.to_string());
                    ui.label(egui::RichText::new(str).strong());
                    for i in 0..warning.ids.len() {
                        let id = warning.ids[i];
                        let bones = &armature.bones;
                        let bone_name = &bones.iter().find(|b| b.id == id as i32).unwrap().name;
                        let mut str = bone_name.to_string();
                        if i != warning.ids.len() - 1 {
                            str += ",";
                        }
                        if ui.clickable_label(str).clicked() {
                            let idx = armature
                                .bones
                                .iter()
                                .position(|b| b.id == id as i32)
                                .unwrap();
                            events.select_bone(idx, false);
                        };
                    }
                }
            });
        });
    }
}
