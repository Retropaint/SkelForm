use crate::ui::EguiUi;
use crate::*;

#[cfg(not(target_arch = "wasm32"))]
pub fn startup_modal(shared: &mut Shared, ctx: &egui::Context) {
    egui::Window::new("startup")
        .title_bar(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.gradient(
                ui.ctx().screen_rect(),
                egui::Color32::TRANSPARENT,
                shared.config.colors.gradient.into(),
            );
            let width = ui.ctx().screen_rect().width();
            let height = ui.ctx().screen_rect().height();
            ui.set_width(width.max(0.));
            ui.set_height(height.max(0.));

            let available_size = ui.available_size();

            ui.horizontal(|ui| {
                ui.set_width(available_size.x);
                ui.set_height(available_size.y);
                startup_content(&ctx, ui, shared, available_size);
            });
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn startup_content(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    shared: &mut Shared,
    available_size: egui::Vec2,
) {
    ui.add_space(10.);

    let padding = 5.;

    ui.vertical(|ui| {
        ui.set_width(133.);
        ui.add_space(10.);
        if startup_leftside_button("+", shared.loc("startup.new"), ui, shared, None, None).clicked()
        {
            shared.armature = Armature::default();
            shared.ui.set_state(UiState::StartupWindow, false);
        }
        ui.add_space(padding);
        let import_pos = Some(egui::Vec2::new(-5., 2.5));
        if startup_leftside_button(
            "🗋",
            shared.loc("startup.import"),
            ui,
            shared,
            import_pos,
            None,
        )
        .clicked()
        {
            utils::open_import_dialog(shared.temp_path.import.clone());
        }
        ui.add_space(padding);
        let samples_pos = Some(egui::Vec2::new(-5., 2.5));
        if startup_leftside_button(
            "🗊",
            shared.loc("startup.samples"),
            ui,
            shared,
            samples_pos,
            None,
        )
        .clicked()
        {
            shared.ui.showing_samples = !shared.ui.showing_samples;
        }
        ui.add_space(padding);
        if shared.ui.showing_samples {
            let skellington_pos = Some(egui::Vec2::new(-5., -10.));
            if !shared.thumb_ui_tex.contains_key("skellington_sample.png") {
                shared.thumb_ui_tex.insert(
                    "skellington_sample.png".to_string(),
                    ui::create_ui_texture(
                        include_bytes!("../skellington_sample.png").to_vec(),
                        true,
                        ctx,
                    )
                    .unwrap(),
                );
            }
            if startup_leftside_button(
                "",
                "Skellington",
                ui,
                shared,
                skellington_pos,
                shared.thumb_ui_tex.get("skellington_sample.png"),
            )
            .clicked()
            {
                file_reader::create_temp_file(
                    &shared.temp_path.import,
                    "./samples/skellington.skf",
                );
                shared.ui.set_state(UiState::StartupWindow, false);
            }
        }
    });

    ui.add_space(10.);
    ui.separator();

    ui.vertical(|ui| {
        let reserved_for_resources = 450.;
        ui.set_width((available_size.x - reserved_for_resources).max(1.));
        let width = ui.available_width();
        ui.with_layout(
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            |ui| {
                let max_width = 600.;
                let right_margin = 50.;
                ui.set_max_width((width.min(max_width) - right_margin).max(1.));
                ui.set_min_width(0.);
                ui.vertical(|ui| {
                    let available_width = ui.available_width();
                    ui.set_width(available_width);
                    if shared.recent_file_paths.len() == 0 {
                        ui.add_space(10.);
                        let msg = shared.loc("startup.empty_recent_files");
                        let text = egui::RichText::new(msg).size(14.);
                        ui.label(text);
                        return;
                    }

                    for p in 0..shared.recent_file_paths.len() {
                        // safeguard for deleting a path during iteration
                        if p > shared.recent_file_paths.len() - 1 {
                            break;
                        }

                        let path = shared.recent_file_paths[p].to_string();
                        if let Err(_) = std::fs::File::open(&path) {
                            let idx = shared
                                .recent_file_paths
                                .iter()
                                .position(|r_path| *r_path == path)
                                .unwrap();
                            shared.recent_file_paths.remove(idx);
                            continue;
                        }

                        skf_file_button(path, shared, ui, ctx, available_width);
                        ui.add_space(5.);
                    }
                });
            },
        );
    });

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(45.);
        let width = 200.;
        ui.set_width(width);
        ui.vertical(|ui| {
            ui.set_width(width);
            let available_size = ui.available_size();
            ui.add_space(10.);
            egui::Frame::new()
                .fill(shared.config.colors.dark_accent.into())
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(available_size.x);
                    ui.set_height(available_size.y - 55.);

                    let header_size = 15.;
                    let sub_size = 13.;
                    let sub_padding = 20.;
                    let separator = 15.;

                    for item in &shared.startup.resources {
                        let heading_str =
                            shared.loc(&("startup.resources.".to_owned() + &item.code));
                        let heading = ui.clickable_label(
                            egui::RichText::new(heading_str)
                                .color(egui::Color32::from_hex("#659adf").unwrap())
                                .size(header_size),
                        );
                        if heading.clicked() {
                            open_link(&item, &item.url_type);
                        }
                        ui.add_space(5.);
                        for sub in &item.items {
                            ui.horizontal(|ui| {
                                let left_top = egui::Pos2::new(
                                    ui.min_rect().left_top().x + 5.,
                                    ui.min_rect().left_top().y - 10.,
                                );
                                ui.painter().rect_filled(
                                    egui::Rect::from_min_size(
                                        left_top,
                                        egui::Vec2::new(2., sub_size + 8.),
                                    ),
                                    egui::CornerRadius::ZERO,
                                    egui::Color32::from_hex("#223752").unwrap(),
                                );
                                ui.add_space(sub_padding);
                                let sub_str =
                                    shared.loc(&("startup.resources.".to_owned() + &sub.code));
                                let sub_text = ui.clickable_label(
                                    egui::RichText::new(sub_str)
                                        .color(egui::Color32::from_hex("#659adf").unwrap())
                                        .size(sub_size),
                                );
                                if sub_text.clicked() {
                                    open_link(&sub, &item.url_type);
                                }
                            });
                        }
                        ui.add_space(separator);
                    }
                })
        });
    });
}

