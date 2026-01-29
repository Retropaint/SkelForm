use crate::*;

pub fn draw(
    ui: &mut egui::Ui,
    selections: &SelectionState,
    armature: &Armature,
    events: &mut EventState,
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

    ui.horizontal(|ui| {
        ui.label("Transition:");

        if selections.anim_frame == -1
            || armature.sel_anim(&sel) == None
            || armature.sel_anim(&sel).unwrap().keyframes.len() == 0
        {
            return;
        }

        let mut transition = Transition::default();
        let frame = selections.anim_frame;
        for kf in &armature.sel_anim(&sel).unwrap().keyframes {
            if kf.frame == frame {
                transition = kf.transition.clone();
                break;
            }
        }

        let og_transition = transition.clone();

        macro_rules! transition {
            ($transition:expr, $ui:expr) => {
                $ui.selectable_value(&mut transition, $transition, $transition.to_string());
            };
        }

        egui::ComboBox::new("transition_dropdown".to_string(), "")
            .selected_text(transition.to_string())
            .show_ui(ui, |ui| {
                transition!(Transition::Linear, ui);
                transition!(Transition::SineIn, ui);
                transition!(Transition::SineOut, ui);
            })
            .response;

        // change all fields to use new transition
        if og_transition != transition {
            let selected_frame = selections.anim_frame;
            events.set_keyframe_transition(selected_frame as usize, transition);
        }
    });
}
