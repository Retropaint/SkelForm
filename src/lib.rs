//! Notable comments:
//!
//! `disabled:` - features that have been implemented at some point, but are dormant for a later version. Usually for complex or optional features.
//!
//! `runtime:` - implementation relevant to runtimes (eg. animation logic, forward/inverse kinematics, etc).
//!
//! `iterable` - snippets that aren't automated/iterated, but probably should be.
//!
//! `todo:` - not important as of being written, but good to keep in mind.

use std::{
    io::{Seek, Write},
    sync::Mutex,
};

use egui_wgpu::wgpu::ExperimentalFeatures;
use shared::*;
use wgpu::{util::DeviceExt, BindGroupLayout, Buffer, InstanceDescriptor};

// native-only imports
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use image::*;
    pub use std::fs;
    pub use std::io::Read;
    pub use std::process::{Command, Stdio};
    pub use std::time::Instant;
}
#[cfg(not(target_arch = "wasm32"))]
use native::*;
use zip::write::FullFileOptions;

#[cfg(target_arch = "wasm32")]
mod web {
    pub use wasm_bindgen::prelude::*;
    pub use web_sys::*;
    pub use web_time::Instant;
}
use std::sync::Arc;
#[cfg(target_arch = "wasm32")]
use web::*;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    window::{Theme, Window},
};

