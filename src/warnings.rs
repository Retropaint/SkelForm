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
                    let bone_id = vec![bone.id as usize];
                    warnings.push(Warning::valued(W::SameZIndex, bone_id, zindex));
                }
            }
        }

        // Warning::NoIkTarget
        if armature.bone_eff(bone.id) == JointEffector::Start && bone.ik_target_id == -1 {
            warnings.push(Warning::new(W::NoIkTarget, vec![bone.id as usize]));
        }

        // Warning::UnboundBind
        // Warning::NoVertsInBind
        for b in 0..bone.binds.len() {
            let id = vec![bone.id as usize];
            if bone.binds[b].bone_id == -1 {
                warnings.push(Warning::valued(W::UnboundBind, id, b as f32));
            } else if bone.binds[b].verts.len() == 0 {
                warnings.push(Warning::valued(W::NoVertsInBind, id, b as f32));
            }
        }

        // Warning::NoVertsInBind
        if bone.binds.len() == 1 && bone.binds[0].is_path {
            warnings.push(Warning::new(W::OnlyPath, vec![bone.id as usize]));
        }

        // Warning::OnlyIk
        // Warning::BoneOutOfFamily
        if armature.bone_eff(bone.id) == JointEffector::Start {
            let mut family: Vec<&Bone> = armature
                .bones
                .iter()
                .filter(|b| b.ik_family_id == bone.ik_family_id)
                .collect();
            if family.len() == 1 {
                warnings.push(Warning::new(W::OnlyIk, vec![bone.id as usize]));
            } else {
                // get all children that are part of this family
                let mut children = vec![bone.clone()];
                armature_window::get_all_children(&armature.bones, &mut children, bone);
                children.retain(|b| b.ik_family_id == bone.ik_family_id);

                if children.len() != family.len() {
                    // get bones in this family that are not children (culprits)
                    let children_ids: Vec<i32> = children.iter().map(|b| b.id).collect();
                    family.retain(|b| !children_ids.contains(&b.id));
                    let ids: Vec<usize> = family.iter().map(|f| f.id as usize).collect();

                    let family_id = bone.ik_family_id as f32;
                    warnings.push(Warning::valued(W::BoneOutOfFamily, ids, family_id));
                }
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
                let bone_id = vec![bone.id as usize];
                warnings.push(Warning::valued(W::NoWeights, bone_id, b as f32));
            }
        }
    }

    // Warning::EmptyStyle
    let mut empty_styles = armature.styles.clone();
    empty_styles.retain(|s| s.textures.len() == 0);
    if empty_styles.len() > 0 {
        let ids: Vec<usize> = empty_styles.iter().map(|s| s.id as usize).collect();
        warnings.push(Warning::new(W::EmptyStyles, ids));
    }

    // Warning::UnassignedTextures
    {
        let mut remaining_texes: Vec<String> = vec![];
        for style in &armature.styles {
            let mut tex_names: Vec<String> =
                style.textures.iter().map(|t| t.name.clone()).collect();
            remaining_texes.append(&mut tex_names);
        }
        remaining_texes.dedup();

        let mut all_bone_tex_names: Vec<String> =
            armature.bones.iter().map(|b| b.tex.clone()).collect();
        all_bone_tex_names.dedup();

        remaining_texes.retain(|name| !all_bone_tex_names.contains(name));
        if remaining_texes.len() > 0 {
            let warn = W::UnusedTextures;
            warnings.push(Warning::full(warn, vec![], 0., remaining_texes));
        }
    }

    warnings.sort_by(|a, b| (a.warn_type.clone() as usize).cmp(&(b.warn_type.clone() as usize)));
    warnings
}

