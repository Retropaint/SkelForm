use shared::*;
use wgpu::{BindGroupLayout, InstanceDescriptor};

// native-only imports
#[cfg(not(target_arch = "wasm32"))]
mod native {
    pub use crate::file_reader::*;
    pub use image::*;
    pub use std::fs;
    pub use std::io::Write;
    pub use std::time::Instant;
}
#[cfg(not(target_arch = "wasm32"))]
use native::*;

use std::io::BufRead;
// native-only imports
#[cfg(target_arch = "wasm32")]
mod web {
    pub use wasm_bindgen::*;
    pub use web_sys::*;
    pub use web_time::Instant;
}
#[cfg(target_arch = "wasm32")]
use web::*;

use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{MouseButton, MouseScrollDelta, WindowEvent},
    window::{Theme, Window},
};

pub mod armature_window;
pub mod bone_window;
pub mod file_reader;
pub mod input;
pub mod keyframe_editor;
pub mod renderer;
pub mod shared;
pub mod ui;
pub mod utils;

#[derive(Default)]
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    gui_state: Option<egui_winit::State>,
    last_render_time: Option<Instant>,
    #[cfg(target_arch = "wasm32")]
    pub renderer_receiver: Option<futures::channel::oneshot::Receiver<Renderer>>,
    last_size: (u32, u32),
    pub shared: shared::Shared,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut attributes = Window::default_attributes();

        #[cfg(not(target_arch = "wasm32"))]
        {
            attributes = attributes.with_title("Skellar");
        }

        #[allow(unused_assignments)]
        #[cfg(target_arch = "wasm32")]
        let (mut canvas_width, mut canvas_height) = (0, 0);

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
            canvas_width = canvas.width();
            canvas_height = canvas.height();
            self.last_size = (canvas_width, canvas_height);
            attributes = attributes.with_canvas(Some(canvas));
        }

        if let Ok(window) = event_loop.create_window(attributes) {
            let first_window_handle = self.window.is_none();
            let window_handle = Arc::new(window);
            self.window = Some(window_handle.clone());
            if first_window_handle {
                let gui_context = egui::Context::default();

                // turn off egui kb zoom
                gui_context.options_mut(|op| {
                    op.zoom_with_keyboard = false;
                });

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let inner_size = window_handle.inner_size();
                    self.last_size = (inner_size.width, inner_size.height);
                }

                #[cfg(target_arch = "wasm32")]
                {
                    gui_context.set_pixels_per_point(window_handle.scale_factor() as f32);
                }

                let viewport_id = gui_context.viewport_id();
                let gui_state = egui_winit::State::new(
                    gui_context,
                    viewport_id,
                    &window_handle,
                    Some(window_handle.scale_factor() as _),
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
                        Renderer::new(window_handle.clone(), width, height).await
                    });
                    self.renderer = Some(renderer);
                }

                #[cfg(target_arch = "wasm32")]
                {
                    let (sender, receiver) = futures::channel::oneshot::channel();
                    self.renderer_receiver = Some(receiver);
                    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
                    console_log::init().expect("Failed to initialize logger!");
                    log::info!("Canvas dimensions: ({canvas_width} x {canvas_height})");
                    wasm_bindgen_futures::spawn_local(async move {
                        let renderer =
                            Renderer::new(window_handle.clone(), canvas_width, canvas_height).await;
                        if sender.send(renderer).is_err() {
                            log::error!("Failed to create and send renderer!");
                        }
                    });
                }

                self.gui_state = Some(gui_state);
                self.last_render_time = Some(Instant::now());
            }
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

        // temp file reading stuff
        if let Some(_) = self.renderer.as_ref() {
            file_reader::read_image_loaders(
                &mut self.shared,
                &self.renderer.as_ref().unwrap().gpu.queue,
                &self.renderer.as_ref().unwrap().gpu.device,
                &self.renderer.as_ref().unwrap().bind_group_layout,
            );

            #[cfg(not(target_arch = "wasm32"))]
            {
                file_reader::read_export(&self.shared);
                file_reader::read_import(
                    &mut self.shared,
                    &self.renderer.as_ref().unwrap().gpu.queue,
                    &self.renderer.as_ref().unwrap().gpu.device,
                    &self.renderer.as_ref().unwrap().bind_group_layout,
                );
                file_reader::read_exported_video_frame(&mut self.shared);
            }
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
            return;
        }

        // increase mouse_left if it's being held down
        if self.shared.input.mouse_left >= 0 {
            self.shared.input.mouse_left += 1;
        }

        // If the gui didn't consume the event, handle it
        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        state,
                        ..
                    },
                ..
            } => {
                input::keyboard_input(&key_code, &state, &mut self.shared);
            }
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                #[cfg(target_arch = "wasm32")]
                log::info!("Resizing renderer surface to: ({width}, {height})");
                renderer.resize(width, height);
                self.last_size = (width, height);
                self.shared.window = Vec2::new(self.last_size.0 as f32, self.last_size.1 as f32);
            }
            WindowEvent::CloseRequested => {
                #[cfg(target_arch = "wasm32")]
                log::info!("Close requested. Exiting...");
                event_loop.exit();
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.shared.input.mouse = Vec2::new(position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                input::mouse_input(&button, &state, &mut self.shared);
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _phase,
            } => {
                input::mouse_wheel_input(delta, &mut self.shared);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                *last_render_time = now;

                let input = gui_state.take_egui_input(window);
                gui_state.egui_ctx().begin_pass(input);

                // ui logic handled in ui.rs
                ui::draw(gui_state.egui_ctx(), &mut self.shared);

                let egui_winit::egui::FullOutput {
                    textures_delta,
                    shapes,
                    pixels_per_point,
                    platform_output,
                    ..
                } = gui_state.egui_ctx().end_pass();

                gui_state.handle_platform_output(window, platform_output);

                let paint_jobs = gui_state.egui_ctx().tessellate(shapes, pixels_per_point);

                let screen_descriptor = {
                    let (width, height) = self.last_size;
                    egui_wgpu::ScreenDescriptor {
                        size_in_pixels: [width, height],
                        pixels_per_point: window.scale_factor() as f32,
                    }
                };

                renderer.render_frame(
                    screen_descriptor,
                    paint_jobs,
                    textures_delta,
                    &mut self.shared,
                );
            }
            _ => (),
        }

        if self.shared.highlight_bindgroup == None {
            self.shared.highlight_bindgroup = Some(renderer::create_texture_bind_group(
                vec![255, 255, 255, 100],
                Vec2::new(1., 1.),
                &self.renderer.as_ref().unwrap().gpu.queue,
                &self.renderer.as_ref().unwrap().gpu.device,
                &self.renderer.as_ref().unwrap().bind_group_layout,
            ));
        }

        #[cfg(not(target_arch = "wasm32"))]
        if self.shared.debug {
            self.shared.debug = false;
            self.shared.selected_bone_idx = 0;
            armature_window::new_bone(&mut self.shared.armature.bones);
            armature_window::new_bone(&mut self.shared.armature.bones);
            armature_window::new_bone(&mut self.shared.armature.bones);
            self.shared.armature.bones[2].parent_id = 1;
            self.shared.armature.bones[1].parent_id = 0;
            self.shared.armature.animations.push(Animation {
                name: "lol".to_string(),
                keyframes: vec![],
                fps: 24,
                ..Default::default()
            });

            file_reader::create_temp_file(
                TEMP_IMG_PATH,
                "/Users/o/projects/code/rust/skelform_wgpu/gopher.png",
            );
        }

        window.request_redraw();
    }
}