pub mod armature_window;
pub mod atlas_modal;
pub mod backwards_compat;
pub mod bone_panel;
pub mod editor;
pub mod export_modal;
pub mod file_reader;
pub mod keyframe_editor;
pub mod keyframe_panel;
pub mod modal;
pub mod renderer;
pub mod settings_modal;
pub mod shared;
pub mod startup_window;
pub mod styles_modal;
pub mod ui;
pub mod utils;
pub mod warnings;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    pub fn getCanvasWidth() -> u32;
    pub fn getCanvasHeight() -> u32;
    pub fn getConfig() -> String;
    pub fn saveConfig(data_str: String);
    pub fn getUiSliderValue() -> f32;
    pub fn toggleElement(open: bool, id: String);
    pub fn isModalActive(id: String) -> bool;
    pub fn getEditInput() -> String;
    pub fn setEditInput(value: String);
    pub fn removeImage();
    pub fn getFile() -> Vec<u8>;
    pub fn getFileName() -> String;
    pub fn removeFile();
    pub fn loaded();
    pub fn focusEditInput();
    pub fn openDocumentation(docs_name: String, path: String);
    pub fn updateUiSlider();
    pub fn downloadSample(filename: String);
    pub fn openLink(url: String);
    pub fn isMobile() -> bool;
    pub fn clickFileInput(isImage: bool);
    pub fn hasElement(id: &str) -> bool;
    pub fn getImgName(idx: usize) -> String;
    pub fn hasLoadedAllImages() -> bool;
    pub fn downloadZip(zip: Vec<u8>, saving: String);
    pub fn downloadMp4(data: Vec<u8>, resX: f32, resY: f32, name: &str, fps: i32);
    pub fn downloadGif(resX: f32, resY: f32, name: &str, fps: i32);
    pub fn addGifFrame(frame: Vec<u8>);
}

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<BackendRenderer>,
    gui_state: Option<egui_winit::State>,
    last_render_time: Option<Instant>,
    #[cfg(target_arch = "wasm32")]
    pub renderer_receiver: Option<futures::channel::oneshot::Receiver<BackendRenderer>>,
    pub shared: shared::Shared,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut attributes = Window::default_attributes();

        #[cfg(target_os = "windows")]
        {
            let file_bytes = include_bytes!("../assets/skf_icon.png");
            let diffuse_image = image::load_from_memory(file_bytes).unwrap();
            let rgba = diffuse_image.to_rgba8();
            let pixels = rgba.as_bytes().to_vec();

            let icon = winit::window::Icon::from_rgba(pixels, rgba.width(), rgba.height()).unwrap();
            attributes.window_icon = Some(icon);
        }

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = wgpu::web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .get_element_by_id("canvas")
                .unwrap()
                .dyn_into::<wgpu::web_sys::HtmlCanvasElement>()
                .unwrap();
            self.shared.camera.window = Vec2::new(canvas.width() as f32, canvas.height() as f32);
            attributes = attributes.with_canvas(Some(canvas));
        }

        if let Ok(window) = event_loop.create_window(attributes) {
            let first_window_handle = self.window.is_none();
            let window_handle = Arc::new(window);
            self.window = Some(window_handle.clone());

            if !first_window_handle {
                return;
            }

            let gui_context = egui::Context::default();

            // turn off egui kb zoom
            gui_context.options_mut(|op| {
                op.zoom_with_keyboard = false;
            });

            #[cfg(target_arch = "wasm32")]
            {
                gui_context.set_pixels_per_point(window_handle.scale_factor() as f32);
            }

            let viewport_id = gui_context.viewport_id();
            let gui_state = egui_winit::State::new(
                gui_context,
                viewport_id,
                &window_handle,
                Some(window_handle.scale_factor() as f32),
                Some(Theme::Dark),
                None,
            );

            #[cfg(not(target_arch = "wasm32"))]
            let (width, height) = (
                window_handle.inner_size().width,
                window_handle.inner_size().height,
            );

            #[cfg(not(target_arch = "wasm32"))]
            {
                let renderer = pollster::block_on(async move {
                    BackendRenderer::new(window_handle.clone(), width, height).await
                });
                self.renderer = Some(renderer);
            }

            #[cfg(target_arch = "wasm32")]
            {
                let (sender, receiver) = futures::channel::oneshot::channel();
                self.renderer_receiver = Some(receiver);
                let size = self.shared.camera.window.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let renderer =
                        BackendRenderer::new(window_handle.clone(), size.x as u32, size.y as u32)
                            .await;
                    if sender.send(renderer).is_err() {
                        log::error!("Failed to create and send renderer!");
                    }
                });
            }

            self.gui_state = Some(gui_state);
            self.last_render_time = Some(Instant::now());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        #[cfg(target_arch = "wasm32")]
        {
            let mut renderer_received = false;
            if let Some(receiver) = self.renderer_receiver.as_mut() {
                if let Ok(Some(renderer)) = receiver.try_recv() {
                    self.renderer = Some(renderer);
                    renderer_received = true;
                }
            }
            if renderer_received {
                self.renderer_receiver = None;
            }
        }

        file_reader::read(
            &mut self.shared,
            &self.renderer,
            self.gui_state.as_ref().unwrap().egui_ctx(),
        );

        if self.shared.ui.scale <= 0. {
            self.shared.ui.scale = 1.;
        }

        let (Some(gui_state), Some(renderer), Some(window), Some(last_render_time)) = (
            self.gui_state.as_mut(),
            self.renderer.as_mut(),
            self.window.as_ref(),
            self.last_render_time.as_mut(),
        ) else {
            return;
        };

        // Receive gui window event
        if gui_state.on_window_event(window, &event).consumed {
            //return;
        }

        // If the gui didn't consume the event, handle it
        match event {
            WindowEvent::HoveredFile(_) => {
                self.shared.events.open_modal("drop_file", true);
            }
            WindowEvent::HoveredFileCancelled => {
                self.shared.ui.modal = false;
            }
            WindowEvent::DroppedFile(_path_buf) => {
                self.shared.ui.modal = false;
                #[cfg(not(target_arch = "wasm32"))]
                {
                    *self.shared.ui.file_path.lock().unwrap() = vec![_path_buf];
                }
            }
            WindowEvent::CloseRequested => {
                utils::exit(
                    &mut self.shared.undo_states,
                    &self.shared.config,
                    &mut self.shared.ui,
                );
            }
            WindowEvent::Focused(is_focused) => {
                let manager = &mut self.shared.input.hotkey_manager;
                let mod_q = &mut self.shared.input.mod_q;
                let mod_w = &mut self.shared.input.mod_w;
                if is_focused {
                    _ = manager.as_mut().unwrap().register(mod_q.unwrap());
                    _ = manager.as_mut().unwrap().register(mod_w.unwrap());
                } else {
                    let manager = &mut self.shared.input.hotkey_manager;
                    _ = manager.as_mut().unwrap().unregister(mod_q.unwrap());
                    _ = manager.as_mut().unwrap().unregister(mod_w.unwrap());
                }
            }
            #[allow(unused_mut)]
            WindowEvent::Resized(PhysicalSize {
                mut width,
                mut height,
            }) => {
                if width == 0 || height == 0 {
                    return;
                }
                self.shared.camera.window = Vec2::new(width as f32, height as f32);
                renderer.resize(width as u32, height as u32);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                *last_render_time = now;

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let undo = &mut self.shared.undo_states;
                    if undo.prev_undo_actions != undo.undo_actions.len() {
                        self.shared.ui.changed_window_name = false;
                        undo.prev_undo_actions = undo.undo_actions.len();
                    }
                    if !self.shared.ui.changed_window_name {
                        let file = if self.shared.ui.save_path == None {
                            "SkelForm".to_string()
                        } else {
                            let path = self.shared.ui.save_path.clone().unwrap();
                            let filename = path.as_path().file_name().unwrap();
                            filename.to_str().unwrap().to_string()
                        };
                        let undo = &self.shared.undo_states;
                        let unsaved = if undo.unsaved_undo_actions != undo.undo_actions.len() {
                            " *"
                        } else {
                            ""
                        };
                        let title =
                            file + " - v" + &env!("CARGO_PKG_VERSION").to_string() + unsaved;
                        window.set_title(&title);
                        self.shared.ui.changed_window_name = true;
                    }
                }

                let input = gui_state.take_egui_input(&window);
                gui_state.egui_ctx().begin_pass(input);

                utils::animate_bones(
                    &mut self.shared.armature,
                    &self.shared.selections,
                    &self.shared.edit_mode,
                );

                let s = &mut self.shared;
                #[rustfmt::skip]
                ui::process_inputs(
                    gui_state.egui_ctx(), &mut s.input, &mut s.ui, &s.config,
                    &mut s.edit_mode, &mut s.events, &s.camera, &s.selections, &mut s.armature
                );

                // ui logic handled in ui.rs
                #[rustfmt::skip]
                ui::draw(
                    gui_state.egui_ctx(), &mut s.ui, &mut s.input, &mut s.selections, 
                    &mut s.config, &mut s.events, &mut s.edit_mode, &s.camera, &mut s.armature
                );

                let egui_winit::egui::FullOutput {
                    textures_delta,
                    shapes,
                    pixels_per_point,
                    platform_output,
                    ..
                } = gui_state.egui_ctx().end_pass();

                gui_state.handle_platform_output(window, platform_output);
                let paint_jobs = gui_state.egui_ctx().tessellate(shapes, pixels_per_point);

                let size = window.inner_size();
                let screen_descriptor = {
                    egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [size.width, size.height],
                        pixels_per_point,
                    }
                };

                if !self.shared.renderer.initialized_window && size.width != 0 && size.height != 0 {
                    renderer.resize(size.width as u32, size.height as u32);
                    self.shared.renderer.initialized_window = true;
                }

                self.shared.camera.window = Vec2::new(size.width as f32, size.height as f32);
                renderer.render_frame(
                    screen_descriptor,
                    paint_jobs,
                    textures_delta,
                    &mut self.shared,
                );

                #[cfg(target_arch = "wasm32")]
                {
                    self.shared.ui.scale = getUiSliderValue() * window.scale_factor() as f32;
                    self.shared.ui.mobile = isMobile();
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.shared.ui.scale =
                        self.shared.config.ui_scale * window.scale_factor() as f32;
                }
                gui_state
                    .egui_ctx()
                    .set_pixels_per_point(self.shared.ui.scale);

                while self.shared.events.events.len() > 0 {
                    let s = &mut self.shared;
                    #[rustfmt::skip]
                    editor::iterate_events(
                        &s.input, &mut s.config, &mut s.events, &mut s.camera, &mut s.edit_mode, &mut s.selections, 
                        &mut s.undo_states, &mut s.armature, &mut s.copy_buffer, &mut s.ui, &mut s.renderer
                    );
                }
            }
            _ => (),
        }

        // read system shortcuts (defined in main.rs with global_hotkey)
        if let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            let pressing_w = event.id() == self.shared.input.mod_w.unwrap().id();
            let pressing_q = event.id() == self.shared.input.mod_q.unwrap().id();
            if (pressing_w || pressing_q) && self.shared.ui.can_quit {
                utils::exit(
                    &mut self.shared.undo_states,
                    &self.shared.config,
                    &mut self.shared.ui,
                );
            }
        }

        if self.shared.ui.exiting {
            let undo = &self.shared.undo_states;
            if undo.unsaved_undo_actions != undo.undo_actions.len() {
                let str_del = self.shared.ui.loc("polar.unsaved").clone().to_string();
                let exiting = PolarId::Exiting;
                self.shared.events.open_polar_modal(exiting, str_del);
            } else {
                event_loop.exit();
            }
            self.shared.ui.exiting = false;
        }

        if self.shared.ui.confirmed_exit {
            if self.shared.ui.never_donate {
                self.shared.config.ignore_donate = true;
                crate::utils::save_config(&self.shared.config);
            }
            event_loop.exit();
        }

        window.request_redraw();
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct BlitUniforms {
    magnification: f32,
    _pad: [f32; 7], // std140 alignment (important!)
}

