use ui::EguiUi;

use egui::{
    epaint::{self},
    pos2, Color32, Pos2, Rect, Sense, Shape, Stroke, StrokeKind,
};

use crate::*;

pub fn draw(
    ui: &mut egui::Ui,
    selections: &SelectionState,
    armature: &Armature,
    events: &mut EventState,
    shared_ui: &mut crate::Ui,
    config: &Config,
) {
    let str_header = shared_ui.loc("keyframe_panel.header");
    ui.heading(format!("{} ({})", str_header, selections.anim_frame));
    let sel = selections.clone();

    #[allow(unreachable_code)]
    let keyframes = &armature.sel_anim(&sel).unwrap().keyframes;
    let frame = selections.anim_frame;
    let keyframes_in_frame = keyframes.iter().filter(|a| a.frame == frame);

    // show keyframe editor shortcuts if there's no keyframe here
    if keyframes_in_frame.count() == 0 {
        ui.add_space(10.);
        #[rustfmt::skip]
        {
            ui::keyboard_shortcut(ui, shared_ui.loc("settings_modal.keyboard.prev_anim_frame"), config.keys.prev_anim_frame);
            ui::keyboard_shortcut(ui, shared_ui.loc("settings_modal.keyboard.next_anim_frame"), config.keys.next_anim_frame);
            ui::keyboard_shortcut(ui, shared_ui.loc("settings_modal.keyboard.next_keyframe"), config.keys.prev_keyframe);
            ui::keyboard_shortcut(ui, shared_ui.loc("settings_modal.keyboard.prev_keyframe"), config.keys.next_keyframe);
            ui::keyboard_shortcut(ui, shared_ui.loc("settings_modal.keyboard.timeline_zoom_mode"), config.keys.timeline_zoom_mode);
        };
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
    if keyframe.frame == 0 {
        ui.label(shared_ui.loc("keyframe_panel.empty"));
        return;
    }

    ui.horizontal(|ui| {
        if selections.anim_frame == -1
            || armature.sel_anim(&sel) == None
            || armature.sel_anim(&sel).unwrap().keyframes.len() == 0
        {
            return;
        }

        ui.label(shared_ui.loc("keyframe_panel.preset_label"));

        let mut sel = keyframe.handle_preset.clone();
        let og_selected = sel.clone();
        macro_rules! preset {
            ($name:expr) => {
                shared_ui.loc(&format!("keyframe_panel.presets.{}", $name))
            };
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            egui::ComboBox::new("transition".to_string(), "")
                .selected_text(preset!(keyframe.handle_preset))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut sel, HandlePreset::Linear, preset!("Linear"));
                    ui.selectable_value(&mut sel, HandlePreset::SineIn, preset!("SineIn"));
                    ui.selectable_value(&mut sel, HandlePreset::SineOut, preset!("SineOut"));
                    ui.selectable_value(&mut sel, HandlePreset::SineInOut, preset!("SineInOut"));
                    ui.selectable_value(&mut sel, HandlePreset::Snap, preset!("Snap"));
                });

            // update keyframe preset
            if sel != og_selected {
                let (start, end) = utils::interp_preset(sel.clone());
                let sel = sel.clone() as i32;
                events.update_keyframe_transition(keyframe.frame, true, start, sel);
                events.update_keyframe_transition(keyframe.frame, false, end, sel as i32);
            }
        });
    });

    // render bezier curve
    let bezier_frame = egui::Frame::new().fill(config.colors.dark_accent.into());
    let og_start_handle = keyframe.start_handle;
    let og_end_handle = keyframe.end_handle;
    ui.add_enabled_ui(keyframe.handle_preset != HandlePreset::Snap, |ui| {
        let rect = Rect::from_min_size(ui.cursor().min, egui::Vec2::splat(ui.available_width()));
        bezier_frame.show(ui, |ui| {
            bezier_graph(ui, &mut keyframe, rect, config);
        });
    });

    ui.add_space(ui.available_width());

    if og_start_handle != keyframe.start_handle || og_end_handle != keyframe.end_handle {
        // process bezier dragging
        if !shared_ui.started_edit_dragging {
            events.save_animation();
            shared_ui.started_edit_dragging = true;
        }
        events.update_keyframe_transition(frame, true, keyframe.start_handle, -1);
        events.update_keyframe_transition(frame, false, keyframe.end_handle, -1);
    }

    macro_rules! handle_input {
        ($id:expr, $handle:expr, $is_x:expr, $is_in:expr, $ui:expr) => {
            let value_to_change = if $is_x { $handle.x } else { $handle.y };
            let (edited, value, _) =
                $ui.float_input($id.to_string(), shared_ui, value_to_change, 1., None);
            let new_value = if $is_x {
                Vec2::new(value, $handle.y)
            } else {
                Vec2::new($handle.x, value)
            };
            if edited {
                events.save_animation();
                events.update_keyframe_transition(frame, $is_in, new_value, -1);
            }
            $ui.label(if $is_x { "X:" } else { "Y:" });
        };
    }

    // handle input fields
    let start_handle = keyframe.start_handle;
    let end_handle = keyframe.end_handle;
    ui.add_enabled_ui(keyframe.handle_preset != HandlePreset::Snap, |ui| {
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("keyframe_panel.start"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                handle_input!("start_handle_y", start_handle, false, true, ui);
                handle_input!("start_handle_x", start_handle, true, true, ui);
            });
        });
        ui.horizontal(|ui| {
            ui.label(shared_ui.loc("keyframe_panel.end"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                handle_input!("end_handle_y", end_handle, false, false, ui);
                handle_input!("end_handle_x", end_handle, true, false, ui);
            });
        });
    });

    // show snap note if that's the selected preset
    if keyframe.handle_preset == HandlePreset::Snap {
        ui.add_space(10.);
        ui.label(shared_ui.loc("keyframe_panel.snap_note"));
    }
}

