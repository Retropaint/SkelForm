use crate::*;

pub fn draw(ui: &mut egui::Ui, shared: &mut Shared) {
    ui.heading("Keyframe");

    ui.horizontal(|ui| {
        ui.label("Transition:");

        if shared.selected_keyframe() == None
            || shared.selected_keyframe().unwrap().bones.len() == 0
        {
            return;
        }

        let mut transition = shared.selected_keyframe_mut().unwrap().bones[0].fields[0]
            .transition
            .clone();
        let og_transition = transition.clone();

        egui::ComboBox::new("transition_dropdown".to_string(), "")
            .selected_text(transition.to_string())
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut transition, Transition::Linear, "Linear");
                ui.selectable_value(&mut transition, Transition::Sine, "Sine");
            })
            .response;

        // change all fields to use new transition
        if og_transition != transition {
            for bone in &mut shared.selected_keyframe_mut().unwrap().bones {
                for field in &mut bone.fields {
                    field.transition = transition.clone();
                }
            }
        }
    });
}