pub struct BackendRenderer {
    gpu: Gpu,
    egui_renderer: egui_wgpu::Renderer,
    scene: Scene,
    bind_group_layout: BindGroupLayout,
    blit_bind_group_layout: BindGroupLayout,
    blit_buffer: Buffer,
}

impl BackendRenderer {
    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let gpu = Gpu::new_async(window, width, height).await;

        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.surface_config.format,
            egui_wgpu::RendererOptions {
                depth_stencil_format: None,
                msaa_samples: 1,
                ..Default::default()
            },
        );

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        let blit_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Texture {
                                multisampled: false,
                                view_dimension: wgpu::TextureViewDimension::D2,
                                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                    label: Some("texture_bind_group_layout"),
                });

        let scene = Scene::new(&gpu.device, gpu.surface_format, &bind_group_layout);

        let blit_uniforms = BlitUniforms {
            magnification: 1.0,
            _pad: [0.0; 7],
        };

        let blit_uniform_buffer =
            gpu.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Blit Uniform Buffer"),
                    contents: bytemuck::bytes_of(&blit_uniforms),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        Self {
            gpu,
            egui_renderer,
            scene,
            bind_group_layout,
            blit_bind_group_layout,
            blit_buffer: blit_uniform_buffer,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
    }

    pub fn render_frame(
        &mut self,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        paint_jobs: Vec<egui::epaint::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
        shared: &mut shared::Shared,
    ) {
        if shared.camera.window == Vec2::new(0., 0.) {
            return;
        }

        let clear = wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: shared.config.colors.background.r as f64 / 255.,
                g: shared.config.colors.background.g as f64 / 255.,
                b: shared.config.colors.background.b as f64 / 255.,
                a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
        };

        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }
        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        #[rustfmt::skip]
        let desc = &wgpu::CommandEncoderDescriptor { label: Some("Render Encoder") };
        let mut encoder = self.gpu.device.create_command_encoder(desc);
        encoder.insert_debug_marker("Render scene");

        #[rustfmt::skip]
        self.egui_renderer.update_buffers(&self.gpu.device, &self.gpu.queue, &mut encoder, &paint_jobs, &screen_descriptor);

        let surface_texture = self.gpu.surface.get_current_texture().unwrap();
        let surface_texture_view =
            surface_texture
                .texture
                .create_view(&wgpu::TextureViewDescriptor {
                    label: wgpu::Label::default(),
                    aspect: wgpu::TextureAspect::default(),
                    format: Some(self.gpu.surface_format),
                    dimension: None,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                    usage: None,
                });

        let uniforms = BlitUniforms {
            magnification: shared.config.pixel_magnification as f32,
            _pad: [0.0; 7],
        };
        let format;
        match self.gpu.surface_format {
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                format = wgpu::TextureFormat::Bgra8Unorm;
            }
            _ => format = wgpu::TextureFormat::Rgba8Unorm,
        }
        let pixel_texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: shared.camera.window.x as u32 / shared.config.pixel_magnification as u32,
                height: shared.camera.window.y as u32 / shared.config.pixel_magnification as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            label: Some("Capture Texture"),
            view_formats: &[],
        });
        let pixel_view = pixel_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut pixel_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &pixel_view,
                resolve_target: None,
                ops: clear,
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pixel_pass.set_pipeline(&self.scene.pipeline);
        self.skf_render(shared, &mut pixel_pass.forget_lifetime());

        let sampler = self.gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Nearest, // pixelated look
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let blit_bind_group = self
            .gpu
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.blit_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&pixel_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.blit_buffer.as_entire_binding(),
                    },
                ],
                label: Some("LowRes Blit BindGroup"),
            });
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture_view,
                resolve_target: None,
                ops: clear,
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.scene.blit_pipeline);
        render_pass.set_bind_group(0, &blit_bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        self.egui_renderer.render(
            &mut render_pass.forget_lifetime(),
            &paint_jobs,
            &screen_descriptor,
        );

        self.gpu
            .queue
            .write_buffer(&self.blit_buffer, 0, bytemuck::bytes_of(&uniforms));
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();
    }

    fn skf_render(&mut self, shared: &mut Shared, render_pass: &mut wgpu::RenderPass) {
        // if the spritesheet timer has initiaed, wait a little for all buffers to complete before saving them
        let elapsed = &mut shared.ui.spritesheet_elapsed;
        let duration_in_millis = 250;

        // keep timer still until all frames have been mapped
        let mut total_frames = 0;
        for sheet in &shared.ui.rendered_spritesheets {
            total_frames += sheet.len();
        }
        if *shared.ui.mapped_frames.lock().unwrap() < total_frames {
            *elapsed = Some(Instant::now());
        }

        if *elapsed != None && elapsed.unwrap().elapsed().as_millis() > duration_in_millis {
            if shared.ui.exporting_video_type != ExportVideoType::None {
                #[rustfmt::skip]
                let bufs = utils::encode_sequence(&shared.armature, &mut shared.ui, self);
                let anim_idx = shared.ui.exporting_video_anim;
                let name = &shared.armature.animations[anim_idx].name;
                let mut _path: String = "".to_string();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let raw_path = &shared.ui.file_path.lock().unwrap()[0];
                    _path = raw_path.as_path().to_str().unwrap().to_string();
                }
                let _ext;
                let ffmpeg_bin = if shared.ui.use_system_ffmpeg {
                    "ffmpeg".to_string()
                } else {
                    #[cfg(target_os = "windows")]
                    {
                        utils::bin_path()
                            .join("ffmpeg.exe")
                            .to_str()
                            .unwrap()
                            .to_string()
                    }
                    #[cfg(not(target_os = "windows"))]
                    {
                        utils::bin_path()
                            .join("ffmpeg")
                            .to_str()
                            .unwrap()
                            .to_string()
                    }
                };
                let size = shared.ui.sprite_size;
                let this_anim = bufs[0].clone();
                let fps = shared.armature.animations[anim_idx].fps;
                if shared.ui.exporting_video_type == ExportVideoType::Gif {
                    shared.ui.custom_error =
                        Self::encode_gif(this_anim, fps, size, name, &_path, ffmpeg_bin);
                    _ext = ".gif";
                } else {
                    let codec_str = match shared.ui.exporting_video_encoder {
                        ExportVideoEncoder::Libx264 => "libx264",
                        ExportVideoEncoder::AV1 => "libsvtav1",
                    };
                    shared.ui.custom_error = Self::encode_video(
                        this_anim, fps, size, name, codec_str, &_path, ffmpeg_bin,
                    );
                    _ext = ".mp4";
                }
                if shared.ui.custom_error != "" {
                    shared.events.open_modal("error_vid_export", false);
                } else if shared.ui.open_after_export {
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Err(e) = open::that(_path + _ext) {
                        println!("{}", e);
                    }
                }
            } else {
                #[rustfmt::skip] #[cfg(not(target_arch = "wasm32"))]
                self._skf_native_spritesheet(&shared.armature, &mut shared.ui);
                #[rustfmt::skip] #[cfg(target_arch = "wasm32")]
                self.skf_web_spritesheet(&shared.armature, &mut shared.ui);
            }

            shared.ui.spritesheet_elapsed = None;
            shared.ui.modal = false;
            shared.ui.sprite_size = shared.screenshot_res;
            *shared.ui.mapped_frames.lock().unwrap() = 0;
        }

        if shared.renderer.generic_bindgroup == None {
            shared.renderer.generic_bindgroup = Some(renderer::create_texture_bind_group(
                vec![255, 255, 255, 255],
                Vec2::new(1., 1.),
                &self.gpu.queue,
                &self.gpu.device,
                &self.bind_group_layout,
            ));
        }
        if *shared.ui.save_finished.lock().unwrap() {
            shared.undo_states.unsaved_undo_actions = shared.undo_states.undo_actions.len();
            shared.undo_states.prev_undo_actions = shared.undo_states.undo_actions.len();
            shared.ui.changed_window_name = false;
            shared.ui.modal = false;
            shared.ui.can_quit = true;
            *shared.ui.save_finished.lock().unwrap() = false;
        } else if *shared.ui.export_finished.lock().unwrap() {
            shared.ui.modal = false;
            shared.ui.can_quit = true;
            *shared.ui.export_finished.lock().unwrap() = false;
        }

        let saving = shared.ui.saving.lock().unwrap().clone();

        let recording_spritesheets = saving == Saving::Spritesheet || saving == Saving::Video;
        if saving != Saving::None && !recording_spritesheets {
            #[cfg(target_arch = "wasm32")]
            {
                let saving_type = shared.ui.saving.lock().unwrap().clone();
                if saving_type == Saving::CustomPath || saving_type == Saving::Exporting {
                    #[rustfmt::skip]
                    utils::save_web(&shared.armature, &shared.camera, &shared.edit_mode, saving_type);
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            self.save(shared);
        } else if recording_spritesheets {
            shared.events.open_modal("exporting", true);
            #[rustfmt::skip]
            utils::render_spritesheets(&shared.armature, &mut shared.ui, &shared.camera, &shared.config, self);
            *shared.ui.saving.lock().unwrap() = Saving::None;
            shared.ui.spritesheet_elapsed = Some(Instant::now());
            shared.ui.export_modal = false;
        }

        for b in 0..shared.armature.bones.len() {
            let tex = shared.armature.tex_of(shared.armature.bones[b].id);
            if tex != None && shared.armature.bones[b].vertices.len() == 0 {
                let size = tex.unwrap().size;
                let bone = &mut shared.armature.bones[b];
                (bone.vertices, bone.indices) = renderer::create_tex_rect(&size);
                shared.armature.bones[b].verts_edited = false;
            }
        }

        // core rendering logic handled in renderer.rs
        let s = shared;
        #[rustfmt::skip]
        renderer::render(render_pass, &self.gpu.device, &s.camera, &s.input, &mut s.armature, &s.config, &s.edit_mode, &mut s.selections, &mut s.renderer, &mut s.events,);

        s.ui.warnings = warnings::check_warnings(&s.armature);
    }

    fn _skf_native_spritesheet(&self, armature: &Armature, shared_ui: &mut Ui) {
        let path = shared_ui.file_path.lock().unwrap()[0].clone();
        let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let options = zip::write::FullFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        #[rustfmt::skip]
        self.skf_pack_sprites(armature, shared_ui, &mut zip, &options);

        _ = zip.finish();
    }

    #[cfg(target_arch = "wasm32")]
    fn skf_web_spritesheet(&self, armature: &Armature, shared_ui: &mut Ui) {
        let mut buf: Vec<u8> = Vec::new();
        let cursor = std::io::Cursor::new(&mut buf);
        let mut zip = zip::ZipWriter::new(cursor);
        let options = zip::write::FullFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);

        #[rustfmt::skip]
        self.skf_pack_sprites(armature, shared_ui, &mut zip, &options);

        let bytes = zip.finish().unwrap().into_inner().to_vec();
        downloadZip(bytes, Saving::Spritesheet.to_string());
    }

    // sprite-packing stuff that applies to both native and web
    fn skf_pack_sprites<W: Write + Seek>(
        &self,
        armature: &Armature,
        shared_ui: &mut Ui,
        zip: &mut zip::ZipWriter<W>,
        options: &FullFileOptions,
    ) {
        if shared_ui.image_sequences {
            let bufs = utils::encode_sequence(armature, shared_ui, self);

            let mut buf_idx = 0;
            for a in 0..shared_ui.exporting_anims.len() {
                if !shared_ui.exporting_anims[a] {
                    continue;
                }
                zip.add_directory(armature.animations[a].name.clone(), options.clone())
                    .unwrap();
                for b in 0..bufs[buf_idx].len() {
                    let png_name =
                        armature.animations[a].name.to_string() + "/" + &b.to_string() + ".png";
                    zip.start_file(png_name, options.clone()).unwrap();
                    zip.write(&bufs[buf_idx][b]).unwrap();
                }
                buf_idx += 1;
            }
        } else {
            let bufs = utils::encode_spritesheets(armature, shared_ui, self);
            for b in 0..bufs.len() {
                let png_name = b.to_string() + ".png";
                zip.start_file(png_name, options.clone()).unwrap();
                zip.write(&bufs[b]).unwrap();
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&mut self, shared: &mut Shared) {
        let mut save_path = "".to_string();
        let saving_type = shared.ui.saving.lock().unwrap().clone();
        match saving_type {
            Saving::CustomPath => {
                let path = &shared.ui.file_path.lock().unwrap()[0];
                save_path = path.as_path().to_str().unwrap().to_string();
                shared.ui.changed_window_name = false;
                shared.events.open_modal("saving", true);
                *shared.ui.save_finished.lock().unwrap() = false;
                shared.ui.save_path = Some(path.clone());
                shared.ui.can_quit = false;
                if !shared.ui.recent_file_paths.contains(&save_path) {
                    shared.ui.recent_file_paths.push(save_path.clone());
                }
            }
            Saving::Exporting => {
                let path = &shared.ui.file_path.lock().unwrap()[0];
                save_path = path.as_path().to_str().unwrap().to_string();
                shared.events.open_modal("exporting", true);
                *shared.ui.export_finished.lock().unwrap() = false;
                shared.ui.can_quit = false;
                utils::save_to_recent_files(&shared.ui.recent_file_paths);
            }
            Saving::Autosaving => {
                let dir_init = directories_next::ProjectDirs::from("com", "retropaint", "skelform");
                let dir = dir_init.unwrap().data_dir().to_str().unwrap().to_string();
                save_path = dir + "/autosave.skf";
                let freq = shared.config.autosave_frequency as f32;
                let in_cooldown = shared.edit_mode.time - shared.last_autosave < freq;
                if in_cooldown {
                    *shared.ui.saving.lock().unwrap() = Saving::None;
                    return;
                }
                if !shared.ui.recent_file_paths.contains(&save_path) {
                    shared.ui.recent_file_paths.push(save_path.clone());
                }
                shared.last_autosave = shared.edit_mode.time;
            }
            _ => {}
        }

        utils::save_to_recent_files(&shared.ui.recent_file_paths);

        let mut frames = vec![];
        #[rustfmt::skip]
        self.take_screenshot(shared.screenshot_res, &shared.armature, &shared.camera, &shared.config.colors.background, &mut frames, &mut shared.ui.mapped_frames, &shared.config);
        let buffer = frames[0].buffer.clone();
        let screenshot_res = shared.screenshot_res;

        let mut armature = shared.armature.clone();
        let camera = shared.camera.clone();
        let edit_mode = shared.edit_mode.clone();

        // clear export options
        shared.edit_mode.export_bake_ik = false;
        shared.edit_mode.export_exclude_ik = false;
        shared.edit_mode.export_clear_color = Color::new(0, 0, 0, 0);
        shared.edit_mode.export_img_format = ExportImgFormat::PNG;

        let was_exporting = *shared.ui.saving.lock().unwrap() == Saving::Exporting;
        let autosaving = *shared.ui.saving.lock().unwrap() == Saving::Autosaving;
        *shared.ui.saving.lock().unwrap() = Saving::None;

        let save_finished = Arc::clone(&shared.ui.save_finished);
        let export_finished = Arc::clone(&shared.ui.export_finished);
        let device = self.gpu.device.clone();
        let surface_format = self.gpu.surface_format;
        std::thread::spawn(move || {
            let mut png_bufs = vec![];
            let mut sizes = vec![];

            if armature.styles.len() > 0 && armature.styles[0].textures.len() > 0 {
                (png_bufs, sizes) = utils::create_tex_sheet(&mut armature, &edit_mode);
            }

            let (armatures_json, editor_json) =
                utils::prepare_files(&armature, camera, sizes.clone(), &edit_mode);

            // create zip file
            let mut zip = zip::ZipWriter::new(std::fs::File::create(save_path.clone()).unwrap());

            let options = zip::write::FullFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            let thumb_buf =
                utils::process_screenshot(&buffer, &device, surface_format, screenshot_res);

            // save relevant files into the zip
            zip.start_file("armature.json", options.clone()).unwrap();
            zip.write(armatures_json.as_bytes()).unwrap();
            zip.start_file("editor.json", options.clone()).unwrap();
            zip.write(editor_json.as_bytes()).unwrap();
            zip.start_file("thumbnail.png", options.clone()).unwrap();
            zip.write(&thumb_buf).unwrap();
            zip.start_file("readme.md", options.clone()).unwrap();
            zip.write(include_bytes!("../assets/skf_readme.md"))
                .unwrap();
            let atlas_ext = match edit_mode.export_img_format {
                ExportImgFormat::PNG => ".png",
                ExportImgFormat::JPG => ".jpg",
            };
            for i in 0..png_bufs.len() {
                let atlas_name = "atlas".to_owned() + &i.to_string() + atlas_ext;
                zip.start_file(atlas_name, options.clone()).unwrap();
                zip.write(&png_bufs[i]).unwrap();
            }

            zip.finish().unwrap();

            let _ = std::fs::copy(save_path.clone(), save_path + "~");

            // trigger saving modal if manually saving
            if !autosaving {
                if was_exporting {
                    *export_finished.lock().unwrap() = true;
                } else {
                    *save_finished.lock().unwrap() = true;
                }
            }
        });
    }

    pub fn take_screenshot(
        &self,
        screenshot_res: Vec2,
        armature: &Armature,
        camera: &Camera,
        clear_color: &Color,
        rendered_frames: &mut Vec<RenderedFrame>,
        mapped_frames: &mut Arc<Mutex<usize>>,
        config: &Config,
    ) {
        let width = screenshot_res.x as u32;
        let height = screenshot_res.y as u32;

        let format;
        match self.gpu.surface_format {
            wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
                format = wgpu::TextureFormat::Bgra8Unorm;
            }
            _ => format = wgpu::TextureFormat::Rgba8Unorm,
        }
        let capture_texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: Some("Capture Texture"),
            view_formats: &[],
        });

        let capture_view = capture_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let device = &self.gpu.device;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut capture_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Capture Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &capture_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color.r as f64 / 255.,
                            g: clear_color.g as f64 / 255.,
                            b: clear_color.b as f64 / 255.,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            capture_pass.set_pipeline(&self.scene.pipeline);

            // core rendering logic handled in renderer.rs
            renderer::render_screenshot(
                &mut capture_pass,
                &self.gpu.device,
                &armature,
                &camera,
                &config,
            );
        }

        // pad screenshot width to a multiple of 256
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as u32;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;

        let buffer_size = (padded_bytes_per_row * height * 4) as u64;
        let output_buffer = self.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &capture_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        rendered_frames.push(RenderedFrame {
            buffer: output_buffer.clone(),
            width,
            height,
        });

        let arc_mapped_frames = Arc::clone(mapped_frames);
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        let buffer_slice = output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            if let Ok(()) = result {
                *arc_mapped_frames.lock().unwrap() += 1;
            } else {
                println!("Failed to map buffer for read.");
            }
        });
    }

    fn encode_video(
        rendered_frames: Vec<Vec<u8>>,
        fps: i32,
        window: Vec2,
        _name: &str,
        _codec: &str,
        _path: &String,
        _ffmpeg_bin: String,
    ) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            #[rustfmt::skip]
            let mut child = Command::new(_ffmpeg_bin)
                .args(["-y", "-f", "rawvideo", "-pixel_format", "rgba", "-video_size", &format!("{}x{}", window.x, window.y), 
                    "-framerate", &fps.to_string(), "-i", "pipe:0", 
                    // disabled: manual encoder codec - not needed for now
                    // "-c:v", codec, 
                    "-pix_fmt", "yuv420p",
                    &(_path.to_owned() + &".mp4")])
                //.stderr(Stdio::piped())
                .stdin(Stdio::piped())
                .spawn();

            if let Err(e) = child {
                return "spawn ffmpeg: ".to_owned() + &e.to_string();
            }

            {
                let stdin = child.as_mut().unwrap().stdin.as_mut().unwrap();
                for frame in &rendered_frames {
                    let rgb = image::load_from_memory(&frame).unwrap();
                    if let Err(e) = stdin.write_all(&rgb.to_rgba8()) {
                        return "stdin: ".to_owned() + &e.to_string();
                    }
                }
            }

            drop(child.as_mut().unwrap().stdin.take());
            child.as_mut().unwrap().wait().unwrap();
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut raw_video = vec![];
            for frame in rendered_frames {
                let rgb = image::load_from_memory(&frame).unwrap();
                for chunk in rgb.to_rgba8().chunks_exact(4) {
                    raw_video.push(chunk[0]);
                    raw_video.push(chunk[1]);
                    raw_video.push(chunk[2]);
                }
            }
            downloadMp4(raw_video, window.x, window.y, _name, fps);
        }

        "".to_string()
    }

    fn encode_gif(
        rendered_frames: Vec<Vec<u8>>,
        fps: i32,
        window: Vec2,
        _name: &str,
        _path: &String,
        _ffmpeg_bin: String,
    ) -> String {
        #[cfg(target_arch = "wasm32")]
        {
            for frame in rendered_frames {
                addGifFrame(frame);
            }
            downloadGif(window.x, window.y, _name, fps);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            #[rustfmt::skip]
            let mut child = Command::new(_ffmpeg_bin)
            .args(["-y", "-f", "rawvideo", "-pixel_format", "rgba", "-video_size", &format!("{}x{}", window.x, window.y), 
                "-framerate", &fps.to_string(), "-i", "pipe:0", "-filter_complex",
                "[0:v] fps=30,split [a][b]; \
                 [a] palettegen=stats_mode=diff [p]; \
                 [b][p] paletteuse=dither=sierra2_4a",
                "-loop", "0",
                &(_path.to_owned() + &".gif")
            ])
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();

            if let Err(e) = child {
                return "spawn ffmpeg: ".to_owned() + &e.to_string();
            }

            {
                let stdin = child.as_mut().unwrap().stdin.as_mut().unwrap();
                for frame in &rendered_frames {
                    let rgb = image::load_from_memory(&frame).unwrap();
                    if let Err(e) = stdin.write_all(&rgb.to_rgba8()) {
                        return "stdin: ".to_string() + &e.to_string();
                    }
                }
            }

            child.as_mut().unwrap().wait().unwrap();
            let _ = std::fs::remove_file("palette.png");
        }

        "".to_string()
    }
}

