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

        let mut selected = keyframe.handle_preset;
        let og_selected = selected.clone();
        egui::ComboBox::new("transition".to_string(), "")
            .selected_text(shared_ui.loc("keyframe_panel.transition_presets"))
            .show_ui(ui, |ui| {
                macro_rules! preset {
                    ($name:expr) => {
                        shared_ui.loc(&("keyframe_panel.presets.".to_owned() + $name))
                    };
                }
                ui.selectable_value(&mut selected, HandlePreset::Linear, preset!("linear"));
                ui.selectable_value(&mut selected, HandlePreset::SineIn, preset!("sinein"));
                ui.selectable_value(&mut selected, HandlePreset::SineOut, preset!("sineout"));
                ui.selectable_value(&mut selected, HandlePreset::SineInOut, preset!("sineinout"));
                ui.selectable_value(&mut selected, HandlePreset::None, preset!("none"));
            });

        if selected != og_selected {
            let (start, end) = utils::interp_preset(selected.clone());
            events.update_keyframe_transition(keyframe.frame, true, start, selected.clone() as i32);
            events.update_keyframe_transition(keyframe.frame, false, end, selected as i32);
        }
    });

    // render bezier curve
    let bezier_frame = egui::Frame::new().fill(config.colors.dark_accent.into());
    let mut dragged = false;
    bezier_frame.show(ui, |ui| {
        dragged = bezier_graph(ui, &mut keyframe.start_handle, &mut keyframe.end_handle);
    });
    if !dragged {
        shared_ui.dragging_handles = false;
    } else {
        if !shared_ui.dragging_handles {
            events.save_animation();
            shared_ui.dragging_handles = true;
        }
        events.update_keyframe_transition(frame, true, keyframe.start_handle, -1);
        events.update_keyframe_transition(frame, false, keyframe.end_handle, -1);
    }

    macro_rules! handle_input {
        ($id:expr, $handle:expr, $is_x:expr, $ui:expr) => {
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
                events.update_keyframe_transition(frame, true, new_value, -1);
            }
            $ui.label(if $is_x { "X:" } else { "Y:" });
        };
    }

    // handle input fields
    let start_handle = keyframe.start_handle;
    let end_handle = keyframe.end_handle;
    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("keyframe_panel.start"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            handle_input!("start_handle_y", start_handle, false, ui);
            handle_input!("start_handle_x", start_handle, true, ui);
        });
    });
    ui.horizontal(|ui| {
        ui.label(shared_ui.loc("keyframe_panel.end"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            handle_input!("end_handle_y", end_handle, false, ui);
            handle_input!("end_handle_x", end_handle, true, ui);
        });
    });
}

pub fn bezier_graph(ui: &mut egui::Ui, start_handle: &mut Vec2, end_handle: &mut Vec2) -> bool {
    let size = egui::Vec2::new(ui.available_width(), ui.available_width());
    let (response, painter) = ui.allocate_painter(size, Sense::hover());
    let mut dragged = false;

    let to_screen = egui::emath::RectTransform::from_to(
        egui::Rect::from_min_size(Pos2::ZERO, response.rect.size()),
        response.rect,
    );

    let col_prev = Color32::from_rgb(200, 25, 25);
    let col_next = Color32::from_rgb(25, 100, 200);

    let control_point_radius = 8.0;

    let mut control_points = [
        pos2(0.0, size.y),
        pos2(start_handle.x * size.x, size.y - (start_handle.y * size.y)),
        pos2(end_handle.x * size.x, size.y - (end_handle.y * size.y)),
        pos2(size.x, 0.0),
    ];

    let control_point_shapes: Vec<Shape> = control_points
        .iter_mut()
        .enumerate()
        .map(|(i, point)| {
            if i == 0 || i == 3 {
                return Shape::circle_stroke([-100., -100.].into(), 0., egui::Stroke::default());
            }

            let size = egui::Vec2::splat(2.0 * control_point_radius);
            let point_in_screen = to_screen.transform_pos(*point);
            let point_rect = Rect::from_center_size(point_in_screen, size);
            let point_response = ui.interact(point_rect, response.id.with(i), Sense::drag());

            *point += point_response.drag_delta();
            *point = to_screen.from().clamp(*point);

            if point_response.dragged() {
                dragged = true;
            }

            let mut col = if i == 1 { col_prev } else { col_next };
            if point_response.contains_pointer() {
                col = col + Color32::from_rgb(40, 40, 40);
            }
            Shape::circle_filled(to_screen.transform_pos(*point), control_point_radius, col)
        })
        .collect();

    let points_in_screen: Vec<Pos2> = control_points
        .iter()
        .take(4)
        .map(|p| to_screen * *p)
        .collect();

    let bounding_box_stroke = Stroke::new(0.0, Color32::LIGHT_GREEN.linear_multiply(0.25));
    let stroke = Stroke::new(1.0, Color32::from_rgb(25, 200, 100));
    let points = points_in_screen.clone().try_into().unwrap();
    let empty = egui::Color32::default();
    let shape = egui::epaint::CubicBezierShape::from_points_stroke(points, false, empty, stroke);
    #[rustfmt::skip]
    painter.add(epaint::RectShape::stroke(shape.visual_bounding_rect(), 0.0, bounding_box_stroke, StrokeKind::Outside));
    painter.add(shape);

    let points = control_points;
    let p1 = Vec2::new(points[1].x / size.x, 1. - (points[1].y / size.y));
    let p2 = Vec2::new(points[2].x / size.x, 1. - (points[2].y / size.y));
    *start_handle = p1;
    *end_handle = p2;

    let prev = Stroke::new(1., col_prev);
    let next = Stroke::new(1., col_next);
    let cp = &control_points;
    let p0p1: Vec<Pos2> = cp[0..=1].iter().map(|p| to_screen * *p).collect();
    let p2p3: Vec<Pos2> = cp[2..=3].iter().map(|p| to_screen * *p).collect();
    painter.add(egui::epaint::PathShape::line(p0p1, prev));
    painter.add(egui::epaint::PathShape::line(p2p3, next));
    painter.extend(control_point_shapes);
    dragged
}
