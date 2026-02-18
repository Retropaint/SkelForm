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
    for k in 0..keyframes.len() {
        let kf = &keyframes[k];
        if kf.frame == frame {
            keyframe = kf.clone();
            break;
        }
    }

    ui.add_space(10.);

    ui.add_enabled_ui(keyframe.frame != 0, |ui| {
        ui.horizontal(|ui| {
            if selections.anim_frame == -1
                || armature.sel_anim(&sel) == None
                || armature.sel_anim(&sel).unwrap().keyframes.len() == 0
            {
                return;
            }

            let mut selected = -1;
            egui::ComboBox::new("transition".to_string(), "")
                .selected_text("Transition Presets")
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, 0, "Linear");
                    ui.selectable_value(&mut selected, 1, "Sine In");
                    ui.selectable_value(&mut selected, 2, "Sine Out");
                    ui.selectable_value(&mut selected, 3, "Sine In-Out");
                    ui.selectable_value(&mut selected, 4, "None");
                });

            match selected {
                0 => {
                    events.update_keyframe_transition(keyframe.frame, true, 1. / 3.);
                    events.update_keyframe_transition(keyframe.frame, false, 2. / 3.);
                }
                1 => {
                    events.update_keyframe_transition(keyframe.frame, true, 0.);
                    events.update_keyframe_transition(keyframe.frame, false, 2. / 3.);
                }
                2 => {
                    events.update_keyframe_transition(keyframe.frame, true, 1.);
                    events.update_keyframe_transition(keyframe.frame, false, 1. / 3.);
                }
                3 => {
                    events.update_keyframe_transition(keyframe.frame, true, 0.);
                    events.update_keyframe_transition(keyframe.frame, false, 1.);
                }
                4 => {
                    events.update_keyframe_transition(keyframe.frame, true, 999.);
                    events.update_keyframe_transition(keyframe.frame, false, 999.);
                }
                _ => {}
            }
        });

        ui.horizontal(|ui| {
            ui.label("Start Handle: ");
            let id = "trans_in".to_string();
            let (edited, value, _) = ui.float_input(id, shared_ui, keyframe.start_handle, 1., None);
            if edited {
                events.update_keyframe_transition(keyframe.frame, true, value);
            }
        });

        ui.horizontal(|ui| {
            ui.label("End Handle: ");
            let id = "trans_out".to_string();
            let (edited, value, _) = ui.float_input(id, shared_ui, keyframe.end_handle, 1., None);
            if edited {
                events.update_keyframe_transition(keyframe.frame, false, value);
            }
        });
    });
}