pub struct Gpu {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface_format: wgpu::TextureFormat,
}

impl Gpu {
    pub fn aspect_ratio(&self) -> f32 {
        self.surface_config.width as f32 / self.surface_config.height.max(1) as f32
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub async fn new_async(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let surface: wgpu::Surface;
        #[allow(unused_mut, unused_assignments)]
        let mut instance = wgpu::Instance::new(&InstanceDescriptor::default());

        // force DX12 on Windows
        #[cfg(target_os = "windows")]
        {
            let backends = wgpu::Backend::Dx12;
            instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: backends.into(),
                ..Default::default()
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::GL,
                ..Default::default()
            });
        }

        surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = {
            adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("WGPU Device"),
                    memory_hints: wgpu::MemoryHints::default(),
                    required_features: wgpu::Features::default(),
                    #[cfg(not(target_arch = "wasm32"))]
                    required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                    #[cfg(all(target_arch = "wasm32"))]
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    trace: wgpu::Trace::Off,
                    experimental_features: ExperimentalFeatures::disabled(),
                })
                .await
                .expect("Failed to request a device!")
        };

        let surface_capabilities = surface.get_capabilities(&adapter);

        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb()) // egui wants a non-srgb surface texture
            .unwrap_or(surface_capabilities.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        Self {
            surface,
            device,
            queue,
            surface_config,
            surface_format,
        }
    }
}

