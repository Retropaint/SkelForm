use crate::*;
use image::DynamicImage;
use spade::Triangulation;
use std::collections::HashMap;
use std::str::FromStr;

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
                E::NewBone | E::DragBone | E::DeleteBone | E::PasteBone  => undo_states.new_undo_bones(&armature.bones),
                E::NewAnimation | E::DeleteAnim => undo_states.new_undo_anims(&armature.animations),
                E::DeleteTex                    => undo_states.new_undo_style(&armature.sel_style(&selections).unwrap()),
                E::DeleteStyle | E::NewStyle    => undo_states.new_undo_styles(&armature.styles),
                E::RenameStyle => if !ui.just_made_style { undo_states.new_undo_style(&armature.sel_style(&selections).unwrap()); ui.just_made_style = false }
                E::RenameAnim  => if !ui.just_made_anim  { undo_states.new_undo_anim( &armature.sel_anim( &selections).unwrap()); ui.just_made_anim  = false }

                E::DeleteKeyframe | E::DeleteKeyframeLine | E::SetKeyframeFrame | E::SetAllKeyframesFrame | E::PasteKeyframes => {
                    undo_states.new_undo_anim(armature.sel_anim(&selections).unwrap())
                }
                E::ResetVertices | E::CenterBoneVerts | E::RemoveVertex | E::TraceBoneVerts => {
                    undo_states.new_undo_bone(&armature.bones[selections.bone_idx])
                }
                _ => {}
            };
    }

    if event == Events::UpdateKeyframeTransition {
        let frame = events.values[0] as usize;
        let is_in = events.values[1] == 1.;
        let handle = Vec2::new(events.values[2], events.values[3]);
        let preset = events.values[4] as i32;

        for kf in &mut armature.sel_anim_mut(selections).unwrap().keyframes {
            if kf.frame != frame as i32 {
                continue;
            }
            if is_in {
                kf.start_handle = handle;
            } else {
                kf.end_handle = handle;
            }
            kf.handle_preset = if preset == -1 {
                HandlePreset::Custom
            } else {
                HandlePreset::from_repr(preset as usize).unwrap()
            };
        }

        events.events.remove(0);
        events.values.drain(0..=4);
    } else if event == Events::SetExportClearColor {
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
        edit_bone(armature, config, events.values[0] as i32, anim_el, events.values[2], events.str_values[0].clone(), anim_id, anim_frame);

        events.events.remove(0);
        events.values.drain(0..=4);
        events.str_values.remove(0);
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
    } else if event == Events::SetAllKeyframesFrame {
        for kf in &mut armature.sel_anim_mut(&selections).unwrap().keyframes {
            if kf.frame == events.values[0] as i32 {
                kf.frame = events.values[1] as i32
            }
        }
        armature.animations[selections.anim].sort_keyframes();

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::SetKeyframeFrame {
        armature.sel_anim_mut(&selections).unwrap().keyframes[events.values[0] as usize].frame =
            events.values[1] as i32;
        armature.animations[selections.anim].sort_keyframes();

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::DragBone {
        // dropping dragged bone and moving it (or setting it as child)
        let is_above = events.values[0] == 1.;
        let pointing_id = events.values[1] as i32;
        let dragging_id = events.values[2] as i32;
        let bones = &armature.bones;
        if selections.bone_ids.len() < 2 {
            selections.bone_idx = bones.iter().position(|b| b.id == dragging_id).unwrap();
            selections.bone_ids = vec![dragging_id];
        }
        drag_bone(armature, pointing_id, selections, is_above);
        events.events.remove(0);
        events.values.drain(0..=2);
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
        Events::ToggleShowingMesh => {
            edit_mode.showing_mesh = value == 1.;
            if value != 1. {
                ui.tracing = false
            }
        }
        Events::ToggleSettingIkTarget => edit_mode.setting_ik_target = value == 1.,
        Events::ToggleOnionLayers => edit_mode.onion_layers = value == 1.,
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
            if !edit_mode.anim_open {
                selections.anim_frame = -1;
            }
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
            let bone = armature.bones[value as usize].clone();
            if ui.is_animating(&edit_mode, &selections) && !bone.locked {
                let anim = armature.animations[selections.anim as usize].clone();
                undo_states.new_undo_anim(&anim);
            } else {
                undo_states.new_undo_bone(&bone);
            }
            *ui.saving.lock().unwrap() = Saving::Autosaving;
        }
        Events::ApplySettings => {
            ui.scale = config.ui_scale;
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
            verts!() = sort_vertices(verts!().clone());
            armature.sel_bone_mut(&sel).unwrap().indices = triangulate(&verts!(), &tex_img);

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
            bone_mut.vertices = sort_vertices(bone_mut.vertices.clone());
            bone_mut.indices = triangulate(&mut bone_mut.vertices, &tex_img);
            cleanup_vertices(bone_mut);

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
            let mut buffer_frames = copy_buffer.keyframes.clone();
            let anim = &mut armature.sel_anim_mut(&selections).unwrap();

            // set copy buffer to new frames, for the retain() later
            for kf in &mut buffer_frames {
                kf.frame = frame;
            }

            // remove identical keyframes in the new frame
            anim.keyframes
                .retain(|kf| buffer_frames.iter().find(|bkf| **bkf == *kf) == None);

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
            let (verts, indices) = trace_mesh(&data.image, ui.tracing_gap, ui.tracing_padding);
            if verts.len() < 4 || indices.len() < 6 {
                open_modal(ui, false, ui.loc("tracing_high_gap"));
                return;
            }
            let bone = &mut armature.sel_bone_mut(&selections).unwrap();
            bone.vertices = verts;
            bone.indices = indices;
            bone.binds = vec![];
            bone.verts_edited = true;
            cleanup_vertices(bone);
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
        Events::CopyKeyframe => {
            copy_buffer.keyframes =
                vec![armature.sel_anim(selections).unwrap().keyframes[value as usize].clone()];
        }
        Events::CopyKeyframesInFrame => {
            *copy_buffer = CopyBuffer::default();
            for kf in 0..armature.sel_anim(&selections).unwrap().keyframes.len() {
                let frame = selections.anim_frame;
                if armature.sel_anim(&selections).unwrap().keyframes[kf].frame == frame {
                    let keyframe = armature.sel_anim(&selections).unwrap().keyframes[kf].clone();
                    copy_buffer.keyframes.push(keyframe);
                }
            }
        }
        Events::SaveAnimation => {
            undo_states.new_undo_anim(&armature.sel_anim(&selections).unwrap());
        }
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

pub fn open_modal(ui: &mut crate::Ui, forced: bool, headline: String) {
    ui.modal = true;
    ui.forced_modal = forced;
    ui.headline = headline.replace("$err", &ui.custom_error);
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
    let bone_id = armature.bones[idx].id;
    // scroll to this bone in keyframe editor
    if let Some(bone) = ui.bone_tops.tops.iter().find(|b| b.id == bone_id) {
        ui.anim.timeline_offset.y =
            bone.height + ui.anim.timeline_offset.y - ui.keyframe_panel_rect.unwrap().top() - 47.;
    }

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
        let old_parents = armature.get_all_parents(false, id);

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
    value_str: String,
    mut anim_id: usize,
    mut anim_frame: i32,
) {
    let bones = &mut armature.bones;
    let bone = bones.iter_mut().find(|b| b.id == bone_id).unwrap();
    let mut init_value = 0.;
    let mut init_value_str = "".to_string();

    // prevent recording into animation if bone is locked
    if bone.locked {
        anim_id = usize::MAX;
        anim_frame = -1;
    }

    // do nothing if anim is playing and 'edit while playing' config is false
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
    macro_rules! set_str {
        ($field:expr, $enum:ident) => {{
            init_value_str = $field.to_string();
            if anim_id == usize::MAX {
                $field = $enum::from_str(&value_str).unwrap()
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
        AnimElement::IkConstraint => set_str!(bone.ik_constraint, JointConstraint),
        AnimElement::Hidden => {
            init_value = shared::bool_as_f32(bone.hidden);
            if anim_id == usize::MAX {
                bone.hidden = shared::f32_as_bool(value)
            }
        }
        AnimElement::Locked => {
            init_value = shared::bool_as_f32(bone.locked);
            if anim_id == usize::MAX {
                bone.locked = shared::f32_as_bool(value)
            }
        }
        AnimElement::IkMode => set_str!(bone.ik_mode, InverseKinematicsMode),
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
        let mut oth_frame = anim[anim_id].keyframes.iter_mut().find(|kf| check_kf!(kf));
        oth_frame.as_mut().unwrap().value = init_value;
        oth_frame.as_mut().unwrap().value_str = init_value_str;
    }
    let frame = anim[anim_id].check_if_in_keyframe(bone_id, anim_frame, element.clone());
    anim[anim_id].keyframes[frame].value = value;
    anim[anim_id].keyframes[frame].value_str = value_str;
}

// remove vertices that are not in any triangle or binds
pub fn cleanup_vertices(bone: &mut Bone) {
    'verts: for v in (0..bone.vertices.len()).rev() {
        if bone.indices.contains(&(v as u32)) {
            continue;
        }
        for bind in &bone.binds {
            let ids: Vec<i32> = bind.verts.iter().map(|v| v.id).collect();
            if ids.contains(&(bone.vertices[v].id as i32)) {
                continue 'verts;
            }
        }
        bone.vertices.remove(v);
        for idx in &mut bone.indices {
            *idx -= if *idx >= v as u32 { 1 } else { 0 };
        }
    }
}

pub fn trace_mesh(
    texture: &image::DynamicImage,
    gap: f32,
    padding: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut poi: Vec<Vec2> = vec![];

    // place points across the image where it's own pixel is fully transparent
    let mut cursor = Vec2::default();
    while cursor.y < texture.height() as f32 + padding {
        let out_of_bounds =
            cursor.x >= texture.width() as f32 || cursor.y >= texture.height() as f32;
        if out_of_bounds
            || image::GenericImageView::get_pixel(texture, cursor.x as u32, cursor.y as u32).0[3]
                == 0
        {
            poi.push(cursor);
        }
        cursor.x += gap;
        if cursor.x > texture.width() as f32 + padding {
            cursor.x = 0.;
            cursor.y += gap;
        }
    }

    // remove points which have 8 neighbours, keeping only points
    // that are closest to the image
    let poi_clone = poi.clone();
    poi.retain(|point| {
        let left = Vec2::new(point.x - gap, point.y);
        let right = Vec2::new(point.x + gap, point.y);
        let up = Vec2::new(point.x, point.y + gap);
        let down = Vec2::new(point.x, point.y - gap);

        let lt = Vec2::new(point.x - gap, point.y + gap);
        let lb = Vec2::new(point.x - gap, point.y - gap);
        let rt = Vec2::new(point.x + gap, point.y + gap);
        let rb = Vec2::new(point.x + gap, point.y - gap);

        macro_rules! p {
            ($dir:expr) => {
                !poi_clone.contains($dir)
                    && $dir.x > 0.
                    && $dir.y > 0.
                    && $dir.x < texture.width() as f32
                    && $dir.y < texture.height() as f32
            };
        }

        p!(&left) || p!(&right) || p!(&up) || p!(&down) || p!(&lt) || p!(&lb) || p!(&rt) || p!(&rb)
    });

    if poi.len() == 0 {
        return (vec![], vec![]);
    }

    // sort points in any winding order
    poi = winding_sort(poi);

    let uv_x = poi[0].x / texture.width() as f32;
    let uv_y = poi[0].y / texture.height() as f32;
    let pos = Vec2::new(poi[0].x, -poi[0].y);
    let mut verts = vec![vert(Some(pos), None, Some(Vec2::new(uv_x, uv_y)))];
    let mut curr_poi = 0;

    // get last point that current one has light of sight on
    // if next point checked happens to be first and there's line of sight, tracing is over
    for p in 0..poi.len() {
        if p == poi.len() - 1 {
            break;
        }
        if line_of_sight(&texture, poi[curr_poi], poi[(p + 1) % (poi.len() - 1)]) {
            continue;
        }
        if p == 0 {
            curr_poi = 1;
            continue;
        }

        let tex = Vec2::new(texture.width() as f32, texture.height() as f32);
        verts.push(Vertex {
            pos: Vec2::new(poi[p - 1].x, -poi[p - 1].y),
            uv: poi[p - 1] / tex,
            id: p as u32,
            ..Default::default()
        });
        curr_poi = p - 1;
    }
    curr_poi = 0;

    // do the same line of sight checks, but in reverse (covers corners that initial side might have missed)
    for p in (0..poi.len()).rev() {
        if line_of_sight(&texture, poi[curr_poi], poi[(p + 1) % (poi.len() - 1)]) {
            continue;
        }
        if p == 0 {
            curr_poi = 1;
            continue;
        }

        // don't add if it's already in vertices
        let ids: Vec<u32> = verts.iter().map(|v| v.id).collect();
        if ids.contains(&(p as u32)) {
            continue;
        }

        let tex = Vec2::new(texture.width() as f32, texture.height() as f32);
        verts.push(Vertex {
            pos: Vec2::new(poi[p - 1].x, -poi[p - 1].y),
            uv: poi[p - 1] / tex,
            id: p as u32,
            ..Default::default()
        });
        curr_poi = p - 1;
    }

    //for point in poi {
    //    verts.push(Vertex {
    //        pos: Vec2::new(point.x, -point.y),
    //        uv: Vec2::new(
    //            point.x / texture.width() as f32,
    //            point.y / texture.height() as f32,
    //        ),
    //        ..Default::default()
    //    });
    //}

    verts = sort_vertices(verts);
    editor::center_verts(&mut verts);
    (verts.clone(), triangulate(&verts, texture))
}

fn winding_sort(mut points: Vec<Vec2>) -> Vec<Vec2> {
    let mut center = Vec2::default();
    for p in &points {
        center += *p;
    }
    center /= points.len() as f32;

    points.sort_by(|a, b| {
        let angle_a = (a.y - center.y).atan2(a.x - center.x);
        let angle_b = (b.y - center.y).atan2(b.x - center.x);
        angle_a.partial_cmp(&angle_b).unwrap()
    });

    points
}

/// sort vertices in cw (or ccw?) order
pub fn sort_vertices(mut verts: Vec<Vertex>) -> Vec<Vertex> {
    let mut center = Vec2::default();
    for v in 0..verts.len() {
        center += verts[v].pos;
    }
    center /= verts.len() as f32;

    verts.sort_by(|a, b| {
        let angle_a = (a.pos.y - center.y).atan2(a.pos.x - center.x);
        let angle_b = (b.pos.y - center.y).atan2(b.pos.x - center.x);
        angle_a.partial_cmp(&angle_b).unwrap()
    });

    verts
}

fn vert(pos: Option<Vec2>, col: Option<VertexColor>, uv: Option<Vec2>) -> Vertex {
    Vertex {
        pos: pos.unwrap_or_default(),
        color: col.unwrap_or_default(),
        uv: uv.unwrap_or_default(),
        ..Default::default()
    }
}

pub fn triangulate(verts: &Vec<Vertex>, tex: &image::DynamicImage) -> Vec<u32> {
    let mut triangulation: spade::DelaunayTriangulation<_> = spade::DelaunayTriangulation::new();
    let size = Vec2::new(tex.width() as f32, tex.height() as f32);

    for vert in verts {
        let _ = triangulation.insert(spade::Point2::new(vert.uv.x, vert.uv.y));
    }

    let mut indices: Vec<u32> = Vec::new();
    for face in triangulation.inner_faces() {
        let tri_indices = face.vertices().map(|v| v.index()).to_vec();
        if tri_indices.len() != 3 {
            continue;
        }

        // check if this triangle is part of the texture, and ignore if not
        let v1 = verts[tri_indices[0]];
        let v2 = verts[tri_indices[1]];
        let v3 = verts[tri_indices[2]];
        let blt = Vec2::new(
            v1.uv.x.min(v2.uv.x).min(v3.uv.x),
            v1.uv.y.min(v2.uv.y).min(v3.uv.y),
        ) * size;
        let brb = Vec2::new(
            v1.uv.x.max(v2.uv.x).max(v3.uv.x),
            v1.uv.y.max(v2.uv.y).max(v3.uv.y),
        ) * size;

        'pixel_check: for x in (blt.x as i32)..(brb.x as i32) {
            for y in (blt.y as i32)..(brb.y as i32) {
                let pos = &Vec2::new(x as f32, y as f32);
                let bary = tri_point(pos, &(v1.uv * size), &(v2.uv * size), &(v3.uv * size));
                let uv = v1.uv * bary.3 + v2.uv * bary.1 + v3.uv * bary.2;
                let pos = Vec2::new(
                    (uv.x * tex.width() as f32).min(tex.width() as f32 - 1.),
                    (uv.y * tex.height() as f32).min(tex.height() as f32 - 1.),
                );
                let pixel_alpha =
                    image::GenericImageView::get_pixel(tex, pos.x as u32, pos.y as u32).0[3];
                if pixel_alpha > 125 {
                    indices.push(tri_indices[0] as u32);
                    indices.push(tri_indices[1] as u32);
                    indices.push(tri_indices[2] as u32);
                    break 'pixel_check;
                }
            }
        }
    }

    indices
}

fn line_of_sight(img: &DynamicImage, mut p0: Vec2, p1: Vec2) -> bool {
    let dx = (p1.x - p0.x).abs();
    let sx = if p0.x < p1.x { 1 } else { -1 };
    let dy = -(p1.y - p0.y).abs();
    let sy = if p0.y < p1.y { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if p0.x >= 0. && p0.y >= 0. && p0.x < img.width() as f32 && p0.y < img.height() as f32 {
            let px = image::GenericImageView::get_pixel(img, p0.x as u32, p0.y as u32);
            if px[3] == 255 {
                return false;
            }
        }

        if p0.x == p1.x && p0.y == p1.y {
            break;
        }
        let e2 = 2. * err;
        if e2 >= dy {
            err += dy;
            p0.x += sx as f32;
        }
        if e2 <= dx {
            err += dx;
            p0.y += sy as f32;
        }
    }

    true
}

fn tri_point(p: &Vec2, a: &Vec2, b: &Vec2, c: &Vec2) -> (f32, f32, f32, f32) {
    let s = a.y * c.x - a.x * c.y + (c.y - a.y) * p.x + (a.x - c.x) * p.y;
    let t = a.x * b.y - a.y * b.x + (a.y - b.y) * p.x + (b.x - a.x) * p.y;

    if (s < 0.0) != (t < 0.0) && s != 0.0 && t != 0.0 {
        return (-1., -1., -1., -1.);
    }

    let area = -b.y * c.x + a.y * (c.x - b.x) + a.x * (b.y - c.y) + b.x * c.y;
    if area == 0.0 {
        return (-1., -1., -1., -1.);
    }

    let s_normalized = s / area;
    let t_normalized = t / area;

    if s_normalized >= 0.0 && t_normalized >= 0.0 && (s_normalized + t_normalized) <= 1.0 {
        let third = 1. - (s_normalized + t_normalized);
        return (area, s_normalized, t_normalized, third);
    }

    (-1., -1., -1., -1.)
}
