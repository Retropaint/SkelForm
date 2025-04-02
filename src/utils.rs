//! Isolated set of helper functions.

use crate::shared::{Vec2, Vertex};

use std::io::Write;
/// Convert a point from screen to world space.
pub fn screen_to_world_space(pos: Vec2, window: Vec2) -> Vec2 {
    Vec2 {
        x: -1. + ((pos.x / window.x as f32) * 2.),
        y: -(-1. + ((pos.y / window.y as f32) * 2.)),
    }
}

/// Rotate a point via rotation matrix.
pub fn rotate(point: &Vec2, rot: f32) -> Vec2 {
    Vec2 {
        x: point.x * rot.cos() - point.y * rot.sin(),
        y: point.x * rot.sin() + point.y * rot.cos(),
    }
}

/// Return the angle that the source would need to look at target.
pub fn look_at(source: &Vec2, target: &Vec2) -> f32 {
    f32::atan2(-(target.x - source.x), target.y - source.y)
}

/// Check if a point is in a rectangle (formed by vertices).
pub fn in_bounding_box(
    point: &Vec2,
    verts: &Vec<Vertex>,
    window_size: &Vec2,
) -> (Vec<Vertex>, bool) {
    // get the bound based on infinitely-long lines
    let mut top = -f32::INFINITY;
    let mut bot = f32::INFINITY;
    let mut left = f32::INFINITY;
    let mut right = -f32::INFINITY;
    for v in verts {
        left = f32::min(left, v.pos.x);
        right = f32::max(right, v.pos.x);
        bot = f32::min(bot, v.pos.y);
        top = f32::max(top, v.pos.y);
    }

    let vertices: Vec<Vertex> = vec![
        Vertex {
            pos: Vec2::new(right, top),
            uv: Vec2::new(1., 0.),
        },
        Vertex {
            pos: Vec2::new(left, top),
            uv: Vec2::new(0., 1.),
        },
        Vertex {
            pos: Vec2::new(left, bot),
            uv: Vec2::new(0., 0.),
        },
        Vertex {
            pos: Vec2::new(right, bot),
            uv: Vec2::new(1., 1.),
        },
    ];

    // convert bound positions to screen space
    let half = Vec2 {
        x: window_size.x / 2.,
        y: window_size.y / 2.,
    };
    top = half.y - (half.y * top);
    bot = half.y - (half.y * bot);
    left = half.x + (half.x * left);
    right = half.x + (half.x * right);

    // finally, check if point is inside
    (
        vertices,
        point.y > top && point.y < bot && point.x > left && point.x < right,
    )
}

pub fn to_vec2(f: f32) -> Vec2 {
    Vec2::new(f, f)
}

pub fn export_textures(textures: &Vec<crate::Texture>, armature: &crate::Armature) {
    // get the image size in advance
    let mut size = Vec2::default();
    for tex in textures {
        size.x += tex.size.x;
        if tex.size.y > size.y {
            size.y = tex.size.y;
        }
    }

    // this is the buffer that will be saved as an image
    let mut final_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::new(size.x as u32, size.y as u32);

    let mut offset: u32 = 0;
    for tex in textures {
        // get current texture as a buffer
        let img_buf = <image::ImageBuffer<image::Rgba<u8>, _>>::from_raw(
            tex.size.x as u32,
            tex.size.y as u32,
            tex.pixels.clone(),
        )
        .unwrap();

        // add it to the final buffer
        for x in 0..img_buf.width() {
            for y in 0..img_buf.height() {
                final_buf.put_pixel(x + offset, y, *img_buf.get_pixel(x, y));
            }
        }

        // make sure the next texture will be added beside this one, instead of overwriting
        offset += img_buf.width();
    }

    // finally, the final buffer as an image
    image::save_buffer(
        "temp.png",
        &final_buf.to_vec(),
        final_buf.width() as u32,
        final_buf.height() as u32,
        image::ExtendedColorType::Rgba8,
    )
    .unwrap();

    let img_data = std::fs::read("./temp.png").unwrap();

    // clone armature and make some edits, then serialize it
    let mut armature_copy = armature.clone();
    armature_copy.textures = vec![];
    let armature_json = serde_json::to_string(&armature_copy).unwrap();

    // create zip file
    let mut zip = zip::ZipWriter::new(std::fs::File::create("armature.zip").unwrap());
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // save armature json and texture image
    zip.start_file("armature.json", options).unwrap();
    zip.write(armature_json.as_bytes()).unwrap();
    zip.start_file("textures.png", options).unwrap();
    zip.write(&img_data.to_vec()).unwrap();

    // Apply the changes you've made.
    // Dropping the `ZipWriter` will have the same effect, but may silently fail
    zip.finish().unwrap();

    std::fs::remove_file("temp.png").unwrap();
}
