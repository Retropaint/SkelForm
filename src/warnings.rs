use ui::EguiUi;

use crate::*;

type W = Warnings;

pub fn check_warnings(armature: &Armature, selections: &SelectionState) -> Vec<Warning> {
    let mut warnings: Vec<Warning> = vec![];

    for b in 0..armature.bones.len() {
        let bone = &armature.bones[b];

        // Warning::SameZindex
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

        // Warning::NoIkTarget
        if armature.bone_eff(bone.id) == JointEffector::Start && bone.ik_target_id == -1 {
            warnings.push(Warning::new(W::NoIkTarget, vec![bone.id as usize], 0.));
        }

        // Warning::UnboundBind
        // Warning::NoVertsInBind
        for b in 0..bone.binds.len() {
            let id = vec![bone.id as usize];
            if bone.binds[b].bone_id == -1 {
                warnings.push(Warning::new(W::UnboundBind, id, b as f32));
            } else if bone.binds[b].verts.len() == 0 {
                warnings.push(Warning::new(W::NoVertsInBind, id, b as f32));
            }
        }

        // Warning::NoVertsInBind
        if bone.binds.len() == 1 && bone.binds[0].is_path {
            warnings.push(Warning::new(W::OnlyPath, vec![bone.id as usize], 0.));
        }

        // Warning::OnlyIk
        if armature.bone_eff(bone.id) == JointEffector::Start {
            let families: Vec<i32> = armature
                .bones
                .iter()
                .map(|b| b.ik_family_id)
                .filter(|id| id == &bone.ik_family_id)
                .collect();
            if families.len() == 1 {
                warnings.push(Warning::new(W::OnlyIk, vec![bone.id as usize], 0.));
            }
        }

        // Warning::NoWeights
        for b in 0..bone.binds.len() {
            let mut no_weights = true;
            for vert in &bone.binds[b].verts {
                if vert.weight > 0. {
                    no_weights = false;
                    break;
                }
            }
            if no_weights {
                warnings.push(Warning::new(W::NoWeights, vec![bone.id as usize], b as f32));
            }
        }
    }

    warnings.sort_by(|a, b| (a.warn_type.clone() as usize).cmp(&(b.warn_type.clone() as usize)));

    warnings
}

pub fn warning_line(
    ui: &mut egui::Ui,
    warning: &Warning,
    shared_ui: &crate::Ui,
    armature: &Armature,
    events: &mut EventState,
) {
    let bones = &armature.bones;

    match warning.warn_type {
        W::SameZIndex => {
            let str = shared_ui
                .loc("warnings.SameZIndex")
                .replace("$bone_count", &warning.ids.len().to_string())
                .replace("$zindex", &warning.value.to_string());
            ui.label(egui::RichText::new(str));
            for i in 0..warning.ids.len() {
                let id = warning.ids[i];
                let bone_name = &bones.iter().find(|b| b.id == id as i32).unwrap().name;
                let mut str = bone_name.to_string();
                if i != warning.ids.len() - 1 {
                    str += ",";
                }
                if ui.clickable_label(str).clicked() {
                    let bones = &armature.bones;
                    let idx = bones.iter().position(|b| b.id == id as i32).unwrap();
                    events.select_bone(idx, true);
                };
            }
        }
        W::NoIkTarget => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.NoIkTarget")
                .replace("$bone", &bone.unwrap().name);
            clickable_bone(ui, armature, str, events, warning);
        }
        W::UnboundBind => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.UnboundBind")
                .replace("$bind", &warning.value.to_string())
                .replace("$bone", &bone.unwrap().name);
            clickable_bone(ui, armature, str, events, warning);
        }
        W::NoVertsInBind => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.NoVertsInBind")
                .replace("$bind", &warning.value.to_string())
                .replace("$bone", &bone.unwrap().name);
            clickable_bone(ui, armature, str, events, warning);
        }
        W::OnlyPath => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.OnlyPath")
                .replace("$bone", &bone.unwrap().name);
            clickable_bone(ui, armature, str, events, warning);
        }
        W::OnlyIk => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.OnlyIk")
                .replace("$bone", &bone.unwrap().name)
                .replace("$ik_family_id", &bone.unwrap().ik_family_id.to_string());
            clickable_bone(ui, armature, str, events, warning);
        }
        W::NoWeights => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.NoWeights")
                .replace("$bone", &bone.unwrap().name)
                .replace("$bind", &warning.value.to_string());
            clickable_bone(ui, armature, str, events, warning);
        }
    }
}

fn clickable_bone(
    ui: &mut egui::Ui,
    armature: &Armature,
    str: String,
    events: &mut EventState,
    warning: &Warning,
) {
    if ui.clickable_label(str).clicked() {
        let bones = &armature.bones;
        let idx = bones.iter().position(|b| b.id == warning.ids[0] as i32);
        events.unselect_all();
        events.select_bone(idx.unwrap(), true);
    };
}
