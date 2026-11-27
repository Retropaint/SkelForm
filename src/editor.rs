use crate::*;
use armature_window::get_all_children;
use image::DynamicImage;
use spade::Triangulation;
use std::collections::HashMap;
use std::str::FromStr;
use warnings::check_warnings;

const MIN_ZOOM: f32 = 1.;

pub fn iterate_events(
    input: &InputStates,
    config: &mut Config,
    events: &mut EventState,
    edit_mode: &mut EditMode,
    tabs: &mut Vec<ArmatureTab>,
    psd_armature: &mut Armature,
    copy_buffer: &mut CopyBuffer,
    ui: &mut crate::Ui,
    renderer: &mut crate::Renderer,
) {
    let mut last_event = Events::None;
    let event = events.events[0].clone();
    type E = Events;

    // set buffers dirty (to be updated later) based on the event
    set_buffers_dirty(renderer, &event);

    // for every new event, create a new undo state
    // note: `edit_bone` is not included, as its undo is conditional (see `save_edited_bone`)
    if last_event != event {
        last_event = event.clone();

        let style_id = tabs[ui.sel_tab].selections.style_id;
        let anim = tabs[ui.sel_tab].selections.anim;
        let sel_bone = tabs[ui.sel_tab].selections.sel_bone();

        #[rustfmt::skip]
        match last_event {
            E::NewBone | E::DragBone | E::DeleteBone | E::PasteBone | E::RaiseGlobalZindex |
            E::CreateBindBone | E::NewBonesFromTextures | E::CreateParentBone => {
                tabs[ui.sel_tab].new_undo_bones()
            }
            E::NewAnimation | E::DeleteAnim => tabs[ui.sel_tab].new_undo_anims(),
            E::DeleteSelectedTextures       => tabs[ui.sel_tab].new_undo_style(style_id),
            E::DeleteStyle | E::NewStyle    => tabs[ui.sel_tab].new_undo_styles(),
            E::RenameStyle => if !ui.just_made_style { tabs[ui.sel_tab].new_undo_style(style_id); ui.just_made_style = false }
            E::DeleteSelectedKeyframes | E::DeleteKeyframeLine | E::PasteKeyframesOnFrame |
            E::AdjustKeyframesByFPS | E::AdjustKeyframesBySpeed => {
                tabs[ui.sel_tab].new_undo_anim(anim)
            }
            E::ResetVertices | E::CenterBoneVerts | E::DeleteVertex |E::TraceBoneVerts |
            E::NewVertex | E::DeleteTriangle => {
                tabs[ui.sel_tab].new_undo_bone(sel_bone)
            }
            _ => {}
        };
    }

    let armature = &mut tabs[ui.sel_tab].armature;

    if event == Events::SetTabPos {
        tabs[events.values[0] as usize].armature.tab_pos =
            Vec2::new(events.values[1], events.values[2]);
        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::SetTabOpacity {
        let tab_idx = events.values[0] as usize;
        let opacity = events.values[1] as f32;
        tabs[tab_idx].armature.tab_opacity = opacity;
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::LoadFile {
        *ui.file_paths.lock().unwrap() = vec![PathBuf::from(events.str_values[0].clone())];
        *ui.file_type.lock().unwrap() = ImportedFileType::Other;
        ui.changed_window_name = false;
        events.events.remove(0);
        events.str_values.pop();
    } else if event == Events::MoveTab {
        let mut remove_idx = events.values[0] as usize;
        tabs.insert(
            events.values[1] as usize,
            tabs[events.values[0] as usize].clone(),
        );
        if events.values[0] > events.values[1] {
            remove_idx += 1;
        }
        tabs.remove(remove_idx);
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::UpdateRenderOptions {
        edit_mode.render_options.textures = events.values[0] == 1.;
        edit_mode.render_options.points = events.values[1] == 1.;
        edit_mode.render_options.kites = events.values[2] == 1.;
        edit_mode.render_options.meshes = events.values[3] == 1.;
        edit_mode.render_options.rects = events.values[4] == 1.;

        events.events.remove(0);
        events.values.drain(0..=4);
    } else if event == Events::NewBonesFromTextures {
        let dragged_tex_id = events.values[0] as i32;
        let pos = Vec2::new(events.values[1], events.values[2]);

        // add dragged texture to list
        let mut tex_ids = tabs[ui.sel_tab].selections.tex_ids.clone();
        if !tex_ids.contains(&dragged_tex_id) {
            tex_ids.push(dragged_tex_id);
        }

        for tex_id in &tex_ids {
            tabs[ui.sel_tab].armature.new_bone(-1);
            let tex_name = tabs[ui.sel_tab].sel_style().unwrap().textures[*tex_id as usize]
                .name
                .clone();
            let bone = tabs[ui.sel_tab].armature.bones.last_mut().unwrap();
            let bone_id = bone.id;
            bone.name = tex_name.clone();
            bone.pos = pos;
            let armature = &mut tabs[ui.sel_tab].armature;
            armature.set_bone_tex(bone_id, tex_name, usize::MAX, -1);
        }

        // activate style so the texture is immediately visible
        tabs[ui.sel_tab].sel_style_mut().unwrap().active = true;

        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::DuplicateTexture {
        for i in 0..tabs[ui.sel_tab].selections.tex_ids.len() {
            let id = tabs[ui.sel_tab].selections.tex_ids[i];

            // get texture data of this selected texture
            let style = tabs[ui.sel_tab].sel_style_mut().unwrap();

            // duplicate texture
            let new_tex = style.textures[id as usize].clone();
            style.textures.push(new_tex);
        }

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::MoveAnimation {
        let drag = events.values[0] as usize;
        let mut hov = events.values[1] as usize;
        let anim = armature.animations[drag].clone();
        armature.animations.remove(drag);

        // anim will end up below selected if hov isn't adjusted
        if hov > drag {
            hov -= 1;
        }

        armature.animations.insert(hov, anim);

        tabs[ui.sel_tab].selections.anim = hov;

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::ToggleSelectedTexture {
        let tex_id = events.values[0] as i32;
        let select = events.values[1] == 1.;
        if select {
            if !tabs[ui.sel_tab].selections.tex_ids.contains(&tex_id) {
                tabs[ui.sel_tab].selections.tex_ids.push(tex_id);
            }
        } else {
            tabs[ui.sel_tab]
                .selections
                .tex_ids
                .retain(|id| *id != tex_id);
        }
        tabs[ui.sel_tab].selections.tex_ids.sort();
        ui.last_selected = "texture".to_string();

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::TrimTexture {
        // search WGPU_EVENTS
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::SetExportTexPadding {
        edit_mode.export_tex_padding = Vec2::new(events.values[0], events.values[1]);
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::EditVertexUV {
        let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
        let vert_id = events.values[0] as u32;
        let vert = bone.vertices.iter_mut().find(|v| v.id == vert_id).unwrap();
        vert.uv = Vec2::new(events.values[1], events.values[2]);

        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::EditVertexPos {
        let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
        let vert_id = events.values[0] as u32;
        let vert = bone.vertices.iter_mut().find(|v| v.id == vert_id).unwrap();
        vert.pos = Vec2::new(events.values[1], events.values[2]);

        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::SelectVertex {
        let vert_id = events.values[0] as i32;
        if vert_id == -1 {
            tabs[ui.sel_tab].selections.vert_ids = vec![];
            renderer.clicked_vert_id = -1;
        } else {
            let force_append = events.values[1] == 1.;
            if input.holding_mod || input.holding_shift || force_append {
                tabs[ui.sel_tab].selections.vert_ids.push(vert_id as usize);
            } else {
                tabs[ui.sel_tab].selections.vert_ids = vec![vert_id as usize];
            }
        }

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::UpdateKeyframeTransition {
        let frame = events.values[0] as usize;
        let is_in = events.values[1] == 1.;
        let handle = Vec2::new(events.values[2], events.values[3]);
        let preset = events.values[4] as i32;

        for kf in &mut tabs[ui.sel_tab].sel_anim_mut().unwrap().keyframes {
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
        tabs[ui.sel_tab]
            .sel_anim_mut()
            .unwrap()
            .keyframes
            .retain(|kf| {
                !(kf.bone_id == events.values[0] as i32
                    && kf.element == AnimElement::from_repr(events.values[1] as usize).unwrap())
            });
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::SelectAnimFrame {
        let selected_anim = tabs[ui.sel_tab].selections.anim;
        let selected_bone_ids = tabs[ui.sel_tab].selections.bone_ids.clone();
        let last_selected = ui.last_selected.clone();
        unselect_all(&mut tabs[ui.sel_tab].selections, edit_mode, ui);
        tabs[ui.sel_tab].selections.anim = selected_anim;
        tabs[ui.sel_tab].selections.anim_frame = events.values[0] as i32;

        // select keyframes in this frame if requested
        if events.values[1] == 1. {
            ui.last_selected = "keyframe".to_string();
            if !input.holding_mod && !input.holding_shift && events.values[2] == 0. {
                ui.selected_keyframes = vec![];
            }
            if !input.holding_shift {
                // select just this diamond's keyframes
                for kf in &tabs[ui.sel_tab].sel_anim().unwrap().keyframes {
                    if kf.frame == tabs[ui.sel_tab].selections.anim_frame
                        && !ui.selected_keyframes.contains(&kf)
                    {
                        ui.selected_keyframes.push(kf.clone());
                    }
                }
            } else {
                // select all diamonds' keyframes between this and last selected one
                let anim_frame = tabs[ui.sel_tab].selections.anim_frame;
                let left = ui.last_selected_frame.min(anim_frame);
                let right = ui.last_selected_frame.max(anim_frame);
                for kf in &tabs[ui.sel_tab].sel_anim().unwrap().keyframes {
                    if kf.frame >= left && kf.frame <= right && !ui.selected_keyframes.contains(&kf)
                    {
                        ui.selected_keyframes.push(kf.clone());
                    }
                }
            }
        }

        tabs[ui.sel_tab].selections.bone_ids = selected_bone_ids;
        ui.last_selected_frame = tabs[ui.sel_tab].selections.anim_frame;
        ui.last_selected = last_selected;

        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::ToggleIkDisabled {
        armature.bones[events.values[0] as usize].ik_disabled = events.values[1] == 1.;
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::SetBindWeight {
        let sel_bind = events.values[0] as usize;
        let vert = events.values[1] as usize;
        let weight = events.values[2];
        tabs[ui.sel_tab].sel_bone_mut().unwrap().binds[sel_bind].verts[vert].weight = weight;

        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::ToggleBindPathing {
        let sel_bind = events.values[0] as usize;
        let is_pathing = events.values[1] == 1.;
        tabs[ui.sel_tab].sel_bone_mut().unwrap().binds[sel_bind].is_path = is_pathing;

        // adjust vertices, so they stay in place
        let bind = &tabs[ui.sel_tab].sel_bone_mut().unwrap().binds[sel_bind].clone();
        let sel_bone = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap();
        let temp_bones = &renderer.temp_arm.bones;
        let temp_bone = temp_bones.iter().find(|b| b.id == sel_bone.id).unwrap();
        for vert in &bind.verts {
            let id = vert.id as u32;
            let vert = sel_bone.vertices.iter_mut().find(|v| v.id == id).unwrap();
            let mut rot = renderer::get_path_normal_angle(temp_bones, temp_bone, sel_bind);
            if is_pathing {
                rot = -rot;
            }
            vert.pos = utils::rotate(&vert.pos, rot);
        }

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::EditCamera {
        tabs[ui.sel_tab].camera.pos = Vec2::new(events.values[0], events.values[1]);
        tabs[ui.sel_tab].camera.zoom = MIN_ZOOM.max(events.values[2]);

        events.events.remove(0);
        events.values.drain(0..=2);
    } else if event == Events::EditBone {
        let anim_el = AnimElement::from_repr(events.values[1] as usize).unwrap();
        let mut anim_id = events.values[3] as usize;
        let anim_frame = events.values[4] as i32;

        // don't record in anims if in Armature mode
        if !edit_mode.anim_open {
            anim_id = usize::MAX;
        }

        #[rustfmt::skip]
        edit_bone(armature, config, events.values[0] as i32, anim_el, events.values[2], events.str_values[0].clone(), anim_id, anim_frame);
        cleanup_keyframes(&mut armature.animations);

        ui.warnings = check_warnings(armature);

        edit_mode.edit_time = 0.;

        events.events.remove(0);
        events.values.drain(0..=4);
        events.str_values.remove(0);
    } else if event == Events::ToggleBoneFolded {
        armature
            .bones
            .iter_mut()
            .find(|b| b.id == events.values[0] as i32)
            .unwrap()
            .folded = events.values[1] == 1.;
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::ToggleBoneAnimFolded {
        let id = events.values[0] as i32;
        let bone = &mut armature.bones.iter_mut().find(|b| b.id == id).unwrap();
        bone.anim_folded = events.values[1] == 1.;

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::MoveTexture {
        let textures = &mut tabs[ui.sel_tab].sel_style_mut().unwrap().textures;
        let tex = textures[events.values[0] as usize].clone();
        textures.insert(events.values[1] as usize, tex);
        let mut remove_idx = events.values[0] as usize;
        if events.values[0] > events.values[1] {
            remove_idx += 1;
        }
        textures.remove(remove_idx);

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::MigrateTexture {
        // add dragged texture to ID list
        let mut tex_ids = tabs[ui.sel_tab].selections.tex_ids.clone();
        if !tex_ids.contains(&(events.values[0] as i32)) {
            tex_ids.push(events.values[0] as i32);
        }

        // move textures to other style
        for tex_id in &tex_ids {
            let tex = tabs[ui.sel_tab].sel_style().unwrap().textures[*tex_id as usize].clone();
            tabs[ui.sel_tab].armature.styles[events.values[1] as usize]
                .textures
                .push(tex);
        }

        // remove those textures from current style
        let mut idx = -1;
        tabs[ui.sel_tab]
            .sel_style_mut()
            .unwrap()
            .textures
            .retain(|_| {
                idx += 1;
                !tex_ids.contains(&idx)
            });

        // reset selected IDs, since it's outdated now
        tabs[ui.sel_tab].selections.tex_ids = vec![];

        ui.dragging_element = "".to_string();
        ui.warnings = check_warnings(&tabs[ui.sel_tab].armature);

        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::MoveStyle {
        let style = armature.styles[events.values[0] as usize].clone();
        armature.styles.insert(events.values[1] as usize, style);
        let mut idx = events.values[0] as usize;
        if events.values[0] > events.values[1] {
            idx += 1;
        }
        armature.styles.remove(idx);
        ui.warnings = check_warnings(armature);
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::ToggleStyleActive {
        armature.styles[events.values[0] as usize].active =
            !armature.styles[events.values[0] as usize].active;
        for b in 0..armature.bones.len() {
            let bone = &armature.bones[b];
            if bone.tex != "" {
                armature.set_bone_tex(bone.id, bone.tex.clone(), usize::MAX, -1);
            }
        }
        ui.warnings = check_warnings(armature);
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::ToggleAnimPlaying {
        let anim = &mut armature.animations[events.values[0] as usize];
        let playing = events.values[1] == 1.;
        anim.elapsed = if playing { Some(Instant::now()) } else { None };
        events.events.remove(0);
        events.values.drain(0..=1);
    } else if event == Events::DragBone {
        // dropping dragged bone and moving it (or setting it as child)
        let is_above = events.values[0] == 1.;
        let pointing_id = events.values[1] as i32;
        let dragging_id = events.values[2] as i32;
        if tabs[ui.sel_tab].selections.bone_ids.len() < 2 {
            tabs[ui.sel_tab].selections.bone_ids = vec![dragging_id];
            let tex = tabs[ui.sel_tab].armature.tex_of(dragging_id);
            if tex == None {
                edit_mode.showing_mesh = false;
            }
        } else {
            // only move root bones (in context of selected bones)
            tabs[ui.sel_tab].selections.bone_ids = tabs[ui.sel_tab]
                .selections
                .only_root_bones(&tabs[ui.sel_tab].armature.bones)
        }
        let bone_ids = tabs[ui.sel_tab].selections.bone_ids.clone();
        drag_bone(
            &mut tabs[ui.sel_tab].armature,
            pointing_id,
            &bone_ids,
            is_above,
        );
        events.events.remove(0);
        events.values.drain(0..=2);
    } else {
        // normal events: 1 event ID, 1 set of value(s)

        let event = &events.events[0].clone();
        let value = events.values[0];
        let str_value = events.str_values[0].clone().to_string();
        #[rustfmt::skip]
        editor::simple_event(
            event, value, str_value, &input, edit_mode,
            tabs, psd_armature, copy_buffer, ui, renderer, config
        );

        events.events.remove(0);
        events.values.remove(0);
        events.str_values.remove(0);
    }
}

// process events that only have one numerical, and one string value
pub fn simple_event(
    event: &crate::Events,
    value: f32,
    str_value: String,
    input: &InputStates,
    edit_mode: &mut EditMode,
    tabs: &mut Vec<ArmatureTab>,
    psd_armature: &mut Armature,
    copy_buffer: &mut CopyBuffer,
    ui: &mut crate::Ui,
    renderer: &mut crate::Renderer,
    config: &mut crate::Config,
) {
    match event {
        Events::CamZoomIn => {
            let old_zoom = tabs[ui.sel_tab].camera.zoom;
            tabs[ui.sel_tab].camera.zoom = MIN_ZOOM.max(tabs[ui.sel_tab].camera.zoom - 10.);
            move_camera_with_zoom(&mut tabs[ui.sel_tab].camera, input, old_zoom);
        }
        Events::CamZoomOut => {
            let old_zoom = tabs[ui.sel_tab].camera.zoom;
            tabs[ui.sel_tab].camera.zoom += 10.;
            move_camera_with_zoom(&mut tabs[ui.sel_tab].camera, input, old_zoom);
        }
        Events::EditModeMove => edit_mode.current = EditModes::Move,
        Events::EditModeRotate => edit_mode.current = EditModes::Rotate,
        Events::EditModeScale => edit_mode.current = EditModes::Scale,
        Events::UnselectAll => unselect_all(&mut tabs[ui.sel_tab].selections, edit_mode, ui),
        Events::Undo => {
            undo_redo(true, &mut tabs[ui.sel_tab]);
            ui.changed_window_name = false;
            let armature = &tabs[ui.sel_tab].armature;
            ui.warnings = check_warnings(armature);
        }
        Events::Redo => {
            undo_redo(false, &mut tabs[ui.sel_tab]);
            ui.changed_window_name = false;
            let armature = &tabs[ui.sel_tab].armature;
            ui.warnings = check_warnings(armature);
        }
        Events::ResetConfig => {
            if let Ok(data) = serde_json::from_str(&utils::config_str()) {
                *config = data;
            }
            if let Ok(data) = serde_json::from_str(&utils::color_str()) {
                config.colors = data;
            }
            if let Ok(data) = serde_json::from_str(&utils::config_keys_str()) {
                config.keys = data;
            }
        }
        Events::RenameBone => {
            let idx = tabs[ui.sel_tab]
                .armature
                .bones
                .iter()
                .position(|b| b.name == str_value);
            if idx != None && !config.allow_same_name {
                // there's another bone with the same name
                ui.context_menu.id = format!("bone_{}", idx.unwrap());
                ui.polar_id = PolarId::GoToSameNameBone;
                ui.polar_modal = true;
                ui.headline = ui.loc("polar.same_name_bone");
            } else {
                let bones = &mut tabs[ui.sel_tab].armature.bones;
                let id = value as i32;
                bones.iter_mut().find(|b| b.id == id).unwrap().name = str_value;
            }
            ui.warnings = warnings::check_warnings(&tabs[ui.sel_tab].armature);
        }
        Events::RenameAnim => {
            if !ui.just_made_anim {
                tabs[ui.sel_tab].new_undo_anim(value as usize);
            }
            tabs[ui.sel_tab].armature.animations[value as usize].name = str_value;
            ui.just_made_anim = false;
        }
        Events::PointerOnUi => tabs[ui.sel_tab].camera.on_ui = value == 1.,
        Events::ToggleShowingMesh => {
            edit_mode.showing_mesh = value == 1.;
            if value != 1. {
                ui.tracing = false;
                tabs[ui.sel_tab].selections.hovering_vert_id = -1;
                tabs[ui.sel_tab].selections.vert_ids = vec![];
            }
        }
        Events::ToggleEditingMesh => {
            edit_mode.editing_mesh = !edit_mode.editing_mesh;

            // unselect all verts when switching modes
            tabs[ui.sel_tab].selections.vert_ids = vec![];
        }
        Events::ToggleSettingIkTarget => {
            edit_mode.setting_ik_target = value == 1.;
            if edit_mode.setting_ik_target {
                ui.flash_armature_timer = Some(Instant::now());
            }
        }
        Events::ToggleSettingBindBone => {
            edit_mode.setting_bind_bone = value == 1.;
            if edit_mode.setting_bind_bone {
                ui.flash_armature_timer = Some(Instant::now());
            }
        }
        Events::ToggleOnionLayers => edit_mode.onion_layers = value == 1.,
        Events::DeleteIkTarget => tabs[ui.sel_tab].sel_bone_mut().unwrap().ik_target_id = -1,
        Events::ToggleIkFolded => tabs[ui.sel_tab].sel_bone_mut().unwrap().ik_folded = value == 1.,
        Events::TogglePhysFolded => {
            tabs[ui.sel_tab].sel_bone_mut().unwrap().phys_folded = value == 1.
        }
        Events::ToggleIkDisabled => {
            tabs[ui.sel_tab].sel_bone_mut().unwrap().ik_disabled = value == 1.
        }
        Events::ToggleMeshdefFolded => {
            tabs[ui.sel_tab].sel_bone_mut().unwrap().meshdef_folded = value == 1.
        }
        Events::ToggleEffectsFolded => {
            tabs[ui.sel_tab].sel_bone_mut().unwrap().effects_folded = value == 1.
        }
        Events::CamZoomScroll => {
            let old_zoom = tabs[ui.sel_tab].camera.zoom;
            tabs[ui.sel_tab].camera.zoom =
                MIN_ZOOM.max(tabs[ui.sel_tab].camera.zoom - input.scroll_delta);
            move_camera_with_zoom(&mut tabs[ui.sel_tab].camera, input, old_zoom);
        }
        Events::ToggleAnimPanelOpen => {
            edit_mode.anim_open = value == 1.;
            if !edit_mode.anim_open {
                tabs[ui.sel_tab].selections.anim_frame = -1;
            }
            for anim in &mut tabs[ui.sel_tab].armature.animations {
                anim.elapsed = None;
            }
        }
        Events::CancelPendingTexture => {
            _ = tabs[ui.sel_tab].sel_style_mut().unwrap().textures.pop()
        }
        Events::DeleteAnim => {
            if let Some(idx) = tabs[ui.sel_tab]
                .armature
                .animations
                .iter()
                .position(|anim| anim.id == value as i32)
            {
                _ = tabs[ui.sel_tab].armature.animations.remove(idx);
                tabs[ui.sel_tab].selections.anim = usize::MAX
            }
        }
        Events::RenameStyle => {
            tabs[ui.sel_tab].sel_style_mut().unwrap().name = str_value;
            ui.just_made_style = false
        }
        Events::NewArmature => {
            unselect_all(&mut tabs[ui.sel_tab].selections, edit_mode, ui);
            edit_mode.anim_open = false;
            tabs[ui.sel_tab].camera.pos = Vec2::new(0., 0.);
            tabs[ui.sel_tab].camera.zoom = 2000.;
            ui.save_path = None;
            ui.changed_window_name = false;

            // add armature to list
            let mut replaced = false;
            for a in 0..tabs.len() {
                // replace existing empty armatures
                if tabs[a].armature == Armature::default() {
                    switch_to_armature(tabs.len() - 1, tabs, ui, &config, false);
                    replaced = true;
                    break;
                }
            }
            if !replaced {
                tabs.push(ArmatureTab {
                    armature: Armature::default().clone(),
                    undo_states: UndoStates::default(),
                    selections: SelectionState::init(),
                    camera: Camera::default(),
                    bone_buffer: RenderBuffer::default(),
                });
                switch_to_armature(tabs.len() - 1, tabs, ui, &config, false);
            }
        }
        Events::NewStyle => {
            let ids = tabs[ui.sel_tab]
                .armature
                .styles
                .iter()
                .map(|set| set.id)
                .collect();
            tabs[ui.sel_tab].armature.styles.push(crate::Style {
                id: generate_id(ids),
                name: "".to_string(),
                textures: vec![],
                active: true,
            });
            ui.rename_id =
                "style_".to_string() + &(tabs[ui.sel_tab].armature.styles.len() - 1).to_string();
            ui.just_made_style = true;
        }
        Events::DeleteSelectedKeyframes => {
            for skf in &ui.selected_keyframes {
                let keyframes = &mut tabs[ui.sel_tab].sel_anim_mut().unwrap().keyframes;
                keyframes.retain(|kf| {
                    kf.frame != skf.frame || kf.element != skf.element || kf.bone_id != skf.bone_id
                });
            }
        }
        Events::SelectBone => {
            let render = str_value == "t";
            let val = value as i32;
            select_bone(&mut tabs[ui.sel_tab], ui, edit_mode, input, val, render);
        }
        Events::SelectAnim => {
            let val = value as usize;
            tabs[ui.sel_tab].selections.anim = if value == f32::MAX { usize::MAX } else { val };
            tabs[ui.sel_tab].selections.anim_frame = 0;
        }
        Events::SelectStyle => {
            tabs[ui.sel_tab].selections.bone_ids = vec![];
            tabs[ui.sel_tab].selections.style_id = value as i32;
            ui.last_selected = if value as i32 == -1 { "" } else { "style" }.to_string();
            tabs[ui.sel_tab].selections.tex_ids = vec![];
        }
        Events::OpenModal => {
            open_modal(ui, value == 1., ui.loc(&str_value));
        }
        Events::OpenModalRaw => {
            open_modal(ui, value == 1., str_value);
        }
        Events::OpenPolarModal => {
            ui.polar_id = PolarId::from_repr(value as usize).unwrap();
            ui.polar_modal = true;
            ui.headline = str_value.to_string();
        }
        Events::DeleteBone => {
            let mut ids_to_delete = vec![value as i32];

            // delete all selected bones, if the one being deleted is also selected
            let bone_id = value as i32;
            if tabs[ui.sel_tab].selections.bone_ids.contains(&bone_id) {
                for id in &tabs[ui.sel_tab].selections.bone_ids {
                    ids_to_delete.push(*id);
                }
            }

            tabs[ui.sel_tab].new_undo_anims();
            tabs[ui.sel_tab]
                .undo_states
                .undo_actions
                .last_mut()
                .unwrap()
                .continued = true;

            let armature = &mut tabs[ui.sel_tab].armature;
            for id in ids_to_delete {
                let bone;
                if let Some(result) = armature.bones.iter().find(|b| b.id == id) {
                    bone = result;
                } else {
                    continue;
                }

                // remove all children of this bone as well
                let mut children = vec![bone.clone()];
                armature_window::get_all_children(&armature.bones, &mut children, &bone);
                children.reverse();
                for bone in &children {
                    let idx = armature.bones.iter().position(|b| b.id == bone.id);
                    armature.bones.remove(idx.unwrap());
                }

                // remove all references to this bone and it's children from all animations
                for bone in &children {
                    for a in 0..armature.animations.len() {
                        let anim = &mut armature.animations[a];

                        let mut temp_kfs = anim.keyframes.clone();
                        temp_kfs.retain(|kf| kf.bone_id != bone.id);
                        armature.animations[a].keyframes = temp_kfs;
                    }
                }

                // remove this bone from binds
                for bone in &mut armature.bones {
                    bone.binds.retain(|bind| bind.bone_id != id);
                    for child in &children {
                        bone.binds.retain(|bind| bind.bone_id != child.id);
                    }
                }

                // IK bones that target this are now -1
                let bones = &mut armature.bones;
                let targeters = bones.iter_mut().filter(|b| b.ik_target_id == value as i32);
                for bone in targeters {
                    bone.ik_target_id = -1;
                }
            }

            // de-select bone(s) to prevent any errors
            tabs[ui.sel_tab].selections.bone_ids = vec![];
        }
        Events::DeleteSelectedTextures => {
            let tex_ids = tabs[ui.sel_tab].selections.tex_ids.clone();
            let style = &mut tabs[ui.sel_tab].sel_style_mut().unwrap();
            let mut t = -1;
            style.textures.retain(|_| {
                t += 1;
                !tex_ids.contains(&t)
            });
            tabs[ui.sel_tab].selections.tex_ids = vec![];
        }
        Events::DeleteStyle => {
            if tabs[ui.sel_tab].selections.style_id == value as i32 {
                tabs[ui.sel_tab].selections.style_id = -1;
            }

            let styles = &mut tabs[ui.sel_tab].armature.styles;
            let idx = styles.iter().position(|s| s.id == value as i32).unwrap();
            styles.remove(idx);
        }
        Events::CopyBone => copy_bone(copy_buffer, &mut tabs[ui.sel_tab], value as i32),
        Events::PasteBone => paste_bone(copy_buffer, &mut tabs[ui.sel_tab], value as i32),
        Events::NewAnimation => {
            tabs[ui.sel_tab].armature.new_animation();
            let idx = tabs[ui.sel_tab].armature.animations.len() - 1;
            ui.rename_id = format!("anim_{}", idx.to_string());
            ui.edit_value = Some("".to_string());
        }
        Events::DuplicateAnim => {
            let anims = &tabs[ui.sel_tab].armature.animations;
            let id = anims.iter().position(|a| a.id == value as i32).unwrap();
            let mut new_anim = anims[id].clone();
            let ids: Vec<i32> = anims.iter().map(|anim| anim.id).collect();
            new_anim.id = generate_id(ids);
            tabs[ui.sel_tab].armature.animations.push(new_anim);
        }
        Events::SaveBone => {
            tabs[ui.sel_tab].new_undo_bone(value as i32);
            *ui.saving.lock().unwrap() = Saving::Autosaving;
        }
        Events::SaveEditedBone => {
            // don't save if values are being edited to dragging the input.
            // dragging directly edits bones, so this event would be spammed
            if ui.edited_dragging {
                return;
            }

            let bones = &tabs[ui.sel_tab].armature.bones;
            let bone = bones.iter().find(|b| b.id == value as i32).unwrap().clone();
            if ui.is_animating(&edit_mode, &tabs[ui.sel_tab].selections) && !bone.locked {
                let anim = tabs[ui.sel_tab].selections.anim;
                tabs[ui.sel_tab].new_undo_anim(anim as usize);
            } else {
                // save all bones, if multiple were edited
                if tabs[ui.sel_tab].selections.bone_ids.len() > 0 {
                    tabs[ui.sel_tab].new_undo_bones();
                } else {
                    tabs[ui.sel_tab].new_undo_bone(value as i32);
                }
            }
            *ui.saving.lock().unwrap() = Saving::Autosaving;
        }
        Events::ApplySettings => {
            ui.scale = config.ui_scale;
            crate::utils::save_config(&config);
        }
        Events::SetBoneTexture => {
            let anim = tabs[ui.sel_tab].selections.anim;
            let frame = tabs[ui.sel_tab].selections.anim_frame;
            tabs[ui.sel_tab]
                .armature
                .set_bone_tex(value as i32, str_value.clone(), anim, frame);
            ui.warnings = check_warnings(&tabs[ui.sel_tab].armature);
        }
        Events::DeleteVertex => {
            #[rustfmt::skip]
            macro_rules! verts {() => { tabs[ui.sel_tab].sel_bone_mut().unwrap().vertices }}

            // don't delete if there are only 4 vertices
            if verts!().len() == 4 {
                open_modal(ui, false, ui.loc("vert_limit"));
                return;
            }

            let idx = verts!().iter().position(|v| v.id == value as u32).unwrap();
            verts!().remove(idx);
            sort_vertices(&mut verts!());

            let bone_id = tabs[ui.sel_tab].sel_bone().unwrap().id;
            let tex = tabs[ui.sel_tab].armature.tex_of(bone_id).unwrap();
            let tex_img = &tabs[ui.sel_tab].armature.tex_data(tex).unwrap().image;

            let indices = triangulate(&tabs[ui.sel_tab].sel_bone().unwrap().vertices, &tex_img);
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.indices = indices;

            remove_blacklisted_tris(&mut bone.indices, &bone.vertices, &mut bone.blacklist);
            cleanup_vertices(bone);

            // remove vertex from selected IDs
            let idx = tabs[ui.sel_tab]
                .selections
                .vert_ids
                .iter()
                .position(|id| (*id) == value as usize);
            if idx != None {
                tabs[ui.sel_tab].selections.vert_ids.remove(idx.unwrap());
            }

            // remove this vert from its binds
            'bind: for bind in &mut tabs[ui.sel_tab].sel_bone_mut().unwrap().binds {
                for v in 0..bind.verts.len() {
                    if bind.verts[v].id == value as i32 {
                        bind.verts.remove(v);
                        continue 'bind;
                    }
                }
            }
        }
        Events::DragVertex => {
            let bones = &renderer.temp_arm.bones;
            let sel_id = tabs[ui.sel_tab].sel_bone().unwrap().id;
            let bone = bones.iter().find(|b| b.id == sel_id).clone().unwrap();
            let temp_vert = bone.vertices.iter().find(|v| v.id == value as u32);
            if bone.vertices.len() == 0 || temp_vert == None {
                return;
            }

            // total rotation to cancel out when dragging vertex
            let mut total_rot = temp_vert.unwrap().offset_rot;

            // if a bind has a weight of 1, all other binds have no effect
            let mut overridden = false;

            let mut binds = bone.binds.clone();
            let mut scale = Vec2::new(1., 1.);
            binds.reverse();
            for bind in &binds {
                if overridden {
                    break;
                }
                if bind.bone_id == -1 {
                    continue;
                }
                let vert = bind.verts.iter().find(|v| v.id == value as i32);
                if vert == None {
                    continue;
                }

                let bones = &renderer.temp_arm.bones;
                let bind_bone = bones.iter().find(|b| b.id == bind.bone_id).unwrap();
                scale = bind_bone.scale;
                if !bind.is_path {
                    total_rot += bind_bone.rot * vert.unwrap().weight;
                }
                overridden |= vert.unwrap().weight == 1.;
            }
            // add mesh bone's own rotation, if this vert is not bound
            if !overridden {
                total_rot += bone.rot;
                scale = bone.scale;
            }
            total_rot += bone.pivot_rot;

            let mouse_vel = renderer::mouse_vel(&input, &tabs[ui.sel_tab].camera);
            let zoom = tabs[ui.sel_tab].camera.zoom;
            let og_bone = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap();
            og_bone.set_editing_vertices(true);
            let vert_mut = og_bone.vertices.iter_mut().find(|v| v.id == value as u32);
            vert_mut.unwrap().pos -= utils::rotate(&(mouse_vel * zoom), -total_rot) / scale;
        }
        Events::ClickVertex => {
            if tabs[ui.sel_tab].selections.bind == -1 {
                return;
            }
            let idx = tabs[ui.sel_tab].selections.bind as usize;
            let bone_mut = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap();
            let vert_id = bone_mut.vertices[value as usize].id;
            let bind = &bone_mut.binds[idx];

            // add/remove vertex to bind
            let mut bound = false;
            if let Some(v) = bind.verts.iter().position(|vert| vert.id == vert_id as i32) {
                bone_mut.binds[idx].verts.remove(v);
            } else {
                bound = true;
                bone_mut.binds[idx].verts.push(BoneBindVert {
                    id: vert_id as i32,
                    weight: 1.,
                });
            }

            adjust_vert_by_bind(
                &mut bone_mut.vertices[value as usize],
                bound,
                &bone_mut.binds[idx],
                &renderer.temp_arm.bones,
                renderer
                    .temp_arm
                    .bones
                    .iter()
                    .find(|b| b.id == bone_mut.id)
                    .unwrap(),
            );
        }
        Events::DeleteTriangle => {
            let bone = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.blacklist
                .push(bone.vertices[bone.indices[value as usize + 0] as usize].id);
            bone.blacklist
                .push(bone.vertices[bone.indices[value as usize + 1] as usize].id);
            bone.blacklist
                .push(bone.vertices[bone.indices[value as usize + 2] as usize].id);
            remove_blacklisted_tris(&mut bone.indices, &bone.vertices, &mut bone.blacklist);
        }
        Events::NewVertex => {
            // remove drag vertex action, since it's always triggered
            tabs[ui.sel_tab].undo_states.undo_actions.pop();
            let sel_bone = tabs[ui.sel_tab].selections.sel_bone();
            tabs[ui.sel_tab].new_undo_bone(sel_bone);

            // give unique ID to vertex
            let bone_verts = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap().vertices;
            bone_verts.push(renderer.new_vert.unwrap());
            let ids: Vec<i32> = bone_verts.iter().map(|v| v.id as i32).collect();
            bone_verts.last_mut().unwrap().id = generate_id(ids) as u32;

            // add vertex to mesh
            sort_vertices(&mut tabs[ui.sel_tab].sel_bone_mut().unwrap().vertices);

            let bone_id = tabs[ui.sel_tab].sel_bone().unwrap().id;
            let tex = tabs[ui.sel_tab].armature.tex_of(bone_id).unwrap();
            let tex_img = &tabs[ui.sel_tab].armature.tex_data(tex).unwrap().image;
            let indices = triangulate(&tabs[ui.sel_tab].sel_bone().unwrap().vertices, &tex_img);

            let bone_mut = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone_mut.indices = indices;

            remove_blacklisted_tris(
                &mut bone_mut.indices,
                &bone_mut.vertices,
                &mut bone_mut.blacklist,
            );
            cleanup_vertices(bone_mut);

            // set this bone as having a mesh
            bone_mut.verts_edited = true;
        }
        Events::AdjustKeyframesByFPS => {
            let anim_mut = tabs[ui.sel_tab].sel_anim_mut().unwrap();

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
        Events::PasteKeyframesOnFrame => {
            paste_keyframes_on_frame(copy_buffer, &mut tabs[ui.sel_tab], value as i32)
        }
        Events::DeleteKeyframesByFrame => {
            let anim = tabs[ui.sel_tab].sel_anim_mut().unwrap();
            anim.keyframes.retain(|kf| kf.frame != value as i32);
        }
        Events::ResetVertices => {
            let sel_bone = tabs[ui.sel_tab].sel_bone().unwrap().clone();
            let tex_size = tabs[ui.sel_tab]
                .armature
                .tex_of(sel_bone.id)
                .unwrap()
                .size
                .clone();
            let (verts, indices) = renderer::create_tex_rect(&tex_size);
            tabs[ui.sel_tab].selections.vert_ids = vec![];
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.vertices = verts;
            bone.indices = indices;
            bone.binds = vec![];
            bone.set_editing_vertices(false);
            tabs[ui.sel_tab].selections.bind = -1;
        }
        Events::SelectBind => {
            if value == -3. {
                tabs[ui.sel_tab].selections.bind = -1;
            } else if value == -2. {
                let binds = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap().binds;
                binds.push(BoneBind {
                    bone_id: -1,
                    ..Default::default()
                });
                tabs[ui.sel_tab].selections.bind = binds.len() as i32 - 1;
            } else if value != -1. {
                tabs[ui.sel_tab].selections.bind = value as i32;
            }
        }
        Events::CenterBoneVerts => {
            let verts = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap().vertices;
            center_verts(verts)
        }
        Events::TraceBoneVerts => {
            let bone = tabs[ui.sel_tab].sel_bone().unwrap().clone();
            let tex = &tabs[ui.sel_tab].armature.tex_of(bone.id).unwrap();
            let tex_data = &tabs[ui.sel_tab].armature.tex_data;
            let data = tex_data.iter().find(|d| tex.data_id == d.id).unwrap();
            let (verts, indices) = trace_mesh(&data.image, ui.tracing_gap, ui.tracing_padding);
            if verts.len() < 4 || indices.len() < 6 {
                open_modal(ui, false, ui.loc("tracing_high_gap"));
                return;
            }
            tabs[ui.sel_tab].selections.vert_ids = vec![];
            let bone = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.vertices = verts;
            bone.indices = indices;
            bone.binds = vec![];
            bone.blacklist = vec![];
            bone.set_editing_vertices(true);
            cleanup_vertices(bone);
            tabs[ui.sel_tab].selections.bind = -1;
        }
        Events::RenameTex => {
            let style = tabs[ui.sel_tab].sel_style_mut();
            if style == None {
                return;
            }

            let style = style.unwrap();
            let trimmed = str_value.trim_start().trim_end().to_string();
            let tex_names: Vec<String> = style.textures.iter().map(|t| t.name.clone()).collect();

            let filter = tex_names.iter().filter(|name| **name == trimmed);
            if filter.count() > 0 {
                // already 1 or more textures with this name - don't do anything
                open_modal(ui, false, ui.loc("styles_modal.same_name"));
            } else {
                let og_name = style.textures[value as usize].name.clone();
                style.textures[value as usize].name = trimmed.clone();

                if !config.keep_tex_str {
                    // change all bone tex names to point to the new one
                    for bone in &mut tabs[ui.sel_tab].armature.bones {
                        if bone.tex != "" && bone.tex == og_name {
                            bone.tex = trimmed.clone();
                        }
                    }
                }
            }

            ui.warnings = check_warnings(&tabs[ui.sel_tab].armature);
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
            ui.exporting_anims = vec![];
            for _ in &tabs[ui.sel_tab].armature.animations {
                ui.exporting_anims.push({
                    #[cfg(target_arch = "wasm32")]
                    {
                        // since web doesn't support multi-exports,
                        // don't select any anim
                        false
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        true
                    }
                });
            }
        }
        Events::UpdateConfig => *config = ui.updated_config.clone(),
        Events::CopySelectedKeyframes => {
            copy_selected_keyframes(copy_buffer, ui);
        }
        Events::SaveAnimation => {
            let anim = tabs[ui.sel_tab].selections.anim;
            tabs[ui.sel_tab].new_undo_anim(anim as usize);
        }
        Events::SetTemporaryEditMode => {
            if value == 3. {
                edit_mode.temporary = None;
            } else {
                edit_mode.temporary = EditModes::from_repr(value as usize);
            }
        }
        Events::ToggleEditModifying => edit_mode.holding_edit_mod = value == 1.,
        Events::ToggleEditSnapping => edit_mode.holding_edit_snap = value == 1.,
        Events::ToggleEditAlt => edit_mode.editing_pivot = value == 1.,
        Events::UpdateCurrentEditing => {
            if value == 1. {
                edit_mode.is_moving = false;
                edit_mode.is_rotating = false;
                edit_mode.is_scaling = false;
            } else {
                ui.cursor_icon = egui::CursorIcon::Crosshair;
                let current_edit = if let Some(temporary) = &edit_mode.temporary {
                    temporary
                } else {
                    &edit_mode.current
                };
                if *current_edit == EditModes::Other {
                    ui.cursor_icon = egui::CursorIcon::Default;
                }
                edit_mode.is_moving = *current_edit == EditModes::Move;
                edit_mode.is_rotating = *current_edit == EditModes::Rotate;
                edit_mode.is_scaling = *current_edit == EditModes::Scale;
            }
        }
        Events::RaiseGlobalZindex => {
            let bones = &tabs[ui.sel_tab].armature.bones;
            let zindex = bones.iter().find(|b| b.id == value as i32).unwrap().zindex;
            for bone in &mut tabs[ui.sel_tab].armature.bones {
                if bone.zindex > zindex && bone.tex != "" {
                    bone.zindex += 1;
                }
            }
            let bones = &mut tabs[ui.sel_tab].armature.bones;
            let bone = bones.iter_mut().find(|b| b.id == value as i32).unwrap();
            bone.zindex += 1;
        }
        Events::SetRotResistance => {
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.phys_sway = value;
            if value == 0. {
                let mut children = vec![];
                let bone = tabs[ui.sel_tab].sel_bone().unwrap();
                get_all_children(&tabs[ui.sel_tab].armature.bones, &mut children, bone);
                for child in children {
                    if child.parent_id != tabs[ui.sel_tab].sel_bone().unwrap().id {
                        continue;
                    }
                    let bone = tabs[ui.sel_tab]
                        .armature
                        .bones
                        .iter_mut()
                        .find(|b| b.id == child.id)
                        .unwrap();
                    bone.pos.y = 0.;
                }
            }
        }
        Events::SetPosDamping => {
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.phys_pos_damping = value;
        }
        Events::SetScaleDamping => {
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.phys_scale_damping = value;
        }
        Events::SetRotDamping => {
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.phys_rot_damping = value;
        }
        Events::SetRotBounce => {
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.phys_rot_bounce = (value).max(0.).min(1.);
        }
        Events::SetPosRatio => {
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.phys_pos_ratio = (value).max(-1.).min(1.);
        }
        Events::SetScaleRatio => {
            let bone = tabs[ui.sel_tab].sel_bone_mut().unwrap();
            bone.phys_scale_ratio = (value).max(-1.).min(1.);
        }
        Events::SetHoveringVertId => tabs[ui.sel_tab].selections.hovering_vert_id = value as i32,
        Events::SetHoveringTri => tabs[ui.sel_tab].selections.hovering_tri_dur = value as i32,
        Events::SetHoveringBoneId => tabs[ui.sel_tab].selections.hovering_bone_id = value as i32,
        Events::SetHoveringLine => tabs[ui.sel_tab].selections.hovering_line_dur = value as i32,
        Events::ImportPsdArmature => {
            tabs[ui.sel_tab].armature.bones = psd_armature.bones.clone();
            tabs[ui.sel_tab].armature.styles = psd_armature.styles.clone();
            tabs[ui.sel_tab].armature.tex_data = psd_armature.tex_data.clone();
            *psd_armature = Armature::default();
        }
        Events::CreateParentBone => {
            // either get the bone ID from event, or selected bones if more than 1
            let bones = &tabs[ui.sel_tab].armature.bones;
            let id = bones.iter().find(|b| b.id == value as i32).unwrap().id;
            let mut bones_to_copy = vec![id];

            if tabs[ui.sel_tab].selections.bone_ids.len() > 1 {
                bones_to_copy = tabs[ui.sel_tab].selections.bone_ids.clone();
            }

            let (parent, _) = tabs[ui.sel_tab].armature.new_bone(bones_to_copy[0]);

            // move new bone(s) above target
            drag_bone(
                &mut tabs[ui.sel_tab].armature,
                bones_to_copy[0],
                &vec![parent.id],
                true,
            );

            // set targets' parent as new bone
            for bone_id in &bones_to_copy {
                let bone = tabs[ui.sel_tab]
                    .armature
                    .bones
                    .iter()
                    .find(|b| b.id == *bone_id);
                if bone == None || bones_to_copy.contains(&bone.unwrap().parent_id) {
                    continue;
                }
                tabs[ui.sel_tab]
                    .armature
                    .find_bone_mut(*bone_id)
                    .unwrap()
                    .parent_id = parent.id;
            }

            // activate renaming for new bone
            tabs[ui.sel_tab]
                .armature
                .find_bone_mut(parent.id)
                .unwrap()
                .name = "".to_string();
            ui.rename_id = format!("bone_{}", parent.id);

            // mark this bone as selected, so focus isn't taken away from bone name input
            ui.prev_selected_bone_id = parent.id;

            // select new parent
            tabs[ui.sel_tab].selections.bone_ids = vec![parent.id];
        }
        Events::MoveSelectedKeyframes => {
            if value as i32 == ui.dragged_keyframe.frame
                || ui.dragged_keyframe.frame == -1
                || value as i32 == -1
            {
                ui.dragged_keyframe.frame = -1;
                return;
            }

            let anim = tabs[ui.sel_tab].selections.anim;
            tabs[ui.sel_tab].new_undo_anim(anim as usize);

            let sel_anim = &mut tabs[ui.sel_tab].sel_anim_mut().unwrap();

            for skf in &mut ui.selected_keyframes {
                // get difference between this frame and the frame being dragged
                let diff = skf.frame - ui.dragged_keyframe.frame;
                let new_frame = (value as i32 + diff).max(0);

                // remove keyframe that is the same as this
                if let Some(k) = sel_anim.keyframes.iter().position(|kf| {
                    kf.bone_id == skf.bone_id && kf.element == skf.element && kf.frame == new_frame
                }) {
                    sel_anim.keyframes.remove(k);
                }

                // set this keyframe's frame to the dropped one
                if let Some(idx) = sel_anim.keyframes.iter().position(|kf| kf == skf) {
                    sel_anim.keyframes[idx].frame = new_frame;
                    skf.frame = new_frame;
                }
            }

            sel_anim.sort_keyframes();
            ui.dragged_keyframe.frame = -1;
        }
        Events::GlobalCopy => match ui.last_selected.as_str() {
            "keyframe" => copy_selected_keyframes(copy_buffer, ui),
            "bone" => {
                let sbi = &tabs[ui.sel_tab].selections.bone_ids;
                let bone_id = if sbi.len() > 0 { sbi[0] } else { i32::MAX };
                copy_bone(copy_buffer, &mut tabs[ui.sel_tab], bone_id)
            }
            _ => {}
        },
        Events::GlobalPaste => match ui.last_selected.as_str() {
            "keyframe" => {
                let anim = tabs[ui.sel_tab].selections.anim;
                tabs[ui.sel_tab].new_undo_anim(anim as usize);
                let frame = tabs[ui.sel_tab].selections.anim_frame;
                paste_keyframes_on_frame(copy_buffer, &mut tabs[ui.sel_tab], frame);
            }
            "bone" => {
                tabs[ui.sel_tab].new_undo_bones();
                let sbi = &tabs[ui.sel_tab].selections.bone_ids;
                let bone_id = if sbi.len() > 0 { sbi[0] } else { i32::MAX };
                paste_bone(copy_buffer, &mut tabs[ui.sel_tab], bone_id);
            }
            _ => {}
        },
        Events::ToggleEditingPivot => edit_mode.editing_pivot = !edit_mode.editing_pivot,
        Events::ReduceGlobalIkFamilyIds => {
            for bone in &mut tabs[ui.sel_tab].armature.bones {
                if bone.ik_family_id > value as i32 {
                    bone.ik_family_id -= 1;
                }
            }
        }
        Events::RestartAnims => {
            for anim in &mut tabs[ui.sel_tab].armature.animations {
                if anim.elapsed != None {
                    anim.elapsed = Some(Instant::now());
                }
            }
        }
        Events::AdjustKeyframesBySpeed => {
            if value != 0. {
                for kfs in &mut tabs[ui.sel_tab].sel_anim_mut().unwrap().keyframes {
                    kfs.frame = (kfs.frame as f32 / value).round() as i32;
                }
            }
        }
        Events::CreateBindBone => {
            // create new bone
            let (_, new_idx) = tabs[ui.sel_tab].armature.new_bone(-1);
            let camera = tabs[ui.sel_tab].camera.clone();
            tabs[ui.sel_tab].armature.bones[new_idx].pos =
                utils::world_mouse(ui.context_menu.last_mouse_pos, &camera).1;

            // set new bone as bind
            let bone_id = tabs[ui.sel_tab].armature.bones[new_idx].id;
            let bind = BoneBind {
                bone_id,
                is_path: false,
                verts: vec![BoneBindVert {
                    id: value as i32,
                    weight: 1.,
                }],
            };

            tabs[ui.sel_tab].sel_bone_mut().unwrap().binds.push(bind);

            // add new bone to temp_bones for adjusting later
            let bone = tabs[ui.sel_tab].armature.bones.last().unwrap().clone();
            renderer.temp_arm.bones.push(bone);

            let sel_bone_id = tabs[ui.sel_tab].sel_bone().unwrap().id;

            // adjust vertex pos so it stays with new bind
            let binds = &tabs[ui.sel_tab].sel_bone().unwrap().binds;
            let bind = binds.last().unwrap().clone();
            let vertices = &mut tabs[ui.sel_tab].sel_bone_mut().unwrap().vertices;
            let vert_id = value as u32;
            let vert = &mut vertices.iter_mut().find(|v| v.id == vert_id).unwrap();
            let temp_bones = &renderer.temp_arm.bones;
            let temp_bone = temp_bones.iter().find(|b| b.id == sel_bone_id).unwrap();
            adjust_vert_by_bind(vert, true, &bind, &renderer.temp_arm.bones, temp_bone);

            // set bone as child of mesh
            let sel_bone = tabs[ui.sel_tab].selections.sel_bone();
            drag_bone(
                &mut tabs[ui.sel_tab].armature,
                sel_bone,
                &vec![bone_id],
                false,
            );
        }
        Events::NewBone => {
            // set target bone as either one selected via the event, or currently selected one
            let target_id = if value != -1. {
                value as i32
            } else if tabs[ui.sel_tab].sel_bone() != None {
                tabs[ui.sel_tab].sel_bone().unwrap().id
            } else {
                -1
            };

            // reset selected bones, so that only new bone will be selected later
            tabs[ui.sel_tab].selections.bone_ids = vec![];

            // create new bone
            let idx;
            (_, idx) = tabs[ui.sel_tab].armature.new_bone(target_id);
            tabs[ui.sel_tab].armature.bones[idx].name = "".to_string();
            let bone_id = tabs[ui.sel_tab].armature.bones[idx].id;

            // don't select new bone until user has done it at least once this session
            if ui.selected_bone_first_time {
                select_bone(&mut tabs[ui.sel_tab], ui, edit_mode, input, bone_id, false);
            }

            ui.rename_id = "bone_".to_string() + &bone_id.to_string();

            // mark this bone as selected, so focus isn't taken away from bone name input
            ui.prev_selected_bone_id = bone_id;
        }
        Events::SetArmatureSplit => edit_mode.armature_split = value,
        Events::SelectAllTexturedBones => {
            let bones = &tabs[ui.sel_tab].armature.bones;
            tabs[ui.sel_tab].selections.bone_ids =
                bones.iter().filter(|b| b.tex != "").map(|b| b.id).collect();
        }
        Events::SwitchToArmature => {
            switch_to_armature(value as usize, tabs, ui, &config, false);
        }
        Events::RemoveTab => {
            tabs.remove(value as usize);
            if ui.sel_tab > tabs.len() - 1 {
                switch_to_armature(ui.sel_tab - 1, tabs, ui, &config, true);
            }
            ui.changed_window_name = false;
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
    tab: &mut ArmatureTab,
    ui: &mut crate::Ui,
    edit_mode: &mut EditMode,
    input: &InputStates,
    id: i32,
    from_renderer: bool,
) {
    if ui.atlas_modal || ui.file_elapsed != None {
        return;
    }

    edit_mode.showing_mesh = false;
    edit_mode.editing_mesh = false;
    edit_mode.sel_time = 0.;
    edit_mode.temporary = None;
    tab.selections.vert_ids = vec![];
    ui.selected_bone_first_time = true;
    edit_mode.editing_pivot = false;
    tab.selections.style_id = -1;

    if id == -1 {
        tab.selections.bone_ids = vec![];
        tab.selections.bind = -1;
        edit_mode.setting_ik_target = false;
        edit_mode.setting_bind_bone = false;
        return;
    }

    // rename bone if already selected and in right-side panel
    let holding_mods = input.holding_mod || input.holding_shift;
    if tab.selections.sel_bone() == id as i32
        && !from_renderer
        && ui.last_selected == "bone"
        && !holding_mods
    {
        ui.rename_id = "bone_".to_string() + &tab.selections.bone_ids[0].to_string().clone();
        ui.edit_value = Some(tab.armature.sel_bone(&tab.selections).unwrap().name.clone());
        return;
    }

    ui.last_selected = "bone".to_string();

    // set this bone as IK target if in IK target mode
    if edit_mode.setting_ik_target {
        tab.armature
            .sel_bone_mut(&tab.selections)
            .unwrap()
            .ik_target_id = tab.armature.bones.iter().find(|b| b.id == id).unwrap().id;
        edit_mode.setting_ik_target = false;
        return;
    }

    // set this bone as bind if in bind mode
    if edit_mode.setting_bind_bone {
        let id = tab.armature.bones.iter().find(|b| b.id == id).unwrap().id;
        if let Some(bind) = tab
            .armature
            .sel_bone_mut(&tab.selections)
            .and_then(|bone| bone.binds.get_mut(tab.selections.bind as usize))
        {
            bind.bone_id = id;
        }
        edit_mode.setting_bind_bone = false;
        return;
    }

    tab.selections.bind = -1;
    let bone_id = tab.armature.bones.iter().find(|b| b.id == id).unwrap().id;
    // scroll to this bone in keyframe editor
    if let Some(bone) = ui.bone_tops.tops.iter().find(|b| b.id == bone_id) {
        ui.timeline_offset.y =
            bone.height + ui.timeline_offset.y - ui.keyframe_panel_rect.unwrap().top() - 47.;
    }

    // select only this bone if not holding modifiers
    if !holding_mods {
        tab.selections.bone_ids = vec![bone_id];
        edit_mode.showing_mesh = false;

        // unfold this bone's parents to reveal it in the hierarchy
        let parents = tab
            .armature
            .get_all_parents(false, tab.sel_bone().unwrap().id);
        let bones = &mut tab.armature.bones;
        for p in parents {
            bones.iter_mut().find(|b| b.id == p.id).unwrap().folded = false;
        }
    } else if tab.selections.bone_ids.contains(&bone_id) {
        // unselect bone if already part of selection
        tab.selections.bone_ids.retain(|id| *id != bone_id);
    } else {
        // add bone to selection (holding cmd/shift)
        if tab.selections.bone_ids.len() == 0 {
            tab.selections.bone_ids = vec![bone_id];
        }
        if input.holding_mod {
            let id = bone_id;
            tab.selections.bone_ids.push(id);
        } else {
            let bones = &tab.armature.bones;
            let mut first = bones
                .iter()
                .position(|b| b.id == tab.selections.sel_bone())
                .unwrap();
            let mut second = bones.iter().position(|b| b.id == id).unwrap();
            if first > second {
                first = bones.iter().position(|b| b.id == id).unwrap();
                second = bones
                    .iter()
                    .position(|b| b.id == tab.selections.sel_bone())
                    .unwrap();
            }
            for i in first..second as usize + 1 {
                let bone = &tab.armature.bones[i];
                tab.selections.bone_ids.push(bone.id);
            }
        }

        // sort bone IDs by their order in the hierarchy
        tab.selections.bone_ids.sort_by(|a, b| {
            let a_pos = tab
                .armature
                .bones
                .iter()
                .position(|bo| bo.id == *a)
                .unwrap();
            let b_pos = tab
                .armature
                .bones
                .iter()
                .position(|bo| bo.id == *b)
                .unwrap();
            a_pos.cmp(&b_pos)
        });

        // remove any duplicate IDs
        tab.selections.bone_ids.dedup();
    }
}

fn unselect_all(selections: &mut SelectionState, edit_mode: &mut EditMode, ui: &mut crate::Ui) {
    selections.bone_ids = vec![];
    selections.anim_frame = -1;
    selections.anim = usize::MAX;
    selections.bind = -1;
    selections.style_id = -1;
    edit_mode.showing_mesh = false;
    edit_mode.setting_ik_target = false;
    edit_mode.setting_bind_bone = false;
    edit_mode.editing_mesh = false;
    ui.last_selected = "".to_string();
}

pub fn undo_redo(undo: bool, tab: &mut ArmatureTab) {
    let action: Action;
    if undo {
        if tab.undo_states.undo_actions.last() == None {
            return;
        }
        action = tab.undo_states.undo_actions.last().unwrap().clone();
    } else {
        if tab.undo_states.redo_actions.last() == None {
            return;
        }
        action = tab.undo_states.redo_actions.last().unwrap().clone();
    }

    // store the state prior to undoing/redoing the action,
    // to add to the opposite stack later
    let mut new_action = action.clone();

    match &action.action {
        ActionType::Bone => {
            let bone = tab.armature.find_bone_mut(action.bones[0].id).unwrap();
            new_action.bones = vec![bone.clone()];
            *bone = action.bones[0].clone();
            // remove selected vertices that no longer exist
            let vert_ids: Vec<usize> = bone.vertices.iter().map(|v| v.id as usize).collect();
            tab.selections.vert_ids.retain(|id| vert_ids.contains(id));
        }
        ActionType::Bones => {
            new_action.bones = tab.armature.bones.clone();
            tab.armature.bones = action.bones.clone();
        }
        ActionType::Animation => {
            let anim_id = action.animations[0].id;
            let animations = &mut tab.armature.animations;
            let anim = animations.iter_mut().find(|a| a.id == anim_id).unwrap();
            new_action.animations = vec![anim.clone()];
            *anim = action.animations[0].clone();
        }
        ActionType::Animations => {
            new_action.animations = tab.armature.animations.clone();
            tab.armature.animations = action.animations.clone();
            let animations = &mut tab.armature.animations;
            if animations.len() == 0 || tab.selections.anim > animations.len() - 1 {
                tab.selections.anim = usize::MAX;
            }
        }
        ActionType::Style => {
            let id = action.styles[0].id;
            let style = tab.armature.styles.iter_mut().find(|a| a.id == id).unwrap();
            new_action.styles = vec![style.clone()];
            *style = action.styles[0].clone();
        }
        ActionType::Styles => {
            new_action.styles = tab.armature.styles.clone();
            tab.armature.styles = action.styles.clone();
            let style_ids: Vec<i32> = tab.armature.styles.iter().map(|s| s.id).collect();
            if !style_ids.contains(&tab.selections.style_id) {
                tab.selections.style_id = -1;
            }
        }
        ActionType::Armature => {
            new_action.armatures = vec![tab.armature.clone()];
            tab.armature = action.armatures[0].clone();
        }
        _ => {}
    }

    // add action(s) to opposing stack
    tab.undo_states.temp_actions.push(new_action);
    if undo {
        tab.undo_states.undo_actions.pop();
        if !action.continued {
            // reverse list to restore order of actions
            tab.undo_states.temp_actions.reverse();
            let temp_actions = &mut tab.undo_states.temp_actions;
            tab.undo_states.redo_actions.append(temp_actions);
            tab.undo_states.temp_actions = vec![];
        }
    } else {
        tab.undo_states.redo_actions.pop();
        if !action.continued {
            // ditto
            tab.undo_states.temp_actions.reverse();
            let temp_actions = &mut tab.undo_states.temp_actions;
            tab.undo_states.undo_actions.append(temp_actions);
            tab.undo_states.temp_actions = vec![];
        }
    }

    tab.undo_states.prev_undo_actions = tab.undo_states.undo_actions.len();
    tab.undo_states.unsaved_undo_actions = tab.undo_states.undo_actions.len();

    // actions tagged with `continue` are part of an action chain
    if action.continued {
        undo_redo(undo, tab);
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

pub fn drag_bone(armature: &mut Armature, pointing_id: i32, bone_ids: &Vec<i32>, is_above: bool) {
    if bone_ids.contains(&pointing_id) {
        return;
    }

    // ignore if pointing bone is a child of this
    if bone_ids.len() != 0 {
        let mut children: Vec<Bone> = vec![];
        let id = bone_ids[0];
        let dragged_bone = armature.bones.iter().find(|b| b.id == id).unwrap();
        let db = dragged_bone;
        armature_window::get_all_children(&armature.bones, &mut children, &db);
        let children_ids: Vec<i32> = children.iter().map(|c| c.id).collect();
        if children_ids.contains(&pointing_id) {
            return;
        }
    }

    let mut sorted_ids = bone_ids.clone();
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

        // adjust dragged bone so it stays in place
        armature.offset_pos_by_parent(old_parents, id);
    }
}

fn edit_bone(
    armature: &mut Armature,
    config: &Config,
    bone_id: i32,
    element: AnimElement,
    mut value: f32,
    value_str: String,
    mut anim_id: usize,
    mut anim_frame: i32,
) {
    let is_any_anim_playing = armature.is_any_anim_playing();
    let eff = armature.bone_eff(bone_id);
    let bones = &mut armature.bones;
    let bone = bones.iter_mut().find(|b| b.id == bone_id).unwrap();

    let mut init_value = 0.;
    let mut init_value_str = "".to_string();

    // set rotation to 0 if this bone is part of IK
    if bone.ik_family_id != -1 && eff != JointEffector::End && element == AnimElement::Rotation {
        value = 0.;
    }

    // prevent recording into animation if bone is locked
    if bone.locked {
        anim_id = usize::MAX;
        anim_frame = -1;
    }

    // do nothing if anim is playing and 'edit while playing' config is false
    if !config.edit_while_playing && is_any_anim_playing {
        return;
    }

    macro_rules! set {
        ($field:expr, $field_type:ident) => {{
            init_value = $field as f32;
            if anim_frame == -1 {
                $field = value as $field_type;
            }
        }};
    }
    macro_rules! set_str {
        ($field:expr, $enum:ident) => {{
            init_value_str = $field.to_string();
            if anim_frame == -1 {
                $field = $enum::from_str(&value_str).unwrap()
            }
        }};
    }
    macro_rules! set_bool {
        ($field:expr) => {{
            init_value = shared::bool_as_f32($field);
            if anim_frame == -1 {
                $field = shared::f32_as_bool(value)
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
        AnimElement::Hidden => set_bool!(bone.hidden),
        AnimElement::Locked => set_bool!(bone.locked),
        AnimElement::IkMode => set_str!(bone.ik_mode, InverseKinematicsMode),
        AnimElement::GroupColorR => set!(bone.group_color.r, u8),
        AnimElement::GroupColorG => set!(bone.group_color.g, u8),
        AnimElement::GroupColorB => set!(bone.group_color.b, u8),
        AnimElement::GroupColorA => set!(bone.group_color.a, u8),
        AnimElement::MimicTarget => set_bool!(bone.ik_mimic_target),
        AnimElement::PivotX => set!(bone.pivot_pos.x, f32),
        AnimElement::PivotY => set!(bone.pivot_pos.y, f32),
        AnimElement::PivotRot => set!(bone.pivot_rot, f32),
        AnimElement::PivotScaleX => set!(bone.pivot_scale.x, f32),
        AnimElement::PivotScaleY => set!(bone.pivot_scale.y, f32),
    };

    if anim_frame == -1 {
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
    let frame = anim[anim_id]
        .check_if_in_keyframe(bone_id, anim_frame, element.clone())
        .1;
    anim[anim_id].keyframes[frame].value = value;
    anim[anim_id].keyframes[frame].value_str = value_str;
}

// remove vertices that are not in any triangle
pub fn cleanup_vertices(bone: &mut Bone) {
    for v in (0..bone.vertices.len()).rev() {
        let vert = bone.vertices[v];
        if bone.indices.contains(&(v as u32)) {
            continue;
        }
        bone.vertices.remove(v);

        // removed vertex causes an offset by +1 for indices higher than itself,
        // so adjust indices to correct this
        for idx in &mut bone.indices {
            *idx -= if *idx >= v as u32 { 1 } else { 0 };
        }

        // remove this vertex from binds
        for bind in &mut bone.binds {
            let vert_id = vert.id as i32;
            let idx = bind.verts.iter().position(|v| v.id == vert_id);
            if idx != None {
                bind.verts.remove(idx.unwrap());
            }
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

    // get last point that current one has light of sight on.
    // if next point checked happens to be first and there's line of sight, tracing is over.
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

    sort_vertices(&mut verts);
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
pub fn sort_vertices(verts: &mut Vec<Vertex>) {
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
}

fn vert(pos: Option<Vec2>, col: Option<Color>, uv: Option<Vec2>) -> Vertex {
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

pub fn remove_blacklisted_tris(
    indices: &mut Vec<u32>,
    verts: &Vec<Vertex>,
    blacklist: &mut Vec<u32>,
) {
    // remove blacklists with vertices that don't exist (prevents lingering triangles)
    let ids: Vec<u32> = verts.iter().map(|v| v.id).collect();
    for (b, raw_bl_chunk) in blacklist.clone().chunks_exact_mut(3).enumerate().rev() {
        let mut chunk = vec![raw_bl_chunk[0], raw_bl_chunk[1], raw_bl_chunk[2]];
        chunk.sort();
        if !ids.contains(&chunk[0]) || !ids.contains(&chunk[1]) || !ids.contains(&chunk[2]) {
            blacklist.remove(b * 3);
            blacklist.remove(b * 3);
            blacklist.remove(b * 3);
        }
    }

    for (_, raw_bl_chunk) in blacklist.chunks_exact(3).enumerate() {
        let mut bl_chunk = vec![raw_bl_chunk[0], raw_bl_chunk[1], raw_bl_chunk[2]];
        bl_chunk.sort();
        for (ref mut i, raw_chunk) in indices.clone().chunks_exact(3).enumerate() {
            let mut chunk = vec![
                verts[raw_chunk[0] as usize].id,
                verts[raw_chunk[1] as usize].id,
                verts[raw_chunk[2] as usize].id,
            ];
            chunk.sort();
            if chunk == bl_chunk {
                indices.remove(*i * 3);
                indices.remove(*i * 3);
                indices.remove(*i * 3);
                break;
            }
        }
    }
}

pub fn copy_bone(copy_buffer: &mut CopyBuffer, tab: &mut ArmatureTab, bone_id: i32) {
    copy_buffer.bones = vec![];

    // ignore copying if there isn't a target bone
    if bone_id == i32::MAX {
        return;
    }

    // either get the bone ID from event, or selected bones if more than 1
    let mut bones_to_copy = vec![bone_id];
    if tab.selections.bone_ids.len() > 1 {
        bones_to_copy = tab.selections.bone_ids.clone();
    }

    // add appropriate bones to copy buffer
    for bone_id in &bones_to_copy {
        let bone = tab.armature.bones.iter().find(|b| b.id == *bone_id);
        let not_root = bones_to_copy.contains(&bone.unwrap().parent_id);
        if bone == None || not_root {
            continue;
        }
        let mut bones = vec![];
        armature_window::get_all_children(&tab.armature.bones, &mut bones, bone.unwrap());
        bones.insert(0, bone.unwrap().clone());
        copy_buffer.bones.extend_from_slice(&bones);
    }
}

fn paste_bone(copy_buffer: &mut CopyBuffer, tab: &mut ArmatureTab, pasted_id: i32) {
    // determine which id to give the new bone(s), based on the highest current id
    let ids: Vec<i32> = tab.armature.bones.iter().map(|bone| bone.id).collect();
    let mut highest_id = 0;
    for id in ids {
        highest_id = id.max(highest_id);
    }
    highest_id += 1;

    let mut insert_idx = usize::MAX;
    let mut id_refs: HashMap<i32, i32> = HashMap::new();

    let mut highest_ik_family_id = 0;
    for bone in &tab.armature.bones {
        highest_ik_family_id = bone.ik_family_id.max(highest_ik_family_id);
    }

    for b in 0..copy_buffer.bones.len() {
        let bone = &mut copy_buffer.bones[b];

        highest_id += 1;
        let new_id = highest_id;

        // put IK bones in a new family index
        id_refs.insert(bone.id, new_id);
        bone.id = highest_id;
        if bone.ik_family_id != -1 {
            bone.ik_family_id += highest_ik_family_id + 1;
        }

        // selected bone's parent is also pasted bone's
        if bone.parent_id != -1 && id_refs.get(&bone.parent_id) != None {
            bone.parent_id = *id_refs.get(&bone.parent_id).unwrap();
        } else if pasted_id != i32::MAX {
            let bones = &tab.armature.bones;
            insert_idx = bones.iter().position(|b| b.id == pasted_id).unwrap();
            bone.parent_id = tab.armature.bones[insert_idx].parent_id;
        } else {
            bone.parent_id = -1;
        }
    }

    // re-set binds that are pointing to child bones
    for b in 0..copy_buffer.bones.len() {
        let bone = &mut copy_buffer.bones[b];
        for bind in &mut bone.binds {
            if let Some(new_id) = id_refs.get(&bind.bone_id) {
                bind.bone_id = *new_id;
            }
        }
    }

    // insert pasted bones on proper position of the bone array
    if insert_idx == usize::MAX {
        tab.armature.bones.extend_from_slice(&copy_buffer.bones);
    } else {
        for bone in &copy_buffer.bones {
            tab.armature.bones.insert(insert_idx, bone.clone());
            insert_idx += 1;
        }
    }

    if id_refs.len() == 0 {
        return;
    }

    // select pasted bones
    tab.selections.bone_ids = id_refs.into_values().collect();
}

fn copy_selected_keyframes(copy_buffer: &mut CopyBuffer, ui: &mut crate::Ui) {
    *copy_buffer = CopyBuffer::default();
    copy_buffer.keyframes = ui.selected_keyframes.clone();
}

fn paste_keyframes_on_frame(copy_buffer: &mut CopyBuffer, tab: &mut ArmatureTab, frame: i32) {
    if frame < 0 || frame == i32::MAX || copy_buffer.keyframes.len() == 0 {
        return;
    }

    copy_buffer.keyframes.sort_by(|a, b| a.frame.cmp(&b.frame));
    let base_frame = copy_buffer.keyframes[0].frame;

    let mut buffer_frames = copy_buffer.keyframes.clone();
    let anim = &mut tab.armature.sel_anim_mut(&tab.selections).unwrap();

    // set copy buffer to new frames, for the retain() later
    for kf in &mut buffer_frames {
        let diff = kf.frame - base_frame;
        kf.frame = frame + diff;
    }

    // remove identical keyframes in the new frame
    anim.keyframes.retain(|kf| {
        buffer_frames
            .iter()
            .find(|bkf| bkf.frame == kf.frame && bkf.element == kf.element)
            == None
    });

    let base_frame = buffer_frames[0].frame;
    for k in 0..buffer_frames.len() {
        let keyframe = buffer_frames[k].clone();
        let diff = keyframe.frame - base_frame;
        anim.keyframes.push(Keyframe {
            frame: frame + diff,
            ..keyframe
        })
    }

    tab.armature
        .sel_anim_mut(&tab.selections)
        .unwrap()
        .sort_keyframes();
}

// remove invalid/duplicate keyframes
pub fn cleanup_keyframes(anims: &mut Vec<Animation>) {
    for anim in anims {
        anim.keyframes.dedup_by(|a, b| {
            a.bone_id == b.bone_id && a.frame == b.frame && a.element == b.element
        });
    }
}

pub fn adjust_vert_by_bind(
    vert: &mut Vertex,
    bound: bool,
    bind: &BoneBind,
    temp_bones: &Vec<Bone>,
    sel_temp_bone: &Bone,
) {
    // get the rotation to offset by, based on bind type (weight, path, etc)
    let bind_id = bind.bone_id;
    let bind_idx = sel_temp_bone
        .binds
        .iter()
        .position(|b| b.bone_id == bind_id);
    let bind_bone = temp_bones.iter().find(|b| b.id == bind.bone_id).unwrap();
    let rot = if bind.is_path {
        renderer::get_path_normal_angle(temp_bones, sel_temp_bone, bind_idx.unwrap())
    } else {
        bind_bone.rot
    };

    // offset vertex such that it stays still after binding/unbinding
    if bound {
        vert.pos /= bind_bone.scale / sel_temp_bone.scale;
        vert.pos = utils::rotate(&vert.pos, sel_temp_bone.rot);
        vert.pos -= (bind_bone.pos - sel_temp_bone.pos) / bind_bone.scale;
        vert.pos = utils::rotate(&vert.pos, -rot);
    } else {
        vert.pos = utils::rotate(&vert.pos, rot);
        vert.pos += (bind_bone.pos - sel_temp_bone.pos) / bind_bone.scale;
        vert.pos = utils::rotate(&vert.pos, -sel_temp_bone.rot);
        vert.pos *= bind_bone.scale / sel_temp_bone.scale;
    }
}

// move camera such that mouse is at the same world position
pub fn move_camera_with_zoom(camera: &mut Camera, input: &InputStates, old_zoom: f32) {
    let center = camera.window / 2.;
    let mouse_center = input.mouse - center;
    let diff = (camera.zoom - old_zoom) / camera.window.y / 0.5;
    camera.pos += Vec2::new(-mouse_center.x * diff, mouse_center.y * diff);
}

pub fn set_buffers_dirty(renderer: &mut Renderer, event: &crate::Events) {
    type E = Events;

    // update gridline buffer
    match event {
        E::Undo
        | E::Redo
        | E::CamZoomIn
        | E::CamZoomOut
        | E::CamZoomScroll
        | E::EditCamera
        | E::UpdateConfig
        | E::SelectBone
        | E::SwitchToArmature
        | E::RemoveTab
        | E::MoveTab => {
            renderer.gridline_buffer.dirty = true;
        }
        _ => {}
    }

    // update point and kite buffers
    //
    // note: these are also updated when animations are being played.
    // search ANIMATED_BUFFERS to find it in lib.rs
    match event {
        E::CamZoomIn
        | E::CamZoomOut
        | E::CamZoomScroll
        | E::EditCamera
        | E::NewBone
        | E::EditBone
        | E::DragBone
        | E::ToggleAnimPanelOpen
        | E::Undo
        | E::Redo
        | E::UpdateRenderOptions
        | E::UpdateConfig
        | E::SetHoveringBoneId
        | E::SelectAnim
        | E::SelectAnimFrame
        | E::SelectBone
        | E::DeleteBone
        | E::SwitchToArmature
        | E::RemoveTab
        | E::MoveTab => {
            renderer.point_buffer.dirty = true;
            renderer.kite_buffer.dirty = true;
            renderer.bone_buffer.dirty = true;
        }
        _ => {}
    }

    // update bone buffers (ANIMATED_BUFFERS applies here too)
    match event {
        E::CamZoomIn
        | E::CamZoomOut
        | E::CamZoomScroll
        | E::EditCamera
        | E::NewBone
        | E::EditBone
        | E::DragBone
        | E::ToggleAnimPanelOpen
        | E::Undo
        | E::Redo
        | E::UpdateRenderOptions
        | E::UpdateConfig
        | E::SelectBone
        | E::SetHoveringBoneId
        | E::SwitchToArmature
        | E::RemoveTab
        | E::MoveTab => {
            renderer.bone_buffer.dirty = true;
        }
        _ => {}
    }
}

pub fn switch_to_armature(
    new_sel_tab: usize,
    tabs: &mut Vec<ArmatureTab>,
    ui: &mut crate::Ui,
    config: &Config,
    deleted: bool,
) {
    if new_sel_tab == ui.sel_tab || new_sel_tab > tabs.len() - 1 {
        return;
    }

    let prev_tab = ui.sel_tab;
    ui.sel_tab = new_sel_tab;

    if config.global_camera && !deleted {
        tabs[ui.sel_tab].camera = tabs[prev_tab].camera.clone();
    }

    ui.warnings = check_warnings(&tabs[new_sel_tab].armature);
    ui.changed_window_name = false;
}
