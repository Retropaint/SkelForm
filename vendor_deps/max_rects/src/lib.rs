//! # Max Rects
//!
//! `max_rects` is a library for solving 2D bin packing problems.
//! It provides a collection of algorithms and utilities for arranging
//! items within bins in an efficient manner, aiming to use as few bins as possible.
//!
//! This library offers structures to represent bins and boxes, and implements the MaxRects algorithm
//! to find an efficient packing solution. Additionally, it provides a visualization utility to generate
//! visual representations of packing solutions.
//!
//! ## Features
//! - MaxRects bin packing algorithm
//! - Visualization of packing solutions
//!
//! ## Usage
//! Add this to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! max_rects = "1.0.2"
//! ```
//!
//! ## Examples
//! Here's a basic example of how you might use `max_rects`:
//!
//! ```rust
//! extern crate max_rects;
//! use max_rects::{bucket::Bucket, packing_box::PackingBox, calculate_packed_percentage, max_rects::MaxRects};
//!
//! fn main() {
//!     let bins = vec![Bucket::new(200, 200, 0, 0, 1)];
//!     let boxes = vec![PackingBox::new(50, 50)];
//!
//!     let mut problem = MaxRects::new(boxes.clone(), bins.clone());
//!     let (placed, _, _) = problem.place();
//!       
//!     let percentage = calculate_packed_percentage(&boxes, &bins);
//!     println!("Packed percentage: {}%", percentage);
//! }
//! ```
//!
//! ## Visualization
//! The library includes a `visualizer` module that you can use to generate images
//! visualizing the packing solution:
//!
//! ```rust
//! extern crate max_rects;
//! use max_rects::{visualizer, bucket::Bucket, packing_box::PackingBox, max_rects::MaxRects};
//!
//! fn main() {
//!     let bins = vec![Bucket::new(200, 200, 0, 0, 1)];
//!     let boxes = vec![PackingBox::new(50, 50)];
//!     
//!     let mut problem = MaxRects::new(boxes, bins);
//!     let (placed, _, _) = problem.place();
//!     
//!     // visualizer::generate_visualization(&placed, &bins);
//! }
//! ```
//!
//! ## License
//! This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
//!
//! ## Contribution
//! Contributions are welcome! Please feel free to open issues and submit pull requests.
//!

 #![allow(warnings)]

pub mod bucket;
pub mod max_rects;
pub mod packing_box;
pub mod visualizer;

use bucket::Bucket;
use packing_box::PackingBox;

pub fn calculate_packed_percentage(placed_boxes: &[PackingBox], bins: &[Bucket]) -> f32 {
    // Summing up the area of all bins
    let total_bin_area: i32 = bins.iter().map(|bin| bin.width * bin.height).sum();

    // Summing up the area of all placed boxes
    let total_placed_box_area: i32 = placed_boxes
        .iter()
        .map(|box_item| box_item.width * box_item.height)
        .sum();

    // Checking for zero to prevent division by zero
    if total_bin_area == 0 {
        return 0.0;
    }

    // Calculating the percentage
    (total_placed_box_area as f32 / total_bin_area as f32) * 100.0
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packed_percentage_no_bins() {
        let placed_boxes = vec![PackingBox::new(5, 5), PackingBox::new(10, 10)];
        let bins = vec![];
        let percentage = calculate_packed_percentage(&placed_boxes, &bins);
        assert_eq!(percentage, 0.0);
    }

    #[test]
    fn test_packed_percentage_no_boxes() {
        let placed_boxes = vec![];
        let bins = vec![Bucket::new(5, 5, 0, 0, 1), Bucket::new(10, 10, 0, 0, 2)];
        let percentage = calculate_packed_percentage(&placed_boxes, &bins);
        assert_eq!(percentage, 0.0);
    }

    #[test]
    fn test_packed_percentage_filled() {
        let placed_boxes = vec![
            PackingBox::new(5, 5),
            PackingBox::new(10, 10),
            PackingBox::new(5, 5),
            PackingBox::new(5, 5),
            PackingBox::new(5, 5),
            PackingBox::new(5, 5),
        ];
        let bins = vec![Bucket::new(15, 15, 0, 0, 1)];
        let percentage = calculate_packed_percentage(&placed_boxes, &bins);
        assert_eq!(percentage, 100.0);
    }

    #[test]
    fn test_packed_percentage_half_filled() {
        let placed_boxes = vec![PackingBox::new(5, 5)];
        let bins = vec![Bucket::new(10, 10, 0, 0, 1)];
        let percentage = calculate_packed_percentage(&placed_boxes, &bins);
        assert_eq!(percentage, 25.0);
    }
}
