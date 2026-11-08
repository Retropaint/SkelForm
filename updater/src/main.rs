use std::fs;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use zip::ZipArchive;

fn main() {
    let url = "https://skelform.org/versions.json";
    let request = ureq::get(url).header("Example-Header", "header value");

    let manifest: serde_json::Value = match request.call() {
        Ok(mut data) => serde_json::from_str(&data.body_mut().read_to_string().unwrap()).unwrap(),
        Err(_) => serde_json::Value::default(),
    };

    // get downloaded zip file
    let resp = ureq::get(
        "https://github.com/Retropaint/SkelForm/releases/latest/download/SkelForm.app.zip",
    )
    .call()
    .unwrap();
    let mut ffmpeg_zip = std::fs::File::create("../../../skf.zip").unwrap();
    let bytes_result: Result<Vec<u8>, _> = resp.into_body().into_reader().bytes().collect();
    if let Ok(bytes) = bytes_result {
        _ = ffmpeg_zip.write(&bytes);
    }

    // extract ffmpeg from zip
    let file = File::open("../../../skf.zip").unwrap();
    let mut archive = ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).unwrap();
        let Some(relative_path) = entry.enclosed_name() else {
            continue;
        };

        let output_path = std::path::Path::new("../../../").join(relative_path);

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
