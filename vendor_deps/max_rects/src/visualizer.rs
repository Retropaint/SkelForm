extern crate image;
use image::{ImageBuffer, Rgb};
//use imageproc::drawing::draw_filled_rect_mut;
//use imageproc::rect::Rect;

use crate::bucket::Bucket;
use crate::packing_box::PackingBox;

/// Generates a visualization of placed boxes within bins and saves the result as an image.
///
/// This function constructs an image where each bin and each placed box within the bins
/// are drawn as rectangles. Bins are rendered side by side with a buffer space between them,
/// and each box within a bin is colorized with a unique color.
///
/// # Arguments
/// - `placed_boxes`: A reference to a slice of `PackingBox` objects representing the boxes that have been placed.
/// - `bins`: A reference to a vector of `Bucket` objects representing the bins.
///
/// # Panics
/// This function will panic if it fails to save the generated image to the file system.
///
/// # Examples
///
/// ```rust
/// use max_rects::bucket::Bucket;
/// use max_rects::packing_box::PackingBox;
/// use max_rects::visualizer::generate_visualization;
///
/// // Assume `placed_boxes` and `bins` are already populated
/// let placed_boxes: Vec<PackingBox> = Vec::new();  // Your actual placed_boxes vector
/// let bins: Vec<Bucket> = Vec::new();  // Your actual bins vector
///
/// // generate_visualization(&placed_boxes, &bins);
/// // This will generate an image named 'output.png' visualizing the packed bins and boxes.
/// ```
pub fn generate_visualization(placed_boxes: &[PackingBox], bins: &Vec<Bucket>) {
    const BUFFER: i32 = 10; // Define a buffer of 10 pixels between bins

    // Find the max bin width and height for standardizing the visualization
    let max_bin_width = bins.iter().map(|bin| bin.width).max().unwrap_or(0);
    let max_bin_height = bins.iter().map(|bin| bin.height).max().unwrap_or(0);

    // Calculate the entire width of the image, considering the buffer between bins
    let width = bins.len() as i32 * (max_bin_width + BUFFER) - BUFFER; // subtract BUFFER to remove the last unnecessary buffer.
    let height = max_bin_height;

    let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(width as u32, height as u32);

    for (i, bin) in bins.iter().enumerate() {
        // Draw the bin with a gray color
        //draw_filled_rect_mut(
        //    &mut img,
        //    Rect::at(i as i32 * (max_bin_width + BUFFER), 0)
        //        .of_size(bin.width as u32, bin.height as u32),
        //    Rgb([200, 200, 200]),
        //);

        for box_item in placed_boxes
            .iter()
            .filter(|&b| b.bucketid == Some(bin.bucketid))
        {
            let (box_x1, box_x2, box_y1, box_y2) = box_item.get_coords();
            let x_offset = i as i32 * (max_bin_width + BUFFER);
            let color = Rgb([
                rand::random::<u8>(),
                rand::random::<u8>(),
                rand::random::<u8>(),
            ]); // random color for each box

            // Draw the box with a darker color
            //draw_filled_rect_mut(
            //    &mut img,
            //    Rect::at(box_x1 + x_offset, box_y1)
            //        .of_size((box_x2 - box_x1) as u32, (box_y2 - box_y1) as u32),
            //    color,
            //);
        }
    }

    img.save::<&str>("output.png").unwrap();
}
