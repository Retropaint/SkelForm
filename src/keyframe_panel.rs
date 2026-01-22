use crate::*;

pub fn draw(ui: &mut egui::Ui, shared: &mut Shared) {
    ui.heading("Keyframe (".to_owned() + &shared.selections.anim_frame.to_string() + ")");
    let sel = shared.selections.clone();

    return;

    #[allow(unreachable_code)]
    let keyframes_in_frame = shared
        .armature
        .sel_anim(&sel)
        .unwrap()
        .keyframes
        .iter()
        .filter(|a| a.frame == shared.selections.anim_frame);

    if keyframes_in_frame.count() == 0 {
        return;
    }

    ui.horizontal(|ui| {
        ui.label("Transition:");

        if shared.selections.anim_frame == -1
            || shared.armature.sel_anim(&sel) == None
            || shared.armature.sel_anim(&sel).unwrap().keyframes.len() == 0
        {
            return;
        }

        let mut transition = Transition::default();
        let frame = shared.selections.anim_frame;
        for kf in &mut shared.armature.sel_anim_mut(&sel).unwrap().keyframes {
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
            let selected_frame = shared.selections.anim_frame;
            for kf in &mut shared.armature.sel_anim_mut(&sel).unwrap().keyframes {
                if kf.frame == selected_frame {
                    kf.transition = transition.clone();
                }
            }
        }
    });
}
