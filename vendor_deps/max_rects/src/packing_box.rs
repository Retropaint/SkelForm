use crate::bucket::Bucket;
/// Represents a rectangular box with a defined width and height, which can be placed within a bucket in a 2D space.
///
/// # Examples
///
/// Creating a new `PackingBox`, placing it, and accessing its properties:
/// ```
/// use max_rects::packing_box::PackingBox;
///
/// let mut box_item = PackingBox::new(5, 6);
/// assert_eq!(box_item.width, 5);
/// assert_eq!(box_item.height, 6);
///
/// box_item.place(10, 20, 1);
/// assert_eq!(box_item.originx, Some(10));
/// assert_eq!(box_item.originy, Some(20));
/// assert_eq!(box_item.bucketid, Some(1));
/// ```
#[derive(Debug, Clone)]
pub struct PackingBox {
    pub width: i32,
    pub height: i32,
    pub originx: Option<i32>,
    pub originy: Option<i32>,
    pub bucketid: Option<i32>,
}

impl PackingBox {
    /// Creates a new `PackingBox` with the specified dimensions.
    /// Boxes are not placed until placed by the algorithm.
    /// # Parameters
    /// - `width`: The width of the box.
    /// - `height`: The height of the box.
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            originx: None,
            originy: None,
            bucketid: None,
        }
    }
    /// Places the box at the specified coordinates within a bucket.
    ///
    /// # Parameters
    /// - `originx`: The X-coordinate of the box's origin point.
    /// - `originy`: The Y-coordinate of the box's origin point.
    /// - `bucketid`: The identifier of the bucket in which the box is placed.
    pub fn place(&mut self, originx: i32, originy: i32, bucketid: i32) {
        self.originx = Some(originx);
        self.originy = Some(originy);
        self.bucketid = Some(bucketid);
    }
    /// Returns the coordinates of the corners of the box.
    ///
    /// The coordinates are returned as a tuple of four `i32` values: `(left, right, top, bottom)`.
    pub fn get_coords(&self) -> (i32, i32, i32, i32) {
        (
            self.originx.unwrap(),
            self.originx.unwrap() + self.width,
            self.originy.unwrap(),
            self.originy.unwrap() + self.height,
        )
    }
    /// Determines whether the current box overlaps with a specified bucket.
    ///
    /// Boxes and buckets with different `bucketid` values are considered not to overlap.
    pub fn overlap(&self, other: &Bucket) -> bool {
        if self.bucketid != Some(other.bucketid) {
            return false;
        }
        let self_corners = self.get_corners();
        let other_corners = other.get_corners();
        !(self_corners[1].0 <= other_corners[2].0
            || self_corners[2].0 >= other_corners[1].0
            || self_corners[1].1 >= other_corners[2].1
            || self_corners[2].1 <= other_corners[1].1)
    }
    /// Returns the coordinates of the corners of the box as an array of tuples.
    ///
    /// Each tuple represents the (x, y) coordinates of a corner of the box.
    pub fn get_corners(&self) -> [(i32, i32); 4] {
        let (x1, x2, y1, y2) = self.get_coords();
        [(x1, y1), (x2, y1), (x1, y2), (x2, y2)]
    }
}

#[cfg(test)]
mod tests {
    use super::Bucket;
    use super::PackingBox;

    #[test]
    fn test_new() {
        let box_item = PackingBox::new(5, 6);
        assert_eq!(box_item.width, 5);
        assert_eq!(box_item.height, 6);
        assert_eq!(box_item.originx, None);
        assert_eq!(box_item.originy, None);
        assert_eq!(box_item.bucketid, None);
    }

    #[test]
    fn test_place() {
        let mut box_item = PackingBox::new(5, 6);
        box_item.place(10, 20, 1);
        assert_eq!(box_item.originx, Some(10));
        assert_eq!(box_item.originy, Some(20));
        assert_eq!(box_item.bucketid, Some(1));
    }

    #[test]
    fn test_get_coords() {
        let mut box_item = PackingBox::new(5, 6);
        box_item.place(10, 20, 1);
        let coords = box_item.get_coords();
        assert_eq!(coords, (10, 15, 20, 26));
    }

    #[test]
    fn test_get_corners() {
        let mut box_item = PackingBox::new(5, 6);
        box_item.place(10, 20, 1);
        let corners = box_item.get_corners();
        assert_eq!(corners, [(10, 20), (15, 20), (10, 26), (15, 26)]);
    }

    #[test]
    fn test_overlap() {
        let mut box1 = PackingBox::new(5, 6);
        box1.place(10, 20, 1);
        let mut bucket2 = Bucket::new(5, 6, 12, 22, 0);

        assert_eq!(box1.overlap(&bucket2), false);

        bucket2 = Bucket::new(5, 6, 9, 19, 1);
        assert_eq!(box1.overlap(&bucket2), true);
    }
}
