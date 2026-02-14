use std::collections::HashMap;

use crate::*;

const MIN_ZOOM: f32 = 1.;

pub fn iterate_events(
    input: &InputStates,
    config: &mut Config,
    events: &mut EventState,
    camera: &mut Camera,
    edit_mode: &mut EditMode,
    selections: &mut SelectionState,
    undo_states: &mut UndoStates,
    armature: &mut Armature,
    copy_buffer: &mut CopyBuffer,
    ui: &mut crate::Ui,
    renderer: &mut crate::Renderer,
) {
    let mut last_event = Events::None;
    let event = events.events[0].clone();

    edit_mode.is_moving = false;
    edit_mode.is_rotating = false;
    edit_mode.is_scaling = false;

    // for every new event, create a new undo state
    // note: `edit_bone` is not included, as its undo is conditional (see `save_edited_bone`)
    if last_event != event {
        last_event = event.clone();

        type E = Events;
        #[rustfmt::skip]
            match last_event {
                E::NewBone | E::DragBone | E::DeleteBone  => undo_states.new_undo_bones(&armature.bones),
                E::NewAnimation | E::DeleteAnim           => undo_states.new_undo_anims(&armature.animations),
                E::DeleteKeyframe | E::DeleteKeyframeLine => undo_states.new_undo_anim(armature.sel_anim(&selections).unwrap()),
                E::DeleteTex                              => undo_states.new_undo_style(&armature.sel_style(&selections).unwrap()),
                E::DeleteStyle | E::NewStyle              => undo_states.new_undo_styles(&armature.styles),
                E::RenameStyle => if !ui.just_made_style { undo_states.new_undo_style(&armature.sel_style(&selections).unwrap()); ui.just_made_style = false }
                E::RenameBone  => if !ui.just_made_bone  { undo_states.new_undo_bone( &armature.sel_bone( &selections).unwrap()); ui.just_made_bone  = false }
                E::RenameAnim  => if !ui.just_made_anim  { undo_states.new_undo_anim( &armature.sel_anim( &selections).unwrap()); ui.just_made_anim  = false }

                E::ResetVertices | E::CenterBoneVerts | E::RemoveVertex | E::TraceBoneVerts => {
                    undo_states.new_undo_bone(&armature.bones[selections.bone_idx])
                }
                _ => {}
            };
    }

    if event == Events::SetExportClearColor {
        edit_mode.export_clear_color = Color::new(
            (events.values[0] * 255.).round() as u8,
            (events.values[1] * 255.).round() as u8,
            (events.values[2] * 255.).round() as u8,
            0,
        );

        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::DeleteKeyframeLine {
        armature
            .sel_anim_mut(&selections)
            .unwrap()
            .keyframes
            .retain(|kf| {
                !(kf.bone_id == events.values[0] as i32
                    && kf.element == AnimElement::from_repr(events.values[1] as usize).unwrap())
            });
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::SetKeyframeTransition {
        for kf in &mut armature.sel_anim_mut(&selections).unwrap().keyframes {
            if kf.frame == events.values[0] as i32 {
                kf.transition = Transition::from_repr(events.values[1] as usize).unwrap();
            }
        }
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::SelectAnimFrame {
        let selected_anim = selections.anim;
        let selected_bone_idx = selections.bone_idx;
        let selected_bone_ids = selections.bone_ids.clone();
        unselect_all(selections, edit_mode);
        selections.anim = selected_anim;
        if events.values[1] != 1. {
            selections.bone_idx = selected_bone_idx;
            selections.bone_ids = selected_bone_ids;
        }
        selections.anim_frame = events.values[0] as i32;
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::ToggleIkDisabled {
        armature.bones[events.values[0] as usize].ik_disabled = events.values[1] == 1.;
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::SetBindWeight {
        let vert = events.values[0] as usize;
        let weight = events.values[1];
        let sel_bind = selections.bind as usize;
        armature.sel_bone_mut(&selections).unwrap().binds[sel_bind].verts[vert].weight = weight;

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::ToggleBindPathing {
        let sel_bind = events.values[0] as usize;
        let is_pathing = events.values[1] == 1.;
        armature.sel_bone_mut(&selections).unwrap().binds[sel_bind].is_path = is_pathing;

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::EditCamera {
        camera.pos = Vec2::new(events.values[0], events.values[1]);
        camera.zoom = MIN_ZOOM.max(events.values[2]);

        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::AdjustVertex {
        let vert = &mut armature.sel_bone_mut(&selections).unwrap().vertices
            [renderer.changed_vert_id as usize];
        vert.pos = Vec2::new(events.values[0], events.values[1]);
        renderer.changed_vert_id = -1;

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::EditBone {
        ui.cursor_icon = egui::CursorIcon::Crosshair;
        let anim_el = AnimElement::from_repr(events.values[1] as usize).unwrap();

        edit_mode.is_moving = edit_mode.current == EditModes::Move;
        edit_mode.is_rotating = edit_mode.current == EditModes::Rotate;
        edit_mode.is_scaling = edit_mode.current == EditModes::Scale;

        let mut anim_id = events.values[3] as usize;
        let anim_frame = events.values[4] as i32;

        if !edit_mode.anim_open {
            anim_id = usize::MAX;
        }

        #[rustfmt::skip]
        edit_bone(armature, config, events.values[0] as i32, anim_el, events.values[2], anim_id, anim_frame);

        events.events.remove(0);
        events.values.drain(0..=4);
    } else if event == Events::ToggleBoneFolded {
        let idx = events.values[0] as usize;

        undo_states.new_undo_bone(&armature.bones[idx]);
        armature.bones[idx].folded = events.values[1] == 1.;

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::MoveTexture {
        let new_idx = events.values[0] as usize;
        let sel = &selections;
        let textures = &mut armature.sel_style_mut(sel).unwrap().textures;
        let tex = textures[events.values[1] as usize].clone();
        textures.remove(events.values[1] as usize);
        textures.insert(new_idx, tex);

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::MigrateTexture {
        let style = &mut armature.sel_style_mut(selections).unwrap();
        let tex = style.textures[events.values[0] as usize].clone();
        style.textures.remove(events.values[0] as usize);
        armature.styles[events.values[1] as usize]
            .textures
            .push(tex);
        ui.dragging_tex = false;

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::MoveStyle {
        armature
            .styles
            .swap(events.values[0] as usize, events.values[1] as usize);
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::ToggleStyleActive {
        armature.styles[events.values[0] as usize].active =
            !armature.styles[events.values[0] as usize].active;
        for b in 0..armature.bones.len() {
            let bone = &armature.bones[b];
            armature.set_bone_tex(bone.id, bone.tex.clone(), usize::MAX, -1);
        }
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::ToggleAnimPlaying {
        let anim = &mut armature.animations[events.values[0] as usize];
        let playing = events.values[1] == 1.;
        anim.elapsed = if playing { Some(Instant::now()) } else { None };
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::SetKeyframeFrame {
        armature.animations[selections.anim].keyframes[events.values[0] as usize].frame =
            events.values[1] as i32;
        armature.animations[selections.anim].sort_keyframes();

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::DragBone {
        // dropping dragged bone and moving it (or setting it as child)
        let pointing_id = events.values[0] as i32;
        let is_above = events.values[1] == 1.;
        drag_bone(armature, pointing_id, selections, is_above);
        events.events.remove(0);
        events.values.drain(0..=1);
    } else {
        // normal events: 1 event ID, 1 set of value(s)

        let event = &events.events[0].clone();
        let value = events.values[0];
        let str_value = events.str_values[0].clone().to_string();

        #[rustfmt::skip]
        editor::process_event(event, value, str_value, camera, &input, edit_mode, selections, undo_states, armature, copy_buffer, ui, renderer, config);

        events.events.remove(0);
        events.values.remove(0);
        events.str_values.remove(0);
    }
}

pub fn process_event(
    event: &crate::Events,
    value: f32,
    str_value: String,
    camera: &mut Camera,
    input: &InputStates,
    edit_mode: &mut EditMode,
    selections: &mut SelectionState,
    undo_states: &mut UndoStates,
    armature: &mut Armature,
    copy_buffer: &mut CopyBuffer,
    ui: &mut crate::Ui,
    renderer: &mut crate::Renderer,
    config: &mut crate::Config,
) {
    match event {
        Events::CamZoomIn => camera.zoom = MIN_ZOOM.max(camera.zoom - 10.),
        Events::CamZoomOut => camera.zoom += 10.,
        Events::EditModeMove => edit_mode.current = EditModes::Move,
        Events::EditModeRotate => edit_mode.current = EditModes::Rotate,
        Events::EditModeScale => edit_mode.current = EditModes::Scale,
        Events::UnselectAll => unselect_all(selections, edit_mode),
        Events::Undo => {
            undo_redo(true, undo_states, armature, selections);
            ui.changed_window_name = false;
        }
        Events::Redo => {
            undo_redo(false, undo_states, armature, selections);
            ui.changed_window_name = false;
        }
        Events::ResetConfig => {
            if let Ok(data) = serde_json::from_str(&utils::config_str()) {
                *config = data;
            }
            if let Ok(data) = serde_json::from_str(&utils::color_str()) {
                config.colors = data;
            }
        }
        Events::RenameBone => armature.bones[value as usize].name = str_value,
        Events::RenameAnim => armature.animations[value as usize].name = str_value,
        Events::PointerOnUi => camera.on_ui = value == 1.,
        Events::ToggleShowingMesh => edit_mode.showing_mesh = value == 1.,
        Events::ToggleSettingIkTarget => edit_mode.setting_ik_target = value == 1.,
        Events::RemoveIkTarget => armature.sel_bone_mut(selections).unwrap().ik_target_id = -1,
        Events::ToggleIkFolded => {
            armature.sel_bone_mut(&selections).unwrap().ik_folded = value == 1.
        }
        Events::ToggleIkDisabled => {
            armature.sel_bone_mut(&selections).unwrap().ik_disabled = value == 1.
        }
        Events::ToggleMeshdefFolded => {
            armature.sel_bone_mut(&selections).unwrap().meshdef_folded = value == 1.
        }
        Events::ToggleEffectsFolded => {
            armature.sel_bone_mut(&selections).unwrap().effects_folded = value == 1.
        }
        Events::CamZoomScroll => {
            camera.zoom = MIN_ZOOM.max(camera.zoom - input.scroll_delta);
            match config.layout {
                UiLayout::Right => camera.pos.x -= input.scroll_delta * 0.5,
                UiLayout::Left => camera.pos.x += input.scroll_delta * 0.5,
                _ => {}
            }
        }
        Events::ToggleAnimPanelOpen => {
            edit_mode.anim_open = value == 1.;
            for anim in &mut armature.animations {
                anim.elapsed = None;
            }
        }
        Events::CancelPendingTexture => {
            _ = armature.sel_style_mut(&selections).unwrap().textures.pop()
        }
        Events::DeleteAnim => {
            _ = armature.animations.remove(value as usize);
            selections.anim = usize::MAX
        }
        Events::RenameStyle => {
            armature.sel_style_mut(&selections).unwrap().name = str_value;
            ui.just_made_style = false
        }
        Events::NewArmature => {
            unselect_all(selections, edit_mode);
            edit_mode.anim_open = false;
            camera.pos = Vec2::new(0., 0.);
            camera.zoom = 2000.;
            *armature = Armature::default();
        }
        Events::NewStyle => {
            let ids = armature.styles.iter().map(|set| set.id).collect();
            armature.styles.push(crate::Style {
                id: generate_id(ids),
                name: "".to_string(),
                textures: vec![],
                active: true,
            });
            ui.rename_id = "style_".to_string() + &(armature.styles.len() - 1).to_string();
            ui.just_made_style = true;
        }
        Events::DeleteKeyframe => {
            _ = armature.animations[selections.anim]
                .keyframes
                .remove(value as usize)
        }
        Events::SelectBone => {
            let render = str_value == "t";
            let val = value as usize;
            select_bone(selections, ui, armature, edit_mode, input, val, render);
        }
        Events::SelectAnim => {
            let val = value as usize;
            selections.anim = if value == f32::MAX { usize::MAX } else { val };
            selections.anim_frame = 0;
        }
        Events::SelectStyle => {
            let val = value as i32;
            selections.style = if value == f32::MAX { -1 } else { val };
        }
        Events::OpenModal => {
            open_modal(ui, value == 1., ui.loc(&str_value));
        }
        Events::OpenPolarModal => {
            ui.polar_id = PolarId::from_repr(value as usize).unwrap();
            ui.polar_modal = true;
            ui.headline = str_value.to_string();
        }
        Events::DeleteBone => {
            let bone = &armature.bones[value as usize];
            let bone_id = bone.id;

            // remove all children of this bone as well
            let mut children = vec![bone.clone()];
            armature_window::get_all_children(&armature.bones, &mut children, &bone);
            children.reverse();
            for bone in &children {
                let idx = armature.bones.iter().position(|b| b.id == bone.id);
                armature.bones.remove(idx.unwrap());
            }

            // remove all references to this bone and it's children from all animations
            let mut set_undo_bone_continued = false;
            for bone in &children {
                for a in 0..armature.animations.len() {
                    let anim = &mut armature.animations[a];
                    let last_len = anim.keyframes.len();

                    // if an animation has this bone, save it in undo data
                    let mut temp_kfs = anim.keyframes.clone();
                    temp_kfs.retain(|kf| kf.bone_id != bone.id);
                    if last_len != temp_kfs.len() && !set_undo_bone_continued {
                        set_undo_bone_continued = true;
                    }

                    armature.animations[a].keyframes = temp_kfs;
                }
            }

            // remove this bone from binds
            for bone in &mut armature.bones {
                for b in 0..bone.binds.len() {
                    if bone.binds[b].bone_id == bone_id {
                        bone.binds.remove(b);
                    }
                }
            }

            // IK bones that target this are now -1
            let context_id = ui.context_id_parsed();
            let bones = &mut armature.bones;
            let targeters = bones.iter_mut().filter(|b| b.ik_target_id == context_id);
            for bone in targeters {
                bone.ik_target_id = -1;
            }

            if selections.bone_idx == value as usize {
                selections.bone_idx = usize::MAX;
            }
        }
        Events::DeleteTex => {
            let style = &mut armature.sel_style_mut(selections).unwrap();
            style.textures.remove(value as usize);
        }
        Events::DeleteStyle => {
            let styles = &mut armature.styles;
            let idx = styles.iter().position(|s| s.id == value as i32).unwrap();
            if selections.style == value as i32 {
                selections.style = -1;
            }
            styles.remove(idx);
        }
        Events::CopyBone => {
            let arm_bones = &armature.bones;
            let mut bones = vec![];
            armature_window::get_all_children(&arm_bones, &mut bones, &arm_bones[value as usize]);
            bones.insert(0, armature.bones[value as usize].clone());
            copy_buffer.bones = bones;
        }
        Events::PasteBone => {
            // determine which id to give the new bone(s), based on the highest current id
            let ids: Vec<i32> = armature.bones.iter().map(|bone| bone.id).collect();
            let mut highest_id = 0;
            for id in ids {
                highest_id = id.max(highest_id);
            }
            highest_id += 1;

            let mut insert_idx = usize::MAX;
            let mut id_refs: HashMap<i32, i32> = HashMap::new();

            for b in 0..copy_buffer.bones.len() {
                let bone = &mut copy_buffer.bones[b];

                highest_id += 1;
                let new_id = highest_id;

                id_refs.insert(bone.id, new_id);
                bone.id = highest_id;

                let val = value as usize;
                if bone.parent_id != -1 && id_refs.get(&bone.parent_id) != None {
                    bone.parent_id = *id_refs.get(&bone.parent_id).unwrap();
                } else if val != usize::MAX {
                    insert_idx = val + 1;
                    bone.parent_id = armature.bones[val].id;
                } else {
                    bone.parent_id = -1;
                }
            }
            if insert_idx == usize::MAX {
                armature.bones.append(&mut copy_buffer.bones);
            } else {
                for bone in &copy_buffer.bones {
                    armature.bones.insert(insert_idx, bone.clone());
                    insert_idx += 1;
                }
            }
        }
        Events::NewAnimation => {
            armature.new_animation();
            let idx = armature.animations.len() - 1;
            ui.original_name = "".to_string();
            ui.rename_id = "anim_".to_owned() + &idx.to_string();
            ui.edit_value = Some("".to_string());
        }
        Events::DuplicateAnim => armature
            .animations
            .push(armature.animations[value as usize].clone()),
        Events::SaveBone => {
            let bone = armature.bones[value as usize].clone();
            undo_states.new_undo_bone(&bone);
            *ui.saving.lock().unwrap() = Saving::Autosaving;
        }
        Events::SaveEditedBone => {
            if ui.is_animating(&edit_mode, &selections) {
                let anim = armature.animations[selections.anim as usize].clone();
                undo_states.new_undo_anim(&anim);
            } else {
                let bone = armature.bones[value as usize].clone();
                undo_states.new_undo_bone(&bone);
            }
            *ui.saving.lock().unwrap() = Saving::Autosaving;
        }
        Events::ApplySettings => {
            ui.scale = config.ui_scale;
            renderer.gridline_gap = config.gridline_gap;
            crate::utils::save_config(&config);
        }
        Events::NewBone => {
            let idx;
            if armature.sel_bone(&selections) == None {
                (_, idx) = armature.new_bone(-1);
            } else {
                let id = armature.sel_bone(&selections).unwrap().id;
                (_, idx) = armature.new_bone(id);
            }
            armature.bones[idx].name = "".to_string();
            let sel = selections;
            select_bone(sel, ui, armature, edit_mode, input, idx, false);
            ui.rename_id = "bone_".to_string() + &idx.to_string();
        }
        Events::SetBoneTexture => {
            let frame = selections.anim_frame;
            armature.set_bone_tex(value as i32, str_value.clone(), selections.anim, frame);
        }
        Events::RemoveVertex => {
            let sel = &selections;
            #[rustfmt::skip]
            macro_rules! verts {() => { armature.sel_bone_mut(&sel).unwrap().vertices }}

            let verts = verts!().clone();
            let tex_img = renderer::sel_tex_img(&armature.sel_bone(&sel).unwrap(), &armature);
            verts!().remove(value as usize);
            verts!() = renderer::sort_vertices(verts!().clone());
            armature.sel_bone_mut(&sel).unwrap().indices =
                renderer::triangulate(&verts!(), &tex_img);

            // remove this vert from its binds
            'bind: for bind in &mut armature.sel_bone_mut(&sel).unwrap().binds {
                for v in 0..bind.verts.len() {
                    if bind.verts[v].id == verts[value as usize].id as i32 {
                        bind.verts.remove(v);
                        break 'bind;
                    }
                }
            }
        }
        Events::DragVertex => {
            let bone = renderer.sel_temp_bone.clone().unwrap();
            let temp_vert = bone.vertices.iter().find(|v| v.id == value as u32);
            if bone.vertices.len() == 0 || temp_vert == None {
                return;
            }

            let mut total_rot = temp_vert.unwrap().offset_rot;
            let mut is_in_path = false;
            for bind in bone.binds {
                let vert = bind.verts.iter().find(|v| v.id == value as i32);
                if vert != None && !bind.is_path {
                    let bones = &renderer.temp_bones;
                    let bind_bone = bones.iter().find(|b| b.id == bind.bone_id).unwrap();
                    total_rot += bind_bone.rot;
                } else if bind.is_path {
                    is_in_path = true;
                }
            }
            if !is_in_path {
                total_rot += bone.rot;
            }

            let mouse_vel = renderer::mouse_vel(&input, &camera);
            let zoom = camera.zoom;
            let og_bone = &mut armature.sel_bone_mut(&selections).unwrap();
            og_bone.verts_edited = true;
            let vert_mut = og_bone.vertices.iter_mut().find(|v| v.id == value as u32);
            vert_mut.unwrap().pos -= utils::rotate(&(mouse_vel * zoom), -total_rot) / bone.scale;
        }
        Events::ClickVertex => {
            let bone_mut = &mut armature.sel_bone_mut(&selections).unwrap();
            let idx = selections.bind as usize;
            let vert_id = bone_mut.vertices[value as usize].id;
            let verts = bone_mut.vertices.clone();

            let bind = &bone_mut.binds[idx];
            if let Some(v) = bind.verts.iter().position(|vert| vert.id == vert_id as i32) {
                bone_mut.binds[idx].verts.remove(v);

                let changed_vert_id = verts.iter().position(|v| v.id == vert_id).unwrap();
                renderer.changed_vert_id = changed_vert_id as i32;

                let temp_bone = renderer.sel_temp_bone.as_ref().unwrap();
                let vert_pos = temp_bone.vertices[changed_vert_id].pos;

                // store this frame's vert pos for adjustment later
                renderer.changed_vert_init_pos = Some(vert_pos);
            } else {
                bone_mut.binds[idx].verts.push(BoneBindVert {
                    id: vert_id as i32,
                    weight: 1.,
                });

                let changed_vert_id = verts.iter().position(|v| v.id == vert_id).unwrap();
                renderer.changed_vert_init_pos = None;
                renderer.changed_vert_id = changed_vert_id as i32;
            }
        }
        Events::RemoveTriangle => {
            let bone = &mut armature.sel_bone_mut(&selections).unwrap();
            bone.indices.remove(value as usize);
            bone.indices.remove(value as usize);
            bone.indices.remove(value as usize);
        }
        Events::NewVertex => {
            // remove drag vertex action, since it's always triggered
            undo_states.undo_actions.pop();
            undo_states.new_undo_bone(&armature.bones[selections.bone_idx]);

            let sel = &selections;
            let tex_img = renderer::sel_tex_img(armature.sel_bone(sel).unwrap(), &armature);
            let bone_mut = armature.sel_bone_mut(sel).unwrap();

            bone_mut.vertices.push(renderer.new_vert.unwrap());
            bone_mut.vertices.last_mut().unwrap().id = 4.max(bone_mut.vertices.len() as u32);
            bone_mut.vertices = renderer::sort_vertices(bone_mut.vertices.clone());
            bone_mut.indices = renderer::triangulate(&mut bone_mut.vertices, &tex_img);

            // remove vertices that are not in any triangle or binds
            'verts: for v in (0..bone_mut.vertices.len()).rev() {
                if bone_mut.indices.contains(&(v as u32)) {
                    continue;
                }
                for bind in &bone_mut.binds {
                    let ids: Vec<i32> = bind.verts.iter().map(|v| v.id).collect();
                    if ids.contains(&(bone_mut.vertices[v].id as i32)) {
                        continue 'verts;
                    }
                }
                bone_mut.vertices.remove(v);
                for idx in &mut bone_mut.indices {
                    *idx -= if *idx >= v as u32 { 1 } else { 0 };
                }
            }

            bone_mut.verts_edited = true;
        }
        Events::AdjustKeyframesByFPS => {
            let anim_mut = armature.sel_anim_mut(selections).unwrap();

            let mut old_unique_keyframes: Vec<i32> =
                anim_mut.keyframes.iter().map(|kf| kf.frame).collect();
            old_unique_keyframes.dedup();

            let mut anim_clone = anim_mut.clone();

            // adjust keyframes to maintain spacing
            let div = anim_mut.fps as f32 / value;
            for kf in &mut anim_clone.keyframes {
                kf.frame = ((kf.frame as f32) / div) as i32
            }

            let mut unique_keyframes: Vec<i32> =
                anim_clone.keyframes.iter().map(|kf| kf.frame).collect();
            unique_keyframes.dedup();

            if unique_keyframes.len() == old_unique_keyframes.len() {
                anim_mut.fps = value as i32;
                anim_mut.keyframes = anim_clone.keyframes;
            } else {
                open_modal(ui, value == 1., ui.loc("keyframe_editor.invalid_fps"));
            }
        }
        Events::PasteKeyframes => {
            let frame = selections.anim_frame;
            let buffer_frames = copy_buffer.keyframes.clone();
            let anim = &mut armature.sel_anim_mut(&selections).unwrap();

            anim.keyframes.retain(|kf| kf.frame != frame);

            for kf in 0..buffer_frames.len() {
                let keyframe = buffer_frames[kf].clone();
                anim.keyframes.push(Keyframe { frame, ..keyframe })
            }

            armature.sel_anim_mut(&selections).unwrap().sort_keyframes();
        }
        Events::RemoveKeyframesByFrame => {
            let anim = armature.sel_anim_mut(&selections).unwrap();
            anim.keyframes.retain(|kf| kf.frame != value as i32);
        }
        Events::ResetVertices => {
            let sel_bone = armature.sel_bone(&selections).unwrap().clone();
            let tex_size = armature.tex_of(sel_bone.id).unwrap().size.clone();
            let (verts, indices) = renderer::create_tex_rect(&tex_size);
            let bone = armature.sel_bone_mut(&selections).unwrap();
            bone.vertices = verts;
            bone.indices = indices;
            bone.binds = vec![];
            bone.verts_edited = false;
            selections.bind = -1;
        }
        Events::SelectBind => {
            if value == -2. {
                let binds = &mut armature.sel_bone_mut(&selections).unwrap().binds;
                binds.push(BoneBind {
                    bone_id: -1,
                    ..Default::default()
                });
                selections.bind = binds.len() as i32 - 1;
            } else if value != -1. {
                selections.bind = value as i32;
            }
        }
        Events::ToggleBindingVerts => {
            edit_mode.setting_bind_verts = !edit_mode.setting_bind_verts;
            let bind =
                &mut armature.sel_bone_mut(&selections).unwrap().binds[selections.bind as usize];
            if ui.was_editing_path {
                bind.is_path = true;
                ui.was_editing_path = false;
            } else {
                ui.was_editing_path = bind.is_path;
                bind.is_path = false;
            }
        }
        Events::CenterBoneVerts => {
            let verts = &mut armature.sel_bone_mut(&selections).unwrap().vertices;
            center_verts(verts)
        }
        Events::TraceBoneVerts => {
            let bone = armature.sel_bone(&selections).unwrap().clone();
            let tex = &armature.tex_of(bone.id).unwrap();
            let tex_data = &armature.tex_data;
            let data = tex_data.iter().find(|d| tex.data_id == d.id).unwrap();
            let (verts, indices) = renderer::trace_mesh(&data.image);
            let bone = &mut armature.sel_bone_mut(&selections).unwrap();
            bone.vertices = verts;
            bone.indices = indices;
            bone.binds = vec![];
            bone.verts_edited = true;
            selections.bind = -1;
        }
        Events::RenameTex => {
            let style = armature.sel_style_mut(&selections).unwrap();
            let t = value as usize;
            let og_name = style.textures[t].name.clone();
            let trimmed = str_value.trim_start().trim_end().to_string();
            style.textures[t].name = trimmed.clone();
            let tex_names: Vec<String> = style.textures.iter().map(|t| t.name.clone()).collect();

            let filter = tex_names.iter().filter(|name| **name == trimmed);
            if filter.count() > 1 {
                style.textures[t].name = og_name.clone();
                open_modal(ui, false, ui.loc("styles_modal.same_name"));
            }

            if !config.keep_tex_str {
                for bone in &mut armature.bones {
                    if bone.tex == og_name {
                        bone.tex = trimmed.clone();
                    }
                }
            }
        }
        Events::OpenFileErrModal => {
            open_modal(ui, false, ui.loc("import_err") + &str_value);
        }
        Events::ToggleBakingIk => edit_mode.export_bake_ik = value == 1.,
        Events::ToggleExcludeIk => edit_mode.export_exclude_ik = value == 1.,
        Events::SetExportImgFormat => {
            edit_mode.export_img_format = ExportImgFormat::from_repr(value as usize).unwrap()
        }
        Events::OpenExportModal => {
            ui.export_modal = true;
            ui.video_clear_bg = config.colors.background;
            ui.exporting_video_type = ExportVideoType::Mp4;
            ui.exporting_anims = vec![];
            ui.anim_cycles = 1;
            for _ in &armature.animations {
                ui.exporting_anims.push(true);
            }
        }
        Events::UpdateConfig => *config = ui.updated_config.clone(),
        _ => {}
    }
}

pub fn center_verts(verts: &mut Vec<Vertex>) {
    let mut min = Vec2::default();
    let mut max = Vec2::default();
    for v in &mut *verts {
        if v.pos.x < min.x {
            min.x = v.pos.x;
        }
        if v.pos.y < min.y {
            min.y = v.pos.y
        }
        if v.pos.x > max.x {
            max.x = v.pos.x;
        }
        if v.pos.y > max.y {
            max.y = v.pos.y;
        }
    }

    let avg = (min + max) / 2.;
    for v in verts {
        v.pos -= avg;
    }
}

fn open_modal(ui: &mut crate::Ui, forced: bool, headline: String) {
    ui.modal = true;
    ui.forced_modal = forced;
    ui.headline = headline.replace("$export_err", &ui.export_error);
}

fn select_bone(
    sel: &mut SelectionState,
    ui: &mut crate::Ui,
    armature: &mut Armature,
    edit_mode: &mut EditMode,
    input: &InputStates,
    idx: usize,
    from_renderer: bool,
) {
    edit_mode.setting_bind_verts = false;
    edit_mode.showing_mesh = false;

    // rename bone if already selected
    if sel.bone_idx == idx && !from_renderer {
        ui.rename_id = "bone_".to_string() + &sel.bone_idx.to_string().clone();
        ui.edit_value = Some(armature.sel_bone(&sel).unwrap().name.clone());
        return;
    }

    // set this bone as IK target if in IK target mode
    if edit_mode.setting_ik_target {
        armature.sel_bone_mut(&sel).unwrap().ik_target_id = armature.bones[idx].id;
        edit_mode.setting_ik_target = false;
        return;
    }

    // set this bone as bind if in bind mode
    if edit_mode.setting_bind_bone {
        let bone_idx = sel.bind as usize;
        let id = armature.bones[idx].id;
        let bind = &mut armature.sel_bone_mut(&sel).unwrap().binds[bone_idx];
        bind.bone_id = id;
        edit_mode.setting_bind_bone = false;
        return;
    }

    sel.bind = -1;
    edit_mode.setting_bind_verts = false;

    // select only this bone if not holding modifiers
    if !input.holding_mod && !input.holding_shift {
        sel.bone_idx = idx;
        sel.bone_ids = vec![armature.bones[idx].id];
        edit_mode.showing_mesh = false;

        // unfold this bone's parents to reveal it in the hierarchy
        let parents = utils::get_all_parents(&armature.bones, armature.sel_bone(sel).unwrap().id);
        let bones = &mut armature.bones;
        for p in parents {
            bones.iter_mut().find(|b| b.id == p.id).unwrap().folded = false;
        }
        return;
    } else if !from_renderer {
        if input.holding_mod {
            let id = armature.bones[idx as usize].id;
            sel.bone_ids.push(id);
        } else {
            let mut first = sel.bone_idx;
            let mut second = idx as usize;
            if first > second {
                first = idx as usize;
                second = sel.bone_idx;
            }
            for i in first..second as usize {
                let bone = &armature.bones[i];
                let this_id = sel.bone_ids.contains(&bone.id);
                let sel_bone = armature.sel_bone(&sel).unwrap();
                if !this_id && bone.parent_id == sel_bone.parent_id {
                    sel.bone_ids.push(bone.id);
                }
            }
        }
    }
}

fn unselect_all(selections: &mut SelectionState, edit_mode: &mut EditMode) {
    selections.bone_idx = usize::MAX;
    selections.bone_ids = vec![];
    selections.anim_frame = -1;
    selections.anim = usize::MAX;
    selections.bind = -1;
    selections.style = -1;
    edit_mode.showing_mesh = false;
    edit_mode.setting_ik_target = false;
    edit_mode.setting_bind_verts = false;
}

pub fn undo_redo(
    undo: bool,
    undo_states: &mut UndoStates,
    armature: &mut Armature,
    selections: &mut SelectionState,
) {
    let action: Action;
    if undo {
        if undo_states.undo_actions.last() == None {
            return;
        }
        action = undo_states.undo_actions.last().unwrap().clone();
    } else {
        if undo_states.redo_actions.last() == None {
            return;
        }
        action = undo_states.redo_actions.last().unwrap().clone();
    }

    // store the state prior to undoing/redoing the action,
    // to add to the opposite stack later
    let mut new_action = action.clone();

    match &action.action {
        ActionType::Bone => {
            let bone = armature.find_bone_mut(action.bones[0].id).unwrap();
            new_action.bones = vec![bone.clone()];
            *bone = action.bones[0].clone();
        }
        ActionType::Bones => {
            new_action.bones = armature.bones.clone();
            armature.bones = action.bones.clone();
            if selections.bone_ids.len() == 0 {
                selections.bone_idx = usize::MAX;
            } else {
                let sel_id = selections.bone_ids[0];
                let sel_idx = armature.bones.iter().position(|b| b.id == sel_id);
                if sel_idx != None {
                    selections.bone_idx = sel_idx.unwrap();
                } else {
                    selections.bone_idx = usize::MAX
                }
            }
        }
        ActionType::Animation => {
            let anim = armature
                .animations
                .iter_mut()
                .find(|a| a.id == action.animations[0].id)
                .unwrap();
            new_action.animations = vec![anim.clone()];
            *anim = action.animations[0].clone();
        }
        ActionType::Animations => {
            new_action.animations = armature.animations.clone();
            armature.animations = action.animations.clone();
            let animations = &mut armature.animations;
            if animations.len() == 0 || selections.anim > animations.len() - 1 {
                selections.anim = usize::MAX;
            }
        }
        ActionType::Style => {
            let id = action.styles[0].id;
            let style = armature.styles.iter_mut().find(|a| a.id == id).unwrap();
            new_action.styles = vec![style.clone()];
            *style = action.styles[0].clone();
        }
        ActionType::Styles => {
            new_action.styles = armature.styles.clone();
            armature.styles = action.styles.clone();
            let style_ids: Vec<i32> = armature.styles.iter().map(|s| s.id).collect();
            if !style_ids.contains(&selections.style) {
                selections.style = -1;
            }
        }
        _ => {}
    }

    // add action(s) to opposing stack
    undo_states.temp_actions.push(new_action);
    if undo {
        undo_states.undo_actions.pop();
        if !action.continued {
            // reverse list to restore order of actions
            undo_states.temp_actions.reverse();
            let temp_actions = &mut undo_states.temp_actions;
            undo_states.redo_actions.append(temp_actions);
            undo_states.temp_actions = vec![];
        }
    } else {
        undo_states.redo_actions.pop();
        if !action.continued {
            // ditto
            undo_states.temp_actions.reverse();
            let temp_actions = &mut undo_states.temp_actions;
            undo_states.undo_actions.append(temp_actions);
            undo_states.temp_actions = vec![];
        }
    }

    undo_states.prev_undo_actions = undo_states.undo_actions.len();
    undo_states.unsaved_undo_actions = undo_states.undo_actions.len();

    // actions tagged with `continue` are part of an action chain
    if action.continued {
        undo_redo(undo, undo_states, armature, selections);
    }
}

pub fn move_bone(bones: &mut Vec<Bone>, old_idx: i32, new_idx: i32, is_setting_parent: bool) {
    let main = &bones[old_idx as usize];
    let anchor = bones[new_idx as usize].clone();

    // gather all bones to be moved (this and its children)
    let mut to_move: Vec<Bone> = vec![main.clone()];
    armature_window::get_all_children(bones, &mut to_move, main);

    // remove them
    for _ in &to_move {
        bones.remove(old_idx as usize);
    }

    // re-add them in the new positions
    if is_setting_parent {
        to_move.reverse();
    }
    for bone in to_move {
        let idx = bones.iter().position(|b| b.id == anchor.id).unwrap();
        bones.insert(idx + is_setting_parent as usize, bone.clone());
    }
}

pub fn drag_bone(
    armature: &mut Armature,
    pointing_id: i32,
    sel: &mut SelectionState,
    is_above: bool,
) {
    if sel.bone_ids.contains(&pointing_id) {
        return;
    }

    // ignore if pointing bone is a child of this
    if sel.bone_ids.len() != 0 {
        let mut children: Vec<Bone> = vec![];
        let id = sel.bone_ids[0];
        let dragged_bone = armature.bones.iter().find(|b| b.id == id).unwrap();
        let db = dragged_bone;
        armature_window::get_all_children(&armature.bones, &mut children, &db);
        let children_ids: Vec<i32> = children.iter().map(|c| c.id).collect();
        if children_ids.contains(&pointing_id) {
            return;
        }
    }

    let mut sorted_ids = sel.bone_ids.clone();
    sorted_ids.sort_by(|a, b| {
        let mut first = *b;
        let mut second = *a;
        if is_above {
            first = *a;
            second = *b;
        }
        let first_idx = armature.bones.iter().position(|b| b.id == first);
        let second_idx = armature.bones.iter().position(|b| b.id == second);
        first_idx.unwrap().cmp(&second_idx.unwrap())
    });

    for id in sorted_ids {
        let old_parents = armature.get_all_parents(id);

        #[rustfmt::skip] macro_rules! dragged {()=>{armature.find_bone_mut(id).unwrap()}}
        #[rustfmt::skip] macro_rules! pointing{()=>{armature.find_bone_mut(pointing_id).unwrap()}}
        #[rustfmt::skip] macro_rules! bones   {()=>{&mut armature.bones}}

        #[rustfmt::skip] let drag_idx = bones!().iter().position(|b| b.id == id).unwrap() as i32;
        #[rustfmt::skip] let point_idx = bones!().iter().position(|b| b.id == pointing_id).unwrap() as i32;

        if is_above {
            // set pointed bone's parent as dragged bone's parent
            dragged!().parent_id = pointing!().parent_id;
            move_bone(bones!(), drag_idx, point_idx, false);
        } else {
            // set pointed bone as dragged bone's parent
            dragged!().parent_id = pointing!().id;
            move_bone(bones!(), drag_idx, point_idx, true);
            pointing!().folded = false;
        }

        // keep bone selected in new dragged position
        let bones = &mut armature.bones;
        let bone_idx = bones.iter().position(|b| b.id == id).unwrap();
        sel.bone_ids = vec![id];
        sel.bone_idx = bone_idx;

        // adjust dragged bone so it stays in place
        armature.offset_pos_by_parent(old_parents, id);
    }
}

fn edit_bone(
    armature: &mut Armature,
    config: &Config,
    bone_id: i32,
    element: AnimElement,
    value: f32,
    anim_id: usize,
    anim_frame: i32,
) {
    let bones = &mut armature.bones;
    let bone = bones.iter_mut().find(|b| b.id == bone_id).unwrap();
    let mut init_value = 0.;

    // do nothing if anim is playing and edit_while_playing config is false
    let anims = &armature.animations;
    let is_any_anim_playing = anims.iter().find(|anim| anim.elapsed != None) != None;
    if !config.edit_while_playing && is_any_anim_playing {
        return;
    }

    macro_rules! set {
        ($field:expr, $field_type:ident) => {{
            init_value = $field as f32;
            if anim_id == usize::MAX {
                $field = value as $field_type;
            }
        }};
    }

    match element {
        AnimElement::PositionX => set!(bone.pos.x, f32),
        AnimElement::PositionY => set!(bone.pos.y, f32),
        AnimElement::Rotation => set!(bone.rot, f32),
        AnimElement::ScaleX => set!(bone.scale.x, f32),
        AnimElement::ScaleY => set!(bone.scale.y, f32),
        AnimElement::Zindex => set!(bone.zindex, i32),
        AnimElement::IkFamilyId => set!(bone.ik_family_id, i32),
        AnimElement::TintR => set!(bone.tint.r, f32),
        AnimElement::TintG => set!(bone.tint.g, f32),
        AnimElement::TintB => set!(bone.tint.b, f32),
        AnimElement::TintA => set!(bone.tint.a, f32),
        AnimElement::Texture => { /* handled in set_bone_tex() */ }
        AnimElement::IkConstraint => {
            init_value = (bone.ik_constraint as usize) as f32;
            if anim_id == usize::MAX {
                bone.ik_constraint = match value {
                    1. => JointConstraint::Clockwise,
                    2. => JointConstraint::CounterClockwise,
                    _ => JointConstraint::None,
                }
            }
        }
        AnimElement::Hidden => {
            init_value = shared::bool_as_f32(bone.is_hidden);
            if anim_id == usize::MAX {
                bone.is_hidden = shared::f32_as_bool(value)
            }
        }
        AnimElement::IkMode => {
            init_value = (bone.ik_mode as usize) as f32;
            if anim_id == usize::MAX {
                bone.ik_mode = match value {
                    0. => InverseKinematicsMode::FABRIK,
                    1. => InverseKinematicsMode::Arc,
                    _ => InverseKinematicsMode::Skip,
                }
            }
        }
    };

    if anim_id == usize::MAX {
        return;
    }

    macro_rules! check_kf {
        ($kf:expr) => {
            $kf.frame == 0 && $kf.element == element && $kf.bone_id == bone_id
        };
    }

    let anim = &mut armature.animations;

    let has_0th = anim[anim_id].keyframes.iter().find(|kf| check_kf!(kf)) != None;
    if anim_frame != 0 && !has_0th {
        anim[anim_id].check_if_in_keyframe(bone_id, 0, element.clone());
        let oth_frame = anim[anim_id].keyframes.iter_mut().find(|kf| check_kf!(kf));
        oth_frame.unwrap().value = init_value;
    }
    let frame = anim[anim_id].check_if_in_keyframe(bone_id, anim_frame, element.clone());
    anim[anim_id].keyframes[frame].value = value;
}
