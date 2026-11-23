use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use zip::ZipArchive;

fn main() {
    std::fs::remove_file("./update.log").unwrap();
    // set up panic hooks, for crash logs and the like
    #[cfg(not(target_arch = "wasm32"))]
    {
        //std::env::set_var("RUST_BACKTRACE", "1");
    }
    #[cfg(all(not(debug_assertions), not(target_arch = "wasm32")))]
    install_panic_handler();
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init().expect("Failed to initialize logger!");
    }

    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        write("To update manually, run again with:\nwindows\nmacos\nlinux");
        return;
    }
    let os = &args[1];
    if os != "windows" && os != "macos" && os != "linux" {
        write("Not a valid OS string.");
        return;
    }

    let url = "https://skelform.org/versions.json";
    let request = ureq::get(url).header("Example-Header", "header value");

    let destination = if os == "macos" {
        "../../../skf.zip"
    } else {
        "../skf.zip"
    };

    let output = if os == "macos" { "../../../" } else { "../" };

    let version = if os == "macos" {
        "https://github.com/Retropaint/SkelForm/releases/latest/download/SkelForm.app.zip"
    } else if os == "windows" {
        "https://github.com/Retropaint/SkelForm/releases/latest/download/skelform_windows.zip"
    } else {
        "https://github.com/Retropaint/SkelForm/releases/latest/download/skelform_linux.zip"
    };

    write("started");

    let manifest: serde_json::Value = match request.call() {
        Ok(mut data) => serde_json::from_str(&data.body_mut().read_to_string().unwrap()).unwrap(),
        Err(_) => serde_json::Value::default(),
    };

    // get downloaded zip file
    let resp = ureq::get(version).call().unwrap();
    let mut zip = std::fs::File::create(destination).unwrap();
    let bytes_result: Result<Vec<u8>, _> = resp.into_body().into_reader().bytes().collect();
    if let Ok(bytes) = bytes_result {
        _ = zip.write(&bytes);
    }

    write("downloaded");

    // extract new program from zip
    let file = File::open(destination).unwrap();
    let mut archive = ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let filename = entry.name().unwrap().to_lowercase();
        if entry.enclosed_name() == None || filename == "updater" {
            continue;
        }
        let relative_path = entry.enclosed_name().unwrap();

        let output_path = std::path::Path::new(output).join(relative_path);

        if entry.is_dir() {
            continue;
        }

        // create this file's directories first
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        // create file
        let mut output_file = File::create(&output_path).unwrap();
        io::copy(&mut entry, &mut output_file).unwrap();

        // set unix permissions
        #[cfg(unix)]
        {
            if let Some(mode) = entry.unix_mode() {
                fs::set_permissions(&output_path, fs::Permissions::from_mode(mode)).unwrap();
            }
        }
    }

    write("extracted");

    std::process::Command::new("./SkelForm").spawn().unwrap();

    //move_contents(Path::new("../../../"));
}

fn move_contents(dir: &Path) {
    let parent = dir.parent().unwrap();

    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let destination = parent.join(entry.file_name());
        fs::rename(entry.path(), destination).unwrap();
    }

    fs::remove_dir(dir).unwrap();
}

pub fn write(content: &str) {
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("update.log")
    {
        let _ = writeln!(file, "{content}");
    }
}

pub fn install_panic_handler() {
    pub fn install_panic_handler() {
        std::panic::set_hook(Box::new(|panic_info| {
            // print error message and file loc to console
            eprintln!("{}", panic_info);

            // write error and backtrace to log file
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("update.log")
            {
                let _ = writeln!(file, "Panic: {panic_info}");
                let bt = std::backtrace::Backtrace::force_capture();
                let _ = writeln!(file, "{bt}");
            }
        }));
    }
}