pub fn bezier_graph(
    ui: &mut egui::Ui,
    keyframe: &mut Keyframe,
    rect: Rect,
    config: &Config,
) -> bool {
    let size = rect.size();
    let mut dragged = false;

    // used to translate positions from a rect at origin, to the requested rect
    let to_screen = egui::emath::RectTransform::from_to(
        egui::Rect::from_min_size(Pos2::ZERO, rect.size()),
        rect,
    );

    let start_handle = keyframe.start_handle;
    let end_handle = keyframe.end_handle;
    let mut control_points = [
        pos2(0.0, size.y),
        pos2(start_handle.x * size.x, size.y - (start_handle.y * size.y)),
        pos2(end_handle.x * size.x, size.y - (end_handle.y * size.y)),
        pos2(size.x, 0.0),
    ];

    let radius = 8.0;
    let mut col_prev = Color32::from_rgb(200, 25, 25);
    let mut col_next = Color32::from_rgb(25, 100, 200);
    for p in 0..control_points.len() {
        let point = &mut control_points[p];
        if p == 0 || p == 3 {
            continue;
        }

        let size = egui::Vec2::splat(2.0 * radius);
        let point_in_screen = to_screen.transform_pos(*point);
        let point_rect = Rect::from_center_size(point_in_screen, size);
        let point_response = ui.interact(
            point_rect,
            egui::Id::new(format!("{}p{}", rect.left(), p)),
            Sense::drag(),
        );

        *point += point_response.drag_delta();
        *point = to_screen.from().clamp(*point);

        if point_response.dragged() {
            dragged = true;
        }

        if point_response.contains_pointer() {
            let col = if p == 1 { &mut col_prev } else { &mut col_next };
            *col = *col + Color32::from_rgb(40, 40, 40);
        }
    }

    // draw rect
    ui.painter()
        .rect_filled(rect, 0.0, config.colors.dark_accent);

    let screen_points: Vec<Pos2> = control_points
        .iter()
        .take(4)
        .map(|p| to_screen * *p)
        .collect();

    // draw bezier curve
    ui.painter()
        .add(egui::epaint::CubicBezierShape::from_points_stroke(
            screen_points.clone().try_into().unwrap(),
            false,
            egui::Color32::default(),
            Stroke::new(1.0, Color32::from_rgb(25, 200, 100)),
        ));

    // draw lines
    let p0p1: Vec<Pos2> = screen_points[0..=1].into();
    let p2p3: Vec<Pos2> = screen_points[2..=3].into();
    ui.painter().line(p0p1, Stroke::new(1., col_prev));
    ui.painter().line(p2p3, Stroke::new(1., col_next));

    // draw circles
    ui.painter()
        .circle_filled(screen_points[1], radius, col_prev);
    ui.painter()
        .circle_filled(screen_points[2], radius, col_next);

    if keyframe.handle_preset != HandlePreset::Snap {
        let cp = &control_points;
        let p1 = Vec2::new(cp[1].x / size.x, 1. - (cp[1].y / size.y));
        let p2 = Vec2::new(cp[2].x / size.x, 1. - (cp[2].y / size.y));
        keyframe.start_handle = p1;
        keyframe.end_handle = p2;
    }

    dragged
}
