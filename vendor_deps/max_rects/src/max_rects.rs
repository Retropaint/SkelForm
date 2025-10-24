/// The `MaxRects` module provides an implementation of the MaxRects bin packing algorithm.
/// This algorithm is designed to efficiently pack a set of rectangles into a larger containing rectangle,
/// with the goal of minimizing wasted space.

/// External crate `rayon` is used to allow parallel processing which optimizes the performance of the algorithm.
extern crate rayon;
use rayon::prelude::*;

use crate::bucket::Bucket;
use crate::packing_box::PackingBox;

/// A `MaxRects` object contains two fields:
/// - `boxes`: A vector of `PackingBox` objects representing the rectangles to be placed.
/// - `bins`: A vector of `Bucket` objects representing the available bins.
pub struct MaxRects {
    pub boxes: Vec<PackingBox>,
    pub bins: Vec<Bucket>,
}

impl MaxRects {
    /// Constructs a new `MaxRects` instance.
    ///
    /// # Arguments
    /// - `boxes`: A vector of `PackingBox` objects representing the rectangles to be placed.
    /// - `bins`: A vector of `Bucket` objects representing the available bins.
    ///
    /// # Returns
    /// A new `MaxRects` object.
    pub fn new(boxes: Vec<PackingBox>, bins: Vec<Bucket>) -> Self {
        Self { boxes, bins }
    }
    /// Attempts to place the boxes into the bins.
    ///
    /// This method iteratively places boxes into bins where they fit, dividing the remaining bin space
    /// into potentially smaller bins as boxes are placed. The process continues until no more boxes
    /// can be placed.
    ///
    /// # Returns
    /// A tuple of three vectors:
    /// - A vector of `PackingBox` objects representing the placed boxes.
    /// - A vector of `PackingBox` objects representing the remaining unplaced boxes.
    /// - A vector of `Bucket` objects representing the updated bins after all possible placements have been made.
    pub fn place(&mut self) -> (Vec<PackingBox>, Vec<PackingBox>, Vec<Bucket>) {
        let mut nochange = false;
        let mut placed = vec![];

        while !nochange {
            nochange = true;

            let search_result: Option<(i32, usize, usize)> = self
                .boxes
                .par_iter()
                .enumerate()
                .filter_map(|(i, box_item)| {
                    self.bins
                        .iter()
                        .enumerate()
                        .filter_map(|(j, rect)| {
                            if box_item.width <= rect.width && box_item.height <= rect.height {
                                let diff = i32::min(
                                    rect.width - box_item.width,
                                    rect.height - box_item.height,
                                );
                                Some((diff, i, j))
                            } else {
                                None
                            }
                        })
                        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
                })
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            if let Some((_, box_idx, idx)) = search_result {
                nochange = false;
                let mut box_item = self.boxes.remove(box_idx);
                let bin_item = self.bins.remove(idx);

                let adjacent = Bucket {
                    width: bin_item.width - box_item.width,
                    height: bin_item.height,
                    originx: bin_item.originx + box_item.width,
                    originy: bin_item.originy,
                    bucketid: bin_item.bucketid,
                };

                let above = Bucket {
                    width: bin_item.width,
                    height: bin_item.height - box_item.height,
                    originx: bin_item.originx,
                    originy: bin_item.originy,
                    bucketid: bin_item.bucketid,
                };

                box_item.place(
                    bin_item.originx,
                    bin_item.height - box_item.height + bin_item.originy,
                    bin_item.bucketid,
                );
                placed.push(box_item.clone());

                if adjacent.area() > 0 {
                    self.bins.push(adjacent);
                }

                if above.area() > 0 {
                    self.bins.push(above);
                }

                let mut overlaps = vec![false; self.bins.len()];
                let mut new_buckets = Vec::new();

                for (idx, rect) in self.bins.iter().enumerate() {
                    if placed.last().unwrap().overlap(rect) {
                        overlaps[idx] = true;
                        // split!
                        let left = Bucket::new(
                            placed.last().unwrap().originx.unwrap() - rect.originx,
                            rect.height,
                            rect.originx,
                            rect.originy,
                            rect.bucketid,
                        );
                        let above = Bucket::new(
                            rect.width,
                            placed.last().unwrap().originy.unwrap() - rect.originy,
                            rect.originx,
                            rect.originy,
                            rect.bucketid,
                        );
                        let right = Bucket::new(
                            rect.get_coords().1 - placed.last().unwrap().get_coords().1,
                            rect.height,
                            placed.last().unwrap().get_coords().1,
                            rect.originy,
                            rect.bucketid,
                        );
                        let below = Bucket::new(
                            rect.width,
                            rect.get_coords().3 - placed.last().unwrap().get_coords().3,
                            rect.originx,
                            placed.last().unwrap().get_coords().3,
                            rect.bucketid,
                        );

                        for new_bucket in [left, above, right, below] {
                            if new_bucket.area() > 0 {
                                new_buckets.push(new_bucket);
                                overlaps.push(false);
                            }
                        }
                    }
                }

                self.bins.extend(new_buckets);
                let filtered_bins: Vec<Bucket> = self
                    .bins
                    .iter()
                    .zip(&overlaps)
                    .filter_map(|(item, &keep)| if !keep { Some(item) } else { None })
                    .cloned()
                    .collect();

                self.bins = filtered_bins;
                let mut overlaps = vec![false; self.bins.len()];

                for (i, bin1) in self.bins.iter().enumerate() {
                    for (j, bin2) in self.bins.iter().enumerate() {
                        if i != j && bin1.contains(bin2) {
                            overlaps[j] = true;
                        }
                    }
                }

                let new_bins: Vec<Bucket> = self
                    .bins
                    .clone()
                    .into_iter()
                    .enumerate()
                    .filter(|(idx, _)| !overlaps[*idx])
                    .map(|(_, bin)| bin)
                    .collect();

                self.bins = new_bins;
            }
        }

        (placed, self.boxes.clone(), self.bins.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bucket::Bucket;
    use crate::packing_box::PackingBox;

    #[test]
    fn test_new() {
        let boxes = vec![PackingBox::new(5, 6)];
        let bins = vec![Bucket::new(10, 20, 0, 0, 1)];
        let max_rects = MaxRects::new(boxes, bins);

        assert_eq!(max_rects.boxes.len(), 1);
        assert_eq!(max_rects.bins.len(), 1);
    }

    #[test]
    fn test_place_single_box() {
        let boxes = vec![PackingBox::new(5, 6)];
        let bins = vec![Bucket::new(10, 20, 0, 0, 1)];
        let mut max_rects = MaxRects::new(boxes, bins);
        let (placed, remaining, updated_bins) = max_rects.place();

        assert_eq!(placed.len(), 1);
        assert!(remaining.is_empty());
        assert_eq!(updated_bins.len(), 2); // Two new bins should be created from the remaining space.
        assert_eq!(placed[0].get_coords(), (0, 5, 14, 20)); //should be placed bottom right
        assert_eq!(updated_bins[0].get_coords(), (5, 10, 0, 20)); // new buckets
        assert_eq!(updated_bins[1].get_coords(), (0, 10, 0, 14)); // new buckets
    }

    #[test]
    fn test_place_multiple_boxes() {
        let boxes = vec![PackingBox::new(5, 6), PackingBox::new(4, 4)];
        let bins = vec![Bucket::new(10, 20, 0, 0, 1)];
        let mut max_rects = MaxRects::new(boxes, bins);
        let (placed, remaining, updated_bins) = max_rects.place();

        assert_eq!(placed.len(), 2);
        assert!(remaining.is_empty());
        assert_eq!(updated_bins.len(), 3); // More bins should be created from the remaining space.
        assert_eq!(placed[0].get_coords(), (0, 5, 14, 20)); //should be placed bottom right
        assert_eq!(placed[1].get_coords(), (5, 9, 16, 20)); //should be placed bottom right

        assert_eq!(updated_bins[0].get_coords(), (0, 10, 0, 14)); // new buckets
        assert_eq!(updated_bins[1].get_coords(), (9, 10, 0, 20)); // new buckets
        assert_eq!(updated_bins[2].get_coords(), (5, 10, 0, 16)); // new buckets
    }

    #[test]
    fn test_place_no_fit() {
        let boxes = vec![PackingBox::new(15, 16)]; // Box is too big to fit in the bin.
        let bins = vec![Bucket::new(10, 20, 0, 0, 1)];
        let mut max_rects = MaxRects::new(boxes, bins);
        let (placed, remaining, updated_bins) = max_rects.place();

        assert!(placed.is_empty());
        assert_eq!(remaining.len(), 1);
        assert_eq!(updated_bins.len(), 1); // Bin remains unchanged.
    }
}
