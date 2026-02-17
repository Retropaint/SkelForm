use std::ops::SubAssign;

use egui::IntoAtoms;
use ui::EguiUi;

use crate::*;

pub fn draw(
    ui: &mut egui::Ui,
    selections: &SelectionState,
    armature: &Armature,
    events: &mut EventState,
    shared_ui: &mut crate::Ui,
) {
    ui.heading("Keyframe (".to_owned() + &selections.anim_frame.to_string() + ")");
    let sel = selections.clone();

    #[allow(unreachable_code)]
    let keyframes = &armature.sel_anim(&sel).unwrap().keyframes;
    let frame = selections.anim_frame;
    let keyframes_in_frame = keyframes.iter().filter(|a| a.frame == frame);

    if keyframes_in_frame.count() == 0 {
        return;
    }

    let mut keyframe = Keyframe::default();
    let mut idx = usize::MAX;
    for k in 0..keyframes.len() {
        let kf = &keyframes[k];
        if kf.frame == frame {
            keyframe = kf.clone();
            idx = k;
            break;
        }
    }

    ui.horizontal(|ui| {
        ui.label("Transition:");

        if selections.anim_frame == -1
            || armature.sel_anim(&sel) == None
            || armature.sel_anim(&sel).unwrap().keyframes.len() == 0
        {
            return;
        }

        let mut selected = -1;
        egui::ComboBox::new("transition".to_string(), "").show_ui(ui, |ui| {
            ui.selectable_value(&mut selected, 0, "Linear");
            ui.selectable_value(&mut selected, 1, "Sine In");
            ui.selectable_value(&mut selected, 2, "Sine Out");
        });

        let mut prev_frame = keyframe.frame;
        if prev_frame > 0 {
            prev_frame -= 1;
        }
        let prev = utils::get_prev_frame(
            prev_frame,
            &armature.sel_anim(&sel).unwrap().keyframes,
            keyframe.bone_id,
            &keyframe.element,
        );

        let d_value = keyframe.value as f32 - keyframes[prev].value as f32;
        let duration = keyframe.frame as f32 - keyframes[prev].frame as f32;

        match selected {
            0 => {
                events.update_keyframe_transition(idx, true, d_value / duration);
                events.update_keyframe_transition(idx, false, d_value / duration);
            }
            1 => {
                events.update_keyframe_transition(idx, true, d_value / duration);
                events.update_keyframe_transition(idx, false, 0.);
            }
            2 => {
                events.update_keyframe_transition(idx, true, 0.);
                events.update_keyframe_transition(idx, false, d_value / duration);
            }
            _ => {}
        }
    });

    ui.horizontal(|ui| {
        let id = "trans_in".to_string();
        let (edited, value, _) = ui.float_input(id, shared_ui, keyframe.in_handle, 1., None);
        if edited {
            events.update_keyframe_transition(idx, true, value);
        }
        let id = "trans_out".to_string();
        let (edited, value, _) = ui.float_input(id, shared_ui, keyframe.out_handle, 1., None);
        if edited {
            events.update_keyframe_transition(idx, false, value);
        }
    });
}
