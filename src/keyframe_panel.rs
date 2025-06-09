use crate::*;

pub fn draw(ui: &mut egui::Ui, shared: &mut Shared) {
    ui.heading("Keyframe (".to_owned() + &shared.ui.anim.selected_frame.to_string() + ")");

    ui.horizontal(|ui| {
        ui.label("Transition:");

        if shared.ui.anim.selected_frame == -1
            || shared.selected_animation() == None
            || shared.selected_animation().unwrap().keyframes.len() == 0
        {
            return;
        }

        let mut transition = Transition::default();
        let frame = shared.ui.anim.selected_frame;
        for kf in &mut shared.selected_animation_mut().unwrap().keyframes {
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
            let selected_frame = shared.ui.anim.selected_frame;
            for kf in &mut shared.selected_animation_mut().unwrap().keyframes {
                if kf.frame == selected_frame {
                    kf.transition = transition.clone();
                }
            }
        }
    });
}
