use crate::ui::EguiUi;
use crate::*;

const SKEL_SKF: &str = "_skellington.skf";
const SKELA_SKF: &str = "_skellina.skf";

pub fn startup_modal(shared: &mut Shared, ctx: &egui::Context) {
    let window = egui::Window::new("startup")
        .title_bar(false)
        .resizable(false)
        .movable(false);
    window.show(ctx, |ui| {
        let gradient = shared.config.colors.gradient.into();
        ui.gradient(
            ui.ctx().content_rect(),
            egui::Color32::TRANSPARENT,
            gradient,
        );
        let width = ui.ctx().content_rect().width();
        let height = ui.ctx().content_rect().height();
        ui.set_width(width.max(0.));
        ui.set_height(height.max(0.));

        let available_size = ui.available_size();

        let layout = egui::Layout::centered_and_justified(egui::Direction::LeftToRight);
        ui.with_layout(layout, |ui| {
            let padding = 600.;
            ui.set_width((available_size.x - padding).max(800.));
            let size = ui.available_size();
            ui.horizontal(|ui| {
                ui.set_width(size.x);
                ui.set_height(size.y);
                startup_content(&ctx, ui, shared, size);
            });
        })
    });
}

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
        let empty = "".to_string();
        if leftside_button("+", &shared.ui.loc("new"), ui, shared, None, None, empty).clicked() {
            shared.ui.unselect_everything();
            shared.ui.anim.open = false;
            shared.camera.pos = Vec2::new(0., 0.);
            shared.camera.zoom = 2000.;
            shared.armature = Armature::default();
            shared.ui.startup_window = false;
        }
        ui.add_space(padding);
        let import_pos = Some(egui::Vec2::new(-5., 2.5));
        let str_import = &shared.ui.loc("startup.import");
        let empty = "".to_string();
        if leftside_button("🗋", str_import, ui, shared, import_pos, None, empty).clicked() {
            #[cfg(target_arch = "wasm32")]
            crate::clickFileInput(false);
            #[cfg(not(target_arch = "wasm32"))]
            utils::open_import_dialog(&shared.file_name, &shared.import_contents);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            ui.add_space(padding);
            let samples_pos = Some(egui::Vec2::new(-5., 2.5));
            let str_samples = &shared.ui.loc("startup.samples");
            let empty = "".to_string();
            if leftside_button("🗊", str_samples, ui, shared, samples_pos, None, empty).clicked()
            {
                shared.ui.showing_samples = !shared.ui.showing_samples;
            }
            ui.add_space(padding);
            if shared.ui.showing_samples {
                let skel_pos = Some(egui::Vec2::new(-5., -10.));
                macro_rules! add_thumb_tex {
                    ($key:expr, $filename:expr) => {
                        if !shared.ui.thumb_ui_tex.contains_key($key) {
                            let skel_file = include_bytes!($filename).to_vec();
                            shared.ui.thumb_ui_tex.insert(
                                $key.to_string(),
                                ui::create_ui_texture(skel_file, true, ctx).unwrap(),
                            );
                        }
                    };
                }

                macro_rules! sample_button {
                    ($key:expr, $name:expr, $path:expr, $desc:expr) => {
                        let thumb_tex = shared.ui.thumb_ui_tex.get($key);
                        if leftside_button("", $name, ui, shared, skel_pos, thumb_tex, $desc)
                            .clicked()
                        {
                            *shared.file_name.lock().unwrap() = $path.to_string();
                            *shared.import_contents.lock().unwrap() = vec![0];
                            shared.ui.startup_window = false;
                        }
                    };
                }

                add_thumb_tex!("skellington_icon.png", "../assets/skellington_icon.png");
                add_thumb_tex!("skellina_icon.png", "../assets/skellina_icon.png");

                let key = "skellington_icon.png";
                let sample = utils::bin_path() + "samples/" + SKEL_SKF;
                let desc = shared.ui.loc("startup.skellington_sample_desc");
                let name = "Skellington";
                sample_button!(key, name, sample, desc);

                let key = "skellina_icon.png";
                let sample = utils::bin_path() + "samples/" + SKELA_SKF;
                let desc = shared.ui.loc("startup.skellina_sample_desc");
                sample_button!(key, "Skellina", sample, desc);
            }
        }
    });

    ui.add_space(10.);
    ui.separator();

    ui.vertical(|ui| {
        ui.add_space(11.);
        let reserved_for_resources = 420.;
        ui.set_width((available_size.x - reserved_for_resources).max(1.));
        let width = ui.available_width();
        let layout = egui::Layout::centered_and_justified(egui::Direction::LeftToRight);
        ui.with_layout(layout, |ui| {
            let max_width = 600.;
            let right_margin = 50.;
            ui.set_max_width((width.min(max_width) - right_margin).max(1.));
            ui.set_min_width(0.);

            #[cfg(not(target_arch = "wasm32"))]
            ui.vertical(|ui| {
                let available_width = ui.available_width();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.set_width(available_width);

                    let mut has_files = false;

                    for p in 0..shared.recent_file_paths.len() {
                        // safeguard for deleting a path during iteration
                        if p > shared.recent_file_paths.len() - 1 {
                            break;
                        }

                        let path = shared.recent_file_paths[p].to_string();

                        // ignore filenames starting with _
                        let filename = path.split('/').collect::<Vec<_>>();
                        if filename.last().unwrap().chars().nth(0).unwrap() == '_' {
                            continue;
                        }

                        has_files = true;

                        if let Err(_) = std::fs::File::open(&path) {
                            let recent = &shared.recent_file_paths;
                            let idx = recent.iter().position(|r_path| *r_path == path).unwrap();
                            shared.recent_file_paths.remove(idx);
                            continue;
                        }

                        skf_file_button(path, shared, ui, ctx, available_width);
                        ui.add_space(5.);
                    }

                    if !has_files {
                        ui.add_space(10.);

                        let msg = &shared.ui.loc("startup.empty_recent_files");
                        let text = egui::RichText::new(msg).size(14.);
                        ui.label(text);

                        let msg = &shared.ui.loc("startup.early_access_warning");
                        let mut orange: crate::Color = egui::Color32::ORANGE.into();
                        orange -= crate::Color::new(30, 30, 30, 0);
                        let text = egui::RichText::new(msg).size(14.).color(orange);
                        ui.label(text);

                        return;
                    }
                });
            });

            #[cfg(target_arch = "wasm32")]
            ui.vertical(|ui| {
                let width = ui.available_width();
                let msg = &shared.ui.loc("startup.web_note");
                let text = egui::RichText::new(msg).size(14.);
                ui.label(text);
                ui.add_space(20.);

                let name = "Skellington Sample".to_owned();
                let skf_name = SKEL_SKF.to_string();
                let skel_file = include_bytes!(".././assets/skellington_icon.png").to_vec();
                let desc = shared.ui.loc("startup.skellington_sample_desc");
                web_sample_button(name, skf_name, skel_file, shared, ui, ctx, width, desc);

                let name = "Skellina Sample".to_owned();
                let skf_name = SKELA_SKF.to_string();
                let skel_file = include_bytes!(".././assets/skellina_icon.png").to_vec();
                let desc = shared.ui.loc("startup.skellina_sample_desc");
                web_sample_button(name, skf_name, skel_file, shared, ui, ctx, width, desc);
            })
        });
    });

    ui.separator();

    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.add_space(15.);
        let width = 200.;
        ui.set_width(width);
        ui.vertical(|ui| {
            ui.set_width(width);
            let available_size = ui.available_size();
            ui.add_space(10.);
            let frame = egui::Frame::new().inner_margin(egui::Margin::same(5));
            frame.show(ui, |ui| {
                ui.set_width(available_size.x);
                ui.set_height(available_size.y - 55.);

                let header_size = 15.;
                let sub_size = 13.;
                let sub_padding = 20.;
                let sub_line_height = 2.;
                let separator = 15.;

                let link_color = shared.config.colors.link;

                let web;
                #[cfg(target_arch = "wasm32")]
                {
                    web = true;
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    web = false;
                }

                for item in &shared.ui.startup.resources {
                    if item.update_checker && web {
                        continue;
                    }
                    let str = &shared.ui.loc(&("startup.resources.".to_owned() + &item.code));
                    let text = egui::RichText::new(str).color(link_color).size(header_size);
                    let heading = ui.clickable_label(text);
                    if heading.clicked() {
                        if !item.update_checker {
                            open_link(&item, &item.url_type);
                        } else {
                            shared.ui.checking_update = true;
                        }
                    }
                    ui.add_space(5.);

                    for sub in &item.items {
                        ui.horizontal(|ui| {
                            let left_top = egui::Pos2::new(
                                ui.min_rect().left_top().x + 5.,
                                ui.min_rect().left_top().y - 10.,
                            );
                            let mut line_color = link_color;
                            let darker = 105;
                            line_color -= Color::new(darker, darker, darker, 0);
                            let right_bot = egui::Vec2::new(2., sub_size + 8. + sub_line_height);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(left_top, right_bot),
                                egui::CornerRadius::ZERO,
                                line_color,
                            );
                            ui.add_space(sub_padding);
                            let sub_str =
                                &shared.ui.loc(&("startup.resources.".to_owned() + &sub.code));

                            let text = egui::RichText::new(sub_str)
                                .color(link_color)
                                .size(sub_size);
                            let sub_text = ui.clickable_label(text);
                            if sub_text.clicked() {
                                open_link(&sub, &item.url_type);
                            }
                        });
                        ui.add_space(sub_line_height);
                    }
                    ui.add_space(separator);
                }
            })
        });
    });
}

