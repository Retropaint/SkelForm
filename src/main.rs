#![windows_subsystem = "windows"]

use global_hotkey::hotkey::{Code, HotKey, Modifiers};

use skelform_lib::{shared::*, utils};

use std::any::Any;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::process::Command;

pub const CRASHLOG_END: &str = "###";

fn main() -> Result<(), winit::error::EventLoopError> {
    #[cfg(all(not(debug_assertions), not(target_arch = "wasm32")))]
    install_panic_handler();

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("Failed to initialize logger!");
    }

    let mut app = skelform_lib::App::default();

    init_shared(&mut app.shared);

    // open 'SkelForm has crashed' modal if there's an untagged crash log
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(does) = fs::exists(utils::crashlog_file()) {
        if does {
            let contents = fs::read_to_string(utils::crashlog_file()).unwrap();
            let last_line = contents.lines().last().map(|s| s.to_string()).unwrap();
            if last_line != CRASHLOG_END {
                app.shared
                    .events
                    .open_polar_modal(PolarId::OpenCrashlog, app.shared.ui.loc("crashed"));
            }
        }
    }

    // tag crashlog to indicate it's been read
    #[cfg(not(target_arch = "wasm32"))]
    {
        let file = std::fs::OpenOptions::new()
            .append(true)
            .open(utils::crashlog_file());
        if let Ok(mut f) = file {
            let contents = fs::read_to_string(utils::crashlog_file()).unwrap();
            let last_line = contents.lines().last().map(|s| s.to_string()).unwrap();
            if last_line != CRASHLOG_END {
                writeln!(&mut f, "{}", CRASHLOG_END).unwrap();
            }
        }
    }

    // load startup.json, but only if no args were given
    let startup: Startup;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.len() == 1 {
            let bytes = include_bytes!("../assets/startup.json").as_slice();
            startup = serde_json::from_slice(bytes).unwrap();
        } else {
            startup = Startup::default();
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let bytes = include_bytes!("../assets/startup.json").as_slice();
        startup = serde_json::from_slice(bytes).unwrap();
    }
    app.shared.ui.startup = startup;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().collect();

        // load .skf based on first arg
        if args.len() > 1 {
            let mut buf = PathBuf::new();
            buf.push(&args[1].to_string());
            *app.shared.ui.file_path.lock().unwrap() = vec![buf];
            *app.shared.ui.file_type.lock().unwrap() = 2;
        }
    }

    let event_loop = winit::event_loop::EventLoop::builder().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    app.shared.input.hotkey_manager = Some(global_hotkey::GlobalHotKeyManager::new().unwrap());

    app.shared.input.mod_q = Some(HotKey::new(Some(Modifiers::SUPER), Code::KeyQ));
    app.shared.input.mod_w = Some(HotKey::new(Some(Modifiers::SUPER), Code::KeyW));

    event_loop.run_app(&mut app)
}

fn init_shared(shared: &mut Shared) {
    shared.selections.bone_idx = usize::MAX;
    shared.camera.zoom = 2000.;
    shared.selections.anim = usize::MAX;
    shared.ui.anim.timeline_zoom = 1.;
    shared.ui.anim.exported_frame = "".to_string();
    shared.selections.anim_frame = -1;
    shared.ui.anim.dragged_keyframe = Keyframe {
        frame: -1,
        ..Default::default()
    };
    shared.renderer.dragging_verts = vec![];
    shared.ui.scale = 1.;
    shared.ui.context_menu.close();
    shared.ui.hovering_tex = -1;
    shared.selections.style = -1;
    shared.selections.style = -1;
    shared.selections.bind = -1;
    shared.ui.styles_modal_size = Vec2::new(500., 500.);
    shared.screenshot_res = Vec2::new(128., 128.);
    shared.ui.sprite_size = Vec2::new(128., 128.);
    shared.ui.sprites_per_row = 4;
    shared.renderer.changed_vert_id = -1;
    shared.ui.dragging_slice = usize::MAX;
    shared.edit_mode.export_exclude_ik = true;
    shared.ui.can_quit = true;
    shared.ui.open_after_export = true;

    #[cfg(feature = "debug")]
    {
        shared.debug = true;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::fs::exists(utils::bin_path() + "dev-docs") {
            Ok(_) => shared.ui.local_doc_url = utils::bin_path(),
            _ => {}
        }

        // import config & colors
        if let Ok(data) = serde_json::from_str(&utils::config_str()) {
            shared.config = data;
        }
        if let Ok(data) = serde_json::from_str(&utils::color_str()) {
            shared.config.colors = data;
        }
        utils::save_config(&shared.config);
    }
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(data) = serde_json::from_str(&utils::config_str()) {
            shared.config = data;
        }
        utils::save_config(&shared.config);
    }

    if !shared.config.skip_startup {
        shared.ui.startup_window = true;
    }

    shared.ui.scale = shared.config.ui_scale;
    shared.renderer.gridline_gap = shared.config.gridline_gap;

    // if this were false, the first click would always
    // be considered non-UI
    shared.camera.on_ui = true;

    #[cfg(not(target_arch = "wasm32"))]
    if recents_path().exists() {
        let mut str = String::new();
        std::fs::File::open(&recents_path())
            .unwrap()
            .read_to_string(&mut str)
            .unwrap();
        shared.ui.recent_file_paths = serde_json::from_str(&str).unwrap();
    }

    let bytes = include_bytes!("../assets/i18n/en.json").as_slice();
    let en: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    shared.ui.init_lang(en);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn install_panic_handler() {
    std::panic::set_hook(Box::new(|panic_info| {
        if let Ok(_) = std::fs::remove_file(&utils::crashlog_file()) {}

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&utils::crashlog_file())
            .unwrap_or_else(|_| {
                // last resort: stderr
                eprintln!("Failed to open panic log at {:?}", utils::crashlog_file());
                std::process::exit(1);
            });

        if let Some(message) = panic_info.payload().downcast_ref::<&str>() {
            let _ = writeln!(file, "Message: {}", message);
        } else if let Some(message) = panic_info.payload().downcast_ref::<String>() {
            let _ = writeln!(file, "Message: {}", message);
        }

        if let Some(location) = panic_info.location() {
            let _ = writeln!(
                file,
                "Location: {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            );
        }

        let bt = backtrace::Backtrace::new();
        let _ = writeln!(file, "\nBacktrace:\n{:?}", bt);

        let _ = file.flush();
    }));
}
