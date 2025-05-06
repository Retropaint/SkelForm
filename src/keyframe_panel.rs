use crate::*;

pub fn draw(ui: &mut egui::Ui, shared: &mut Shared) {
    ui.heading("Keyframe");

    ui.horizontal(|ui| {
        ui.label("Transition:");

        if shared.selected_keyframe() == None || shared.selected_animation().keyframes.len() == 0 {
            return;
        }

        let mut transition = shared.selected_keyframe_mut().unwrap().transition.clone();
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
            for kf in &mut shared.selected_animation_mut().keyframes {
                if kf.frame == selected_frame {
                    kf.transition = transition.clone();
                }
            }
        }
    });
}