pub fn warning_line(
    ui: &mut egui::Ui,
    warning: &Warning,
    shared_ui: &mut crate::Ui,
    armature: &Armature,
    config: &Config,
    events: &mut EventState,
) {
    let bones = &armature.bones;
    let styles = &armature.styles;
    let warn_color = config.colors.warning_text;

    match warning.warn_type {
        W::SameZIndex => {
            let str = shared_ui
                .loc("warnings.SameZIndex")
                .replace("$bone_count", &warning.ids.len().to_string())
                .replace("$zindex", &warning.value.to_string());
            ui.label(egui::RichText::new(str).color(warn_color));
            for i in 0..warning.ids.len() {
                let id = warning.ids[i];
                let bone_name = &bones.iter().find(|b| b.id == id as i32).unwrap().name;
                let mut str = bone_name.to_string();
                if i != warning.ids.len() - 1 {
                    str += ",";
                }
                let egui_str = egui::RichText::new(str).underline();
                if ui.clickable_label(egui_str).clicked() {
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
            clickable_bone(ui, armature, str, events, warning, warn_color);
        }
        W::UnboundBind => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.UnboundBind")
                .replace("$bind", &warning.value.to_string())
                .replace("$bone", &bone.unwrap().name);
            clickable_bone(ui, armature, str, events, warning, warn_color);
        }
        W::NoVertsInBind => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.NoVertsInBind")
                .replace("$bind", &warning.value.to_string())
                .replace("$bone", &bone.unwrap().name);
            clickable_bone(ui, armature, str, events, warning, warn_color);
        }
        W::OnlyPath => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.OnlyPath")
                .replace("$bone", &bone.unwrap().name);
            clickable_bone(ui, armature, str, events, warning, warn_color);
        }
        W::OnlyIk => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.OnlyIk")
                .replace("$bone", &bone.unwrap().name)
                .replace("$ik_family_id", &bone.unwrap().ik_family_id.to_string());
            clickable_bone(ui, armature, str, events, warning, warn_color);
        }
        W::NoWeights => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.NoWeights")
                .replace("$bone", &bone.unwrap().name)
                .replace("$bind", &warning.value.to_string());
            clickable_bone(ui, armature, str, events, warning, warn_color);
        }
        W::BoneOutOfFamily => {
            let bone = &bones.iter().find(|b| b.id == warning.ids[0] as i32);
            let str = shared_ui
                .loc("warnings.BoneOutOfFamily")
                .replace("$bone", &bone.unwrap().name)
                .replace("$ik_family_id", &warning.value.to_string());
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(str).color(warn_color));
                ui.horizontal(|ui| {
                    let text =
                        egui::RichText::new(shared_ui.loc("warnings.BoneOutOfFamilyCulprits"))
                            .color(warn_color);
                    ui.label(text);
                    for i in 0..warning.ids.len() {
                        let id = warning.ids[i];
                        let bone_name = &bones.iter().find(|b| b.id == id as i32).unwrap().name;
                        let mut str = bone_name.to_string();
                        if i != warning.ids.len() - 1 {
                            str += ",";
                        }
                        let egui_str = egui::RichText::new(str).underline();
                        if ui.clickable_label(egui_str).clicked() {
                            let idx = bones.iter().position(|b| b.id == id as i32).unwrap();
                            events.select_bone(idx, true);
                        };
                    }
                });
            });
        }
        W::EmptyStyles => {
            let str = shared_ui.loc("warnings.EmptyStyles");
            ui.label(egui::RichText::new(str).color(warn_color));
            for i in 0..warning.ids.len() {
                let id = warning.ids[i];
                let style_name = &styles.iter().find(|b| b.id == id as i32).unwrap().name;
                let mut str = style_name.to_string();
                if i != warning.ids.len() - 1 {
                    str += ",";
                }
                let egui_str = egui::RichText::new(str).underline();
                if ui.clickable_label(egui_str).clicked() {
                    events.select_style(warning.ids[i]);
                    shared_ui.styles_modal = true;
                };
            }
        }
        W::UnusedTextures => {
            let str = shared_ui.loc("warnings.UnusedTextures");
            ui.label(egui::RichText::new(str).color(warn_color));
            for s in 0..warning.str_values.len() {
                let mut tex_str = warning.str_values[s].clone();
                if s != warning.str_values.len() - 1 {
                    tex_str += ",";
                }
                ui.label(tex_str);
            }
        }
    }
}

fn clickable_bone(
    ui: &mut egui::Ui,
    armature: &Armature,
    str: String,
    events: &mut EventState,
    warning: &Warning,
    color: Color,
) {
    let text = egui::RichText::new(str).color(color).underline();
    if ui.clickable_label(text).clicked() {
        let bones = &armature.bones;
        let idx = bones.iter().position(|b| b.id == warning.ids[0] as i32);
        events.unselect_all();
        events.select_bone(idx.unwrap(), true);
    };
}
