// #![windows_subsystem = "windows"] // uncomment this to suppress terminal on windows

use skelform_lib::shared::*;

#[cfg(not(target_arch = "wasm32"))]
use skelform_lib::file_reader;

fn main() -> Result<(), winit::error::EventLoopError> {
    // uncomment below to get console panic hook as early as possible for debugging
    //
    // otherwise, it's activated in lib.rs

    //#[cfg(target_arch = "wasm32")]
    //{
    //    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    //    console_log::init().expect("Failed to initialize logger!");
    //    log::info!("test");
    //}

    let mut app = skelform_lib::App::default();
    init_shared(&mut app.shared);

    let event_loop = winit::event_loop::EventLoop::builder().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    // delete any leftover temporary files
    #[cfg(not(target_arch = "wasm32"))]
    file_reader::del_temp_files();

    event_loop.run_app(&mut app)
}

fn init_shared(shared: &mut Shared) {
    shared.selected_bone_idx = usize::MAX;
    shared.input.mouse_left = -1;
    shared.debug = false;
    shared.camera.zoom = 5.;
    shared.ui.anim.selected = usize::MAX;
    shared.ui.anim.timeline_zoom = 1.;
    shared.ui.anim.exported_frame = "".to_string();
    shared.ui.anim.selected_frame = -1;

    // if this were false, the first click would always
    // be considered non-UI
    shared.input.on_ui = true;

    //shared.start_time = Some(std::time::Instant::now());
}
