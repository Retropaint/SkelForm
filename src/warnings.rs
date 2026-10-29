use ui::EguiUi;

use crate::*;

type W = Warnings;

pub fn check_warnings(armature: &Armature) -> Vec<Warning> {
    let mut warnings: Vec<Warning> = vec![];

    for b in 0..armature.bones.len() {
        let bone = &armature.bones[b];

        // Warning::SameZindex
        if bone.tex != "" {
            let same_zindex = armature
                .bones
                .iter()
                .find(|b| b.tex != "" && b.zindex == bone.zindex && b.id != bone.id)
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

        // Warning::OnlyPath
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
            let mut no_weights = bone.binds[b].verts.len() > 0;
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
        // prepare all texture names in a list
        let mut remaining_texes: Vec<String> = vec![];
        for style in &armature.styles {
            let mut tex_names: Vec<String> =
                style.textures.iter().map(|t| t.name.clone()).collect();
            remaining_texes.append(&mut tex_names);
        }
        remaining_texes.dedup();

        // get all textures from bones
        let mut all_bone_tex_names: Vec<String> =
            armature.bones.iter().map(|b| b.tex.clone()).collect();
        all_bone_tex_names.dedup();

        // get all textures from animations
        let mut all_anim_texes: Vec<String> = vec![];
        for anim in &armature.animations {
            for keyframe in &anim.keyframes {
                if keyframe.element == AnimElement::Texture {
                    all_anim_texes.push(keyframe.value_str.to_string());
                }
            }
        }

        // get textures that were unused in both bones and animations
        remaining_texes
            .retain(|name| !all_bone_tex_names.contains(name) && !all_anim_texes.contains(name));
        if remaining_texes.len() > 0 {
            let warn = W::UnusedTextures;
            warnings.push(Warning::full(warn, vec![], 0., remaining_texes));
        }
    }

    // Warning::BlockedStyles
    {
        // a style is blocked if it's below another that has all overlapping texture names

        let mut unr_style_ids = vec![];
        let mut blocking_style_name = vec![];

        for s0 in 0..armature.styles.len() {
            let style0 = &armature.styles[s0];
            if !style0.active {
                continue;
            }
            for s1 in 0..armature.styles.len() {
                if s1 >= s0 {
                    break;
                }
                let style1 = &armature.styles[s1];
                if !style1.active {
                    continue;
                }

                let texes0: Vec<String> = style0.textures.iter().map(|t| t.name.clone()).collect();
                let texes1: Vec<String> = style1.textures.iter().map(|t| t.name.clone()).collect();
                if texes0.iter().all(|item| texes1.contains(item)) && texes1.len() >= texes0.len() {
                    // style1 is blocking style0
                    unr_style_ids.push(style0.id as usize);
                    blocking_style_name.push(style1.name.to_string());
                    break;
                }
            }
        }

        if unr_style_ids.len() > 0 {
            let warn = W::BlockedStyles;
            warnings.push(Warning::full(warn, unr_style_ids, 0., blocking_style_name));
        }
    }

    // Warning::SameNameBones
    {
        let warn = W::SameNameBones;
        let mut name_groups: Vec<(&String, &String, Vec<usize>)> = vec![];

        for bone in &armature.bones {
            if name_groups.iter().find(|ng| *ng.0 == bone.name) != None {
                continue;
            }

            let culprit_anim;
            if let Some(anim) = armature
                .animations
                .iter()
                .find(|anim| anim.keyframes.iter().find(|kf| kf.bone_id == bone.id) != None)
            {
                // this bone is part of an animation, so it could be problematic
                culprit_anim = anim
            } else {
                // this bone is anot in an animation; same-name is fine
                continue;
            }

            let id_group: Vec<usize> = armature
                .bones
                .iter()
                .filter(|b| b.name == bone.name)
                .map(|b| b.id as usize)
                .collect();
            if id_group.len() > 1 {
                // there's more than 1 bone with this name, meaning there's a duplicate!
                name_groups.push((&bone.name, &culprit_anim.name, id_group));
            }
        }

        // create warnings separately for each name group
        for group in name_groups {
            let anim_name = vec![group.1.clone()];
            warnings.push(Warning::full(warn.clone(), group.2, 0., anim_name));
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
                .replace("$zindex", &warning.value.to_string());
            bone_list(str, "".to_string(), ui, warning, bones, events, warn_color);
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
                    events.select_style(warning.ids[i] as i32);
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
        W::BlockedStyles => {
            let str = shared_ui.loc("warnings.BlockedStyles");
            ui.label(egui::RichText::new(str).color(warn_color))
                .on_hover_text(shared_ui.loc("warnings.BlockedStyles_desc"));
            for s in 0..warning.ids.len() {
                let id = warning.ids[s];
                let style_name = &styles.iter().find(|b| b.id == id as i32).unwrap().name;
                let mut str = format!("{}/{}", style_name.to_string(), warning.str_values[s]);
                if s != warning.ids.len() - 1 {
                    str += ",";
                }
                let egui_str = egui::RichText::new(str).underline();
                if ui.clickable_label(egui_str).clicked() {
                    events.select_style(warning.ids[s] as i32);
                    shared_ui.styles_modal = true;
                };
            }
        }
        W::SameNameBones => {
            let str = shared_ui
                .loc("warnings.SameNameBones")
                .replace("$anim", &warning.str_values[0]);
            let tooltip = shared_ui.loc("warnings.SameNameBones_desc");
            bone_list(str, tooltip, ui, warning, bones, events, warn_color);
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

pub fn bone_list(
    headline: String,
    tooltip: String,
    ui: &mut egui::Ui,
    warning: &Warning,
    bones: &Vec<Bone>,
    events: &mut EventState,
    warn_color: Color,
) {
    let text = egui::RichText::new(headline).color(warn_color);
    if tooltip == "" {
        ui.label(text);
    } else {
        ui.label(text).on_hover_text(tooltip);
    }

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
