// #![windows_subsystem = "windows"] // uncomment this to suppress terminal on windows

#![windows_subsystem = "windows"]

use std::{
    fs,
    io::{Read, Write},
};

use skelform_lib::shared::*;

#[cfg(not(target_arch = "wasm32"))]
use skelform_lib::file_reader;

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
    shared.input.mouse_left = -1;
    shared.camera.zoom = 500.;
    shared.ui.anim.selected = usize::MAX;
    shared.ui.anim.timeline_zoom = 1.;
    shared.ui.anim.exported_frame = "".to_string();
    shared.ui.anim.selected_frame = -1;
    shared.ui.anim.dragged_keyframe = Keyframe {
        frame: -1,
        ..Default::default()
    };
    shared.dragging_vert = usize::MAX;
    shared.ui.scale = 1.;

    shared.ui.context_menu.close();

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
        import: base_path.clone() + "import_path",
        import_psd: base_path.clone() + "import_tiff_path",
        export_vid_text: base_path.clone() + "export_vid_text",
        export_vid_done: base_path.clone() + "export_vid_done",
    };

    let mut first_time = true;
    #[cfg(not(target_arch = "wasm32"))]
    {
        // import config
        let config_path = directories_next::ProjectDirs::from("com", "retropaint", "skelform")
            .map(|proj_dirs| proj_dirs.data_dir().join("config.json"))
            .unwrap();
        if config_path.exists() {
            let mut str = String::new();
            std::fs::File::open(&config_path)
                .unwrap()
                .read_to_string(&mut str)
                .unwrap();
            shared.config = serde_json::from_str(&str).unwrap();
            first_time = false;
        } else {
            // save config
            fs::create_dir_all(config_path.parent().unwrap()).unwrap();
            let mut file = std::fs::File::create(&config_path).unwrap();
            file.write_all(serde_json::to_string(&shared.config).unwrap().as_bytes())
                .unwrap();
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        // import config (web)
        if let Ok(data) = serde_json::from_str(&skelform_lib::getConfig()) {
            shared.config = data;
            first_time = false;
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        skelform_lib::saveConfig(serde_json::to_string(&shared.config).unwrap());
    }

    if first_time {
        shared.ui.start_tutorial(&shared.armature);
    }
    shared.ui.scale = shared.config.ui_scale;

    // if this were false, the first click would always
    // be considered non-UI
    shared.input.on_ui = true;
}