pub fn leftside_button(
    icon: &str,
    label: &str,
    ui: &mut egui::Ui,
    shared: &Shared,
    mut icon_offset: Option<egui::Vec2>,
    img: Option<&egui::TextureHandle>,
    tooltip: String,
) -> egui::Response {
    if icon_offset == None {
        icon_offset = Some(egui::Vec2::new(0., 0.));
    }

    let gradient = egui::Rect::from_min_size(ui.cursor().left_top(), egui::Vec2::new(133., 48.));

    let button: egui::Response;
    let id = egui::Id::new("leftside".to_owned() + &label);

    if tooltip != "" {
        button = ui
            .interact(gradient, id, egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tooltip);
    } else {
        button = ui
            .interact(gradient, id, egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
    }

    if button.contains_pointer() {
        let dark_accent = shared.config.colors.dark_accent.into();
        ui.gradient(gradient, egui::Color32::TRANSPARENT, dark_accent);
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
        let align = egui::Align2::LEFT_CENTER;
        let white = egui::Color32::WHITE;
        let font25 = egui::FontId::new(25., egui::FontFamily::default());
        ui.painter()
            .text(icon_pos, align, icon.to_string(), font25.clone(), white);

        let label_pos = egui::Pos2::new(
            ui.min_rect().left_center().x + 50.,
            ui.min_rect().left_center().y,
        );

        let text_col = shared.config.colors.text.into();
        let font17 = egui::FontId::new(17., egui::FontFamily::default());
        ui.painter().text(label_pos, align, label, font17, text_col);
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
    let mut zip = zip::ZipArchive::new(file);
    if let Err(_) = zip {
        return;
    }

    // generate thumbnail UI texture
    if !shared.ui.thumb_ui_tex.contains_key(&filename) {
        let mut thumb_bytes = vec![];
        let file = zip.as_mut().unwrap().by_name("thumbnail.png");
        if let Ok(_) = file {
            for byte in file.unwrap().bytes() {
                thumb_bytes.push(byte.unwrap());
            }
            let ui_tex = ui::create_ui_texture(thumb_bytes, false, ctx).unwrap();
            shared.ui.thumb_ui_tex.insert(filename.clone(), ui_tex.clone());
        }
    }

    let thumb_size = Vec2::new(64., 64.);

    ui.horizontal(|ui| {
        ui.set_width(width);
        ui.set_height(85.);

        let gradient_rect = egui::Rect::from_min_max(
            egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() - 5.),
            egui::Pos2::new(ui.min_rect().right() + 25., ui.min_rect().bottom()),
        );

        let id = egui::Id::new("frame rect".to_owned() + &path);
        let button = ui
            .interact(gradient_rect, id, egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if button.hovered() {
            let dark_accent = shared.config.colors.dark_accent.into();
            ui.gradient(gradient_rect, egui::Color32::TRANSPARENT, dark_accent);
        }

        let frame = egui::Frame::new()
            .inner_margin(egui::Margin::same(10))
            .fill(egui::Color32::TRANSPARENT);
        frame.show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(65.);

            let rect = egui::Rect::from_min_size(
                egui::Pos2::new(ui.cursor().min.x, ui.cursor().min.y),
                thumb_size.into(),
            );
            if let Some(thumb_tex) = shared.ui.thumb_ui_tex.get(&filename) {
                egui::Image::new(thumb_tex).paint_at(ui, rect);
            }
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
                    &shared.ui.loc("startup.autosave_note"),
                    egui::FontId::new(11., egui::FontFamily::Proportional),
                    col.into(),
                );
            }
        });

        if button.clicked() {
            *shared.file_name.lock().unwrap() = path.clone();
            *shared.import_contents.lock().unwrap() = vec![0];
            shared.ui.startup_window = false;
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

        let mut pos = egui::Vec2::new(-21., 0.);
        if file_button_icon("X", "Remove from list", egui::Vec2::new(-20., 8.), pos, ui).clicked() {
            let recent = &shared.recent_file_paths;
            let idx = recent.iter().position(|rfp| *rfp == path).unwrap();
            shared.recent_file_paths.remove(idx);
            utils::save_to_recent_files(&shared.recent_file_paths);
        }
        pos += egui::Vec2::new(-21., 0.);

        if file_button_icon("🗑", "Delete file", egui::Vec2::new(-19., 8.), pos, ui).clicked() {
            shared.ui.selected_path = path.clone();
            let str_del = &shared.ui.loc("polar.delete_file").replace("$", &filename);
            shared
                .ui
                .open_polar_modal(PolarId::DeleteFile, str_del.to_string());
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

#[cfg(target_arch = "wasm32")]
pub fn web_sample_button(
    name: String,
    filename: String,
    thumb_bytes: Vec<u8>,
    shared: &mut Shared,
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    width: f32,
    tooltip: String,
) {
    let thumb_size = Vec2::new(64., 64.);

    if !shared.ui.thumb_ui_tex.contains_key(&name) {
        let ui_tex = ui::create_ui_texture(thumb_bytes, true, ctx).unwrap();
        shared.ui.thumb_ui_tex.insert(name.clone(), ui_tex.clone());
    }

    ui.horizontal(|ui| {
        ui.set_width(width);
        ui.set_height(85.);

        let gradient_rect = egui::Rect::from_min_max(
            egui::Pos2::new(ui.min_rect().left(), ui.min_rect().top() - 5.),
            egui::Pos2::new(ui.min_rect().right() + 25., ui.min_rect().bottom()),
        );

        let id = egui::Id::new("frame rect".to_owned() + &name);
        let button = ui
            .interact(gradient_rect, id, egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);

        if button.contains_pointer() {
            let dark_accent = shared.config.colors.dark_accent.into();
            ui.gradient(gradient_rect, egui::Color32::TRANSPARENT, dark_accent);
        }

        let frame = egui::Frame::new()
            .inner_margin(egui::Margin::same(10))
            .fill(egui::Color32::TRANSPARENT);
        frame
            .show(ui, |ui| {
                ui.set_width(width);
                ui.set_height(65.);

                let rect = egui::Rect::from_min_size(
                    egui::Pos2::new(ui.cursor().min.x, ui.cursor().min.y),
                    thumb_size.into(),
                );
                if let Some(thumb_tex) = shared.ui.thumb_ui_tex.get(&name) {
                    egui::Image::new(thumb_tex).paint_at(ui, rect);
                }
                let mut pos = egui::Pos2::new(
                    ui.min_rect().left_top().x + 72.,
                    ui.min_rect().left_top().y + 18.,
                );

                let align = egui::Align2::LEFT_BOTTOM;
                let font = egui::FontId::new(16., egui::FontFamily::Proportional);
                let mut col = shared.config.colors.text;
                ui.painter().text(pos, align, name, font, col.into());

                pos.y += 18.;
                let font = egui::FontId::new(11., egui::FontFamily::Proportional);
                col -= Color::new(40, 40, 40, 0);
                let trunc = utils::trunc_str(ui, &tooltip, ui.min_rect().width());
                ui.painter().text(pos, align, trunc, font, col.into());
            })
            .response
            .on_hover_text(tooltip);

        if button.clicked() {
            crate::downloadSample(filename.to_string());
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

    let id = egui::Id::new("filebutton".to_owned() + icon);
    let hovered = ui
        .interact(rect, id, egui::Sense::hover())
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

fn open_link(item: &StartupResourceItem, url_type: &StartupItemType) {
    if *url_type == StartupItemType::Custom {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = open::that(item.url.clone());
        #[cfg(target_arch = "wasm32")]
        crate::openLink(item.url.clone());
    } else {
        utils::open_docs(*url_type == StartupItemType::DevDocs, &item.url);
    }
}