struct Scene {
    pub pipeline: wgpu::RenderPipeline,
    pub blit_pipeline: wgpu::RenderPipeline,
}

impl Scene {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let pipeline = Self::create_pipeline(device, surface_format, &bind_group_layout);
        let blit_pipeline = Self::create_blit_pipeline(device, surface_format);

        Self {
            pipeline,
            blit_pipeline,
        }
    }

    fn create_blit_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Blit Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blit.wgsl").into()),
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),

            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[], // fullscreen triangle, no vertex buffer
                compilation_options: Default::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },

            depth_stencil: None,

            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },

            multiview: None,
            cache: None,
        })
    }

    fn create_pipeline(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        bind_group_layout: &BindGroupLayout,
    ) -> wgpu::RenderPipeline {
        let shader_str = &String::from_utf8(include_bytes!("shader.wgsl").to_vec())
            .unwrap()
            .to_string();
        let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_str)),
        });

        let attributes =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4].to_vec();
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &attributes,
        };

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("test"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vertex_main"),
                buffers: &[vertex_layout],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::shared::{AnimElement, Shared, Vec2};
    use crate::{armature_window, file_reader};

    fn init_shared() -> Shared {
        let mut shared = Shared::default();
        shared.armature.new_bone(-1);
        shared.armature.new_bone(-1);
        shared.armature.new_bone(-1);
        armature_window::drag_bone(&mut shared.armature, false, 2, 1);
        armature_window::drag_bone(&mut shared.armature, false, 1, 0);
        shared.ui.init_empty_loc();
        shared
    }

    // todo: add headless wgpu and egui to test those that depend on it

    #[test]
    fn import_skf() {
        let mut shared = init_shared();
        *shared.ui.file_name.lock().unwrap() = "./samples/skellington.skf".to_string();
        *shared.ui.import_contents.lock().unwrap() = vec![0];
        file_reader::read_import(&mut shared, None, None, None, None);
        assert_eq!(shared.armature.bones[0].name != "New Bone", true);
        assert_eq!(shared.armature.styles.len() > 0, true);
    }

    // check if exported skf is same as imported
    #[test]
    fn export_skf() {
        let mut shared = init_shared();
        *shared.ui.file_name.lock().unwrap() = "./samples/skellington.skf".to_string();
        *shared.ui.import_contents.lock().unwrap() = vec![0];
        file_reader::read_import(&mut shared, None, None, None, None);
        assert_eq!(shared.armature.bones[0].name != "New Bone", true);
        assert_eq!(shared.armature.styles.len() > 0, true);
    }

    #[test]
    fn import_psd() {
        let mut shared = init_shared();
        *shared.ui.file_name.lock().unwrap() = "./samples/skellington.psd".to_string();
        *shared.ui.import_contents.lock().unwrap() = vec![0];
        file_reader::read_import(&mut shared, None, None, None, None);
        assert_eq!(shared.armature.bones[0].name != "New Bone", true);
        assert_eq!(shared.armature.styles.len() > 0, true);
    }

    #[test]
    fn drag_bone_above() {
        let mut shared = init_shared();
        shared.armature.bones[0].name = "Bone0".to_string();
        shared.armature.bones[1].name = "Bone1".to_string();
        shared.armature.bones[2].name = "Bone2".to_string();
        armature_window::drag_bone(&mut shared, true, 2, 1);
        assert_eq!(shared.armature.bones[1].name, "Bone2");
    }

    #[test]
    fn drag_bone_directly() {
        let mut shared = init_shared();
        shared.armature.bones[0].name = "Bone0".to_string();
        shared.armature.bones[1].name = "Bone1".to_string();
        shared.armature.bones[2].name = "Bone2".to_string();
        armature_window::drag_bone(&mut shared, false, 2, 1);
        assert_eq!(shared.armature.bones[2].parent_id, 1);
    }
}
