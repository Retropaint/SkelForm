// #![windows_subsystem = "windows"] // uncomment this to suppress terminal on windows

fn main() -> Result<(), winit::error::EventLoopError> {
    let event_loop = winit::event_loop::EventLoop::builder().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    let mut app = skeller::App::default();
    app.shared.selected_bone = usize::MAX;
    app.shared.mouse_left = -1;
    event_loop.run_app(&mut app)?;
    Ok(())
}
