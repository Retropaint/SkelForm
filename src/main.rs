#![windows_subsystem = "windows"]

#[cfg(not(target_arch = "wasm32"))]
use skelform_lib::shared::config_path;

use skelform_lib::shared::*;

#[cfg(not(target_arch = "wasm32"))]
use skelform_lib::file_reader;

#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;

fn main() -> Result<(), winit::error::EventLoopError> {
    // uncomment below to get console panic hook as early as possible for debugging
    //
    // otherwise, it's activated in lib.rs

    // #[cfg(target_arch = "wasm32")]
    // {
    //     std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    //     console_log::init().expect("Failed to initialize logger!");
    //     log::info!("test");
    // }

    let mut app = skelform_lib::App::default();

    init_shared(&mut app.shared);

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
    app.shared.startup = startup;

    // delete any leftover temporary files
    #[cfg(not(target_arch = "wasm32"))]
    file_reader::del_temp_files(&app.shared.temp_path.base);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<String> = std::env::args().collect();

        // load .skf based on first arg
        if args.len() > 1 {
            file_reader::create_temp_file(&app.shared.temp_path.import, &args[1].to_string());
        }
    }

    let event_loop = winit::event_loop::EventLoop::builder().build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);

    event_loop.run_app(&mut app)
}

fn init_shared(shared: &mut Shared) {
    shared.ui.selected_bone_idx = usize::MAX;
    shared.camera.zoom = 500.;
    shared.ui.anim.selected = usize::MAX;
    shared.ui.anim.timeline_zoom = 1.;
    shared.ui.anim.exported_frame = "".to_string();
    shared.ui.anim.selected_frame = -1;
    shared.ui.anim.dragged_keyframe = Keyframe {
        frame: -1,
        ..Default::default()
    };
    shared.dragging_verts = vec![];
    shared.ui.scale = 1.;
    shared.ui.context_menu.close();
    shared.ui.hovering_tex = -1;
    shared.ui.selected_style = -1;
    shared.ui.selected_tex_set_id = -1;
    shared.screenshot_res = Vec2::new(128., 128.);

    #[cfg(feature = "debug")]
    {
        shared.debug = true;
    }

    let base_path: String;
    #[cfg(not(target_arch = "wasm32"))]
    {
        base_path = directories_next::BaseDirs::new()
            .unwrap()
            .cache_dir()
            .to_str()
            .unwrap()
            .to_owned()
            + "/.skelform_";
    }
    #[cfg(target_arch = "wasm32")]
    {
        base_path = "".to_string();
    }
    shared.temp_path = TempPath {
        base: base_path.clone(),
        img: base_path.clone() + "img_path",
        save: base_path.clone() + "save_path",
        save_finish: base_path.clone() + "save_finish",
        import: base_path.clone() + "import_path",
        export_vid_text: base_path.clone() + "export_vid_text",
        export_vid_done: base_path.clone() + "export_vid_done",
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        // import config
        if config_path().exists() {
            skelform_lib::utils::import_config(shared);
        } else {
            skelform_lib::utils::save_config(&shared.config);
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        skelform_lib::utils::import_config(shared);
        skelform_lib::utils::save_config(&shared.config);
    }

    if !shared.config.skip_startup {
        shared.ui.set_state(UiState::StartupWindow, true);
    }

    shared.ui.scale = shared.config.ui_scale;
    shared.gridline_gap = shared.config.gridline_gap;

    // if this were false, the first click would always
    // be considered non-UI
    shared.input.on_ui = true;

    #[cfg(not(target_arch = "wasm32"))]
    if recents_path().exists() {
        let mut str = String::new();
        std::fs::File::open(&recents_path())
            .unwrap()
            .read_to_string(&mut str)
            .unwrap();
        shared.recent_file_paths = serde_json::from_str(&str).unwrap();
    }

    let bytes = include_bytes!("../assets/i18n/en.json").as_slice();
    let en: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    shared.init_lang(en);
}