pub fn startup_leftside_button(
    icon: &str,
    label: &str,
    ui: &mut egui::Ui,
    shared: &Shared,
    mut icon_offset: Option<egui::Vec2>,
    img: Option<&egui::TextureHandle>,
) -> egui::Response {
    if icon_offset == None {
        icon_offset = Some(egui::Vec2::new(0., 0.));
    }

    let gradient_rect =
        egui::Rect::from_min_size(ui.cursor().left_top(), egui::Vec2::new(133., 48.));

    let button = ui
        .interact(
            gradient_rect,
            egui::Id::new("leftside".to_owned() + &label),
            egui::Sense::click(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    if button.contains_pointer() {
        ui.gradient(
            gradient_rect,
            egui::Color32::TRANSPARENT,
            shared.config.colors.dark_accent.into(),
        );
    }

    egui::Frame::new().show(ui, |ui| {
        ui.set_width(128.);
        ui.set_height(48.);
        let icon_pos = egui::Pos2::new(
            ui.min_rect().left_center().x + 20.,
            ui.min_rect().left_center().y - 2.5,
        ) + icon_offset.unwrap();
        if img != None {
            let size = egui::Vec2::new(20., 24.);
            let rect = egui::Rect::from_min_size(icon_pos, size.into());
            egui::Image::new(img.unwrap())
                .fit_to_exact_size(size)
                .paint_at(ui, rect);
        }
        ui.painter().text(
            icon_pos,
            egui::Align2::LEFT_CENTER,
            icon.to_string(),
            egui::FontId::new(25., egui::FontFamily::default()),
            egui::Color32::WHITE,
        );
        let label_pos = egui::Pos2::new(
            ui.min_rect().left_center().x + 50.,
            ui.min_rect().left_center().y,
        );
        ui.painter().text(
            label_pos,
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::new(17., egui::FontFamily::default()),
            shared.config.colors.text.into(),
        );
    });

    let bottom = egui::Rect::from_min_size(
        ui.min_rect().left_bottom(),
        egui::Vec2::new(ui.min_rect().right() - ui.min_rect().left(), 1.),
    );
    ui.painter().rect_filled(
        bottom,
        egui::CornerRadius::ZERO,
        shared.config.colors.dark_accent,
    );

    button
}

#[cfg(not(target_arch = "wasm32"))]
pub fn skf_file_button(
    path: String,
    shared: &mut Shared,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    width: f32,
) {
    let filename = path.split('/').last().unwrap().to_string();
    let file = std::fs::File::open(path.clone()).unwrap();
    let zip = zip::ZipArchive::new(file);
    if let Err(_) = zip {
        return;
    }

    // generate thumbnail UI texture
    if !shared.thumb_ui_tex.contains_key(&filename) {
        let mut thumb_bytes = vec![];
        for byte in zip.unwrap().by_name("thumbnail.png").unwrap().bytes() {
            thumb_bytes.push(byte.unwrap());
        }
        let ui_tex = ui::create_ui_texture(thumb_bytes, false, ctx).unwrap();
        shared.thumb_ui_tex.insert(filename.clone(), ui_tex.clone());
    }

    let thumb_size = Vec2::new(64., 64.);

    ui.horizontal(|ui| {
        ui.set_width(width);
        ui.set_height(85.);

        let gradient_rect = egui::Rect::from_min_max(
            egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() - 5.),
            egui::Pos2::new(ui.min_rect().right() + 25., ui.min_rect().bottom()),
        );

        let button = ui
            .interact(
                gradient_rect,
                egui::Id::new("frame rect".to_owned() + &path),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if button.contains_pointer() {
            ui.gradient(
                gradient_rect,
                egui::Color32::TRANSPARENT,
                shared.config.colors.dark_accent.into(),
            );
        }

        egui::Frame::new()
            .inner_margin(egui::Margin::same(10))
            .fill(egui::Color32::TRANSPARENT)
            .show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(65.);

                let rect = egui::Rect::from_min_size(
                    egui::Pos2::new(ui.cursor().min.x, ui.cursor().min.y),
                    thumb_size.into(),
                );
                egui::Image::new(shared.thumb_ui_tex.get(&filename).unwrap()).paint_at(ui, rect);
                let heading_pos = egui::Pos2::new(
                    ui.min_rect().left_top().x + 72.,
                    ui.min_rect().left_top().y + 18.,
                );
                ui.painter().text(
                    heading_pos,
                    egui::Align2::LEFT_BOTTOM,
                    filename.clone(),
                    egui::FontId::new(16., egui::FontFamily::Proportional),
                    shared.config.colors.text.into(),
                );
                if filename == "autosave.skf" {
                    let heading_pos = egui::Pos2::new(
                        ui.min_rect().left_bottom().x + 72.,
                        ui.min_rect().left_bottom().y,
                    );
                    let mut col = shared.config.colors.text;
                    col -= Color::new(40, 40, 40, 0);
                    ui.painter().text(
                        heading_pos,
                        egui::Align2::LEFT_BOTTOM,
                        "Auto-generated backup file",
                        egui::FontId::new(11., egui::FontFamily::Proportional),
                        col.into(),
                    );
                }
            });

        if button.clicked() {
            file_reader::create_temp_file(&shared.temp_path.import, &path);
            shared.ui.set_state(UiState::StartupWindow, false);
        }

        let bottom = egui::Rect::from_min_size(
            ui.min_rect().left_bottom(),
            egui::Vec2::new(ui.min_rect().right() - ui.min_rect().left(), 1.),
        );
        ui.painter().rect_filled(
            bottom,
            egui::CornerRadius::ZERO,
            shared.config.colors.dark_accent,
        );

        if !button.contains_pointer() {
            return;
        }

        let mut pos = egui::Vec2::new(0., 0.);
        if file_button_icon("X", "Remove from list", egui::Vec2::new(-20., 8.), pos, ui).clicked() {
            let idx = shared
                .recent_file_paths
                .iter()
                .position(|rfp| *rfp == path)
                .unwrap();
            shared.recent_file_paths.remove(idx);
            utils::save_to_recent_files(&shared.recent_file_paths);
        }
        pos += egui::Vec2::new(-21., 0.);

        if file_button_icon("🗑", "Delete file", egui::Vec2::new(-19., 8.), pos, ui).clicked() {
            shared.ui.selected_path = path.clone();
            shared.ui.open_polar_modal(
                PolarId::DeleteFile,
                &("Are you sure to delete ".to_owned() + &filename + "?"),
            );
        }
        pos += egui::Vec2::new(-21., 0.);

        if file_button_icon("🗁", "Open folder", egui::Vec2::new(-19., 8.), pos, ui).clicked() {
            match open::that(std::path::Path::new(&path).parent().unwrap()) {
                Ok(file) => file,
                _ => {}
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn file_button_icon(
    icon: &str,
    name: &str,
    offset: egui::Vec2,
    pos: egui::Vec2,
    ui: &mut egui::Ui,
) -> egui::Response {
    let rect = egui::Rect::from_min_size(
        ui.min_rect().right_top() + pos + offset,
        egui::Vec2::splat(20.),
    );

    let hovered = ui
        .interact(
            rect,
            egui::Id::new("filebutton".to_owned() + icon),
            egui::Sense::hover(),
        )
        .contains_pointer();

    let col = if hovered {
        egui::Color32::WHITE
    } else {
        egui::Color32::PLACEHOLDER
    };

    let label = egui::Label::new(egui::RichText::new(icon).size(18.).color(col));
    ui.put(rect, label)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(name)
}

#[cfg(not(target_arch = "wasm32"))]
fn open_link(item: &StartupResourceItem, url_type: &StartupItemType) {
    if *url_type == StartupItemType::Custom {
        let _ = open::that(item.url.clone());
    } else {
        utils::open_docs(*url_type == StartupItemType::DevDocs, &item.url);
    }
}
