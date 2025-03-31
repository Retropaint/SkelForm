// #![windows_subsystem = "windows"] // uncomment this to suppress terminal on windows

use skellar::shared::Shared;

fn main() -> Result<(), winit::error::EventLoopError> {
    let event_loop = winit::event_loop::EventLoop::builder().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = skellar::App::default();
    init_shared(&mut app.shared);
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn init_shared(shared: &mut Shared) {
    shared.selected_bone = usize::MAX;
    shared.input.mouse_left = -1;
    shared.input.modifier = -1;
    shared.debug = true;
    shared.camera.zoom = 1.;
    shared.zoom = 1.;
    shared.animating = true;
    shared.ui.anim.selected = usize::MAX;
    shared.ui.anim.timeline_zoom = 1.;
}