pub struct Renderer {
    gpu: Gpu,
    depth_texture_view: wgpu::TextureView,
    egui_renderer: egui_wgpu::Renderer,
    scene: Scene,
    bind_group_layout: BindGroupLayout,
}

impl Renderer {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub async fn new(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let gpu = Gpu::new_async(window, width, height).await;
        let depth_texture_view = gpu.create_depth_texture(width, height);

        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.surface_config.format,
            Some(Self::DEPTH_FORMAT),
            1,
            false,
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

        let scene = Scene::new(&gpu.device, gpu.surface_format, &bind_group_layout);

        Self {
            gpu,
            depth_texture_view,
            egui_renderer,
            scene,
            bind_group_layout,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.depth_texture_view = self.gpu.create_depth_texture(width, height);
    }

    pub fn render_frame(
        &mut self,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
        paint_jobs: Vec<egui::epaint::ClippedPrimitive>,
        textures_delta: egui::TexturesDelta,
        shared: &mut shared::Shared,
    ) {
        for (id, image_delta) in &textures_delta.set {
            self.egui_renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }

        for id in &textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        self.egui_renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );

        let surface_texture = self
            .gpu
            .surface
            .get_current_texture()
            .expect("Failed to get surface texture!");

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

        encoder.insert_debug_marker("Render scene");

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_texture_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.2,
                        g: 0.2,
                        b: 0.2,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.scene.pipeline);

        // core rendering logic handled in renderer.rs
        renderer::render(&mut render_pass, &self.gpu.device, shared);

        self.egui_renderer.render(
            &mut render_pass.forget_lifetime(),
            &paint_jobs,
            &screen_descriptor,
        );
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        surface_texture.present();

        if shared.recording {
            self.take_screenshot(shared);
        } else if shared.done_recording {
            let frames = shared.rendered_frames.clone();
            let window = shared.window.clone();
            std::thread::spawn(move || {
                Self::export_video(frames, window);
            });
            shared.done_recording = false;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn take_screenshot(&mut self, shared: &mut shared::Shared) {
        let width = shared.window.x as u32;
        let height = shared.window.y as u32;

        let capture_texture = self.gpu.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            label: Some("Capture Texture"),
            view_formats: &[],
        });

