extern crate rand;
use rand::Rng;

use clap::{App, Arg};

use max_rects::bucket::Bucket;
use max_rects::calculate_packed_percentage;
use max_rects::max_rects::MaxRects;
use max_rects::packing_box::PackingBox;
use max_rects::visualizer::generate_visualization;

/// Validates if a string can be parsed to an i32.
///
/// # Arguments
/// - `val`: The string value to be parsed.
///
/// # Returns
/// - A `Result` indicating success or failure.
fn is_i32(val: &str) -> Result<(), String> {
    val.parse::<i32>()
        .map(|_| ())
        .map_err(|err| format!("could not parse '{}' as i32: {}", val, err))
}

/// Parses command line arguments.
///
/// # Returns
/// - A tuple containing two i32 values representing the number of boxes and bins respectively.
fn parse_arguments() -> (i32, i32) {
    let matches = App::new("Box Packing")
        .version("1.0.1")
        .about("Generates a visualization based on the number of boxes and bins")
        .arg(
            Arg::new("boxes")
                .short('b')
                .long("boxes")
                .value_name("NUMBER")
                .takes_value(true)
                .required(true)
                .validator(is_i32)
                .help("Sets the number of boxes to place"),
        )
        .arg(
            Arg::new("bins")
                .short('n')
                .long("bins")
                .value_name("NUMBER")
                .takes_value(true)
                .required(true)
                .validator(is_i32)
                .help("Sets the number of bins to pack"),
        )
        .get_matches();

    let num_boxes = matches.value_of("boxes").unwrap().parse::<usize>().unwrap() as i32;
    let num_bins = matches.value_of("bins").unwrap().parse::<usize>().unwrap() as i32;

    (num_boxes, num_bins)
}

fn main() {
    let (num_boxes, num_bins) = parse_arguments();

    let mut rng = rand::thread_rng();
    let mut bins: Vec<Bucket> = Vec::new();
    for idx in 0..num_bins {
        let new_bin = Bucket::new(200, 200, 0, 0, idx);
        bins.push(new_bin);
    }

    let mut boxes: Vec<PackingBox> = Vec::new();

    for _ in 0..num_boxes {
        let width = rng.gen_range(1..100);
        let height = rng.gen_range(1..100);

        let new_box = PackingBox::new(width, height);
        boxes.push(new_box);
    }
    let bins_clone = bins.clone();
    let mut problem = MaxRects::new(boxes, bins);
    let (placed, missed, remaining_bins) = problem.place();
    generate_visualization(&placed, &bins_clone);

    println!("Placed: {:?}", placed);
    println!("Missed: {:?}", missed);
    println!("Remaining Bins: {:?}", remaining_bins);
    let percentage_packed = calculate_packed_percentage(&placed, &bins_clone);
    println!("Percentage Packed: {:.2}%", percentage_packed);
}