        let capture_view = capture_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy to Buffer Encoder"),
            });

        {
            let mut capture_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Capture Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &capture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.8,
                            g: 0.8,
                            b: 0.8,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            capture_pass.set_pipeline(&self.scene.pipeline);

            // core rendering logic handled in renderer.rs
            renderer::render(&mut capture_pass, &self.gpu.device, shared);
        }

        let buffer_size = (width * height * 4) as u64;
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
                    bytes_per_row: Some(4 * width),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        shared.rendered_frames.push(RenderedFrame {
            buffer: output_buffer.clone(),
            width,
            height,
        });

        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        let buffer_slice = output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |result| {
            if let Ok(()) = result {
            } else {
                panic!("Failed to map buffer for read.");
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_video(rendered_frames: Vec<RenderedFrame>, window: Vec2) {
        let width = rendered_frames[0].width.to_string();
        let height = rendered_frames[0].height.to_string();

        let output_width = width.clone();
        let output_height = height.clone();

        let mut child = std::process::Command::new("ffmpeg")
            .args([
                "-f",
                "rawvideo",
                // input resolution
                "-video_size",
                &(width + "x" + &height),
                // fps
                "-r",
                "60",
                "-pixel_format",
                "rgb24",
                "-i",
                "-",
                // output resolution
                "-s",
                &("".to_owned() + &output_width.to_string() + ":" + &output_height.to_string()),
                // fast preset
                "-preset",
                "veryfast",
                // don't encode audio
                "-c:a",
                "copy",
                "-y",
                "output.mp4",
                "-loglevel",
                "verbose",
            ])
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .unwrap();

        let mut stdin = child.stdin.take().unwrap();

        for i in 0..rendered_frames.len() {
            let buffer_slice = rendered_frames[i].buffer.slice(..);
            let view = buffer_slice.get_mapped_range();

            let mut rgb = vec![0u8; (window.x * window.y * 3.) as usize];
            for (j, chunk) in view.as_ref().chunks_exact(4).enumerate() {
                let offset = j * 3;
                rgb[offset + 0] = chunk[2];
                rgb[offset + 1] = chunk[1];
                rgb[offset + 2] = chunk[0];
            }

            let img = <image::ImageBuffer<image::Rgb<u8>, _>>::from_raw(
                window.x as u32,
                window.y as u32,
                rgb,
            );

            stdin.write_all(img.as_ref().unwrap()).unwrap();

            let frame = i.to_string();
            let headline = "Exporting... ".to_owned()
                + &frame.to_owned()
                + " out of "
                + &(rendered_frames.len() - 1).to_string()
                + " frames";
            if i != rendered_frames.len() - 1 {
                file_reader::create_temp_file(TEMP_EXPORT_VID_TEXT, &headline);
            }
        }

        file_reader::create_temp_file(TEMP_EXPORT_VID_TEXT, EXPORT_VID_DONE);

        stdin.flush().unwrap();
        drop(stdin);
        child.wait().unwrap();
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

    pub fn create_depth_texture(&self, width: u32, height: u32) -> wgpu::TextureView {
        let texture = self.device.create_texture(
            &(wgpu::TextureDescriptor {
                label: Some("Depth Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
        );
        texture.create_view(&wgpu::TextureViewDescriptor {
            label: None,
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            base_array_layer: 0,
            array_layer_count: None,
            mip_level_count: None,
            usage: None,
        })
    }

    pub async fn new_async(
        window: impl Into<wgpu::SurfaceTarget<'static>>,
        width: u32,
        height: u32,
    ) -> Self {
        let mut surface: wgpu::Surface;
        let mut instance: wgpu::Instance;

        // force DX12 on Windows
        #[cfg(target_os = "windows")]
        {
            let backends = wgpu::Backend::Dx12;
            instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends,
                ..Default::default()
            });
        }

        #[cfg(not(target_os = "windows"))]
        {
            instance = wgpu::Instance::new(&InstanceDescriptor::default());
        }

        surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to request adapter!");
        let (device, queue) = {
            #[cfg(target_arch = "wasm32")]
            log::info!("WGPU Adapter Features: {:#?}", adapter.features());
            adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("WGPU Device"),
                        memory_hints: wgpu::MemoryHints::default(),
                        required_features: wgpu::Features::default(),
                        #[cfg(not(target_arch = "wasm32"))]
                        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                        #[cfg(all(target_arch = "wasm32", feature = "webgpu"))]
                        required_limits: wgpu::Limits::default().using_resolution(adapter.limits()),
                        #[cfg(all(target_arch = "wasm32", feature = "webgl"))]
                        required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                            .using_resolution(adapter.limits()),
                    },
                    None,
                )
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
            present_mode: surface_capabilities.present_modes[0],
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
}

impl Scene {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        bind_group_layout: &BindGroupLayout,
    ) -> Self {
        let pipeline = Self::create_pipeline(device, surface_format, &bind_group_layout);

        Self { pipeline }
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

        let attributes = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2].to_vec();
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint32),
                front_face: wgpu::FrontFace::Cw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Renderer::DEPTH_FORMAT,
                depth_write_enabled: false, // disabled for transparency
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
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
