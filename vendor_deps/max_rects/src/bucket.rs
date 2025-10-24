/// Represents a container with a defined width and height, positioned at a specific point in a 2D space.
///
/// # Examples
///
/// Creating a new `Bucket` and accessing its properties:
/// ```
/// use max_rects::bucket::Bucket;
///
/// let bucket = Bucket::new(10, 20, 5, 5, 1);
/// assert_eq!(bucket.width, 10);
/// assert_eq!(bucket.height, 20);
/// assert_eq!(bucket.originx, 5);
/// assert_eq!(bucket.originy, 5);
/// assert_eq!(bucket.bucketid, 1);
/// ```
#[derive(Debug, Clone)]
pub struct Bucket {
    pub width: i32,
    pub height: i32,
    pub originx: i32,
    pub originy: i32,
    pub bucketid: i32,
}

impl Bucket {
    /// Creates a new `Bucket` with the specified dimensions and position.
    ///
    /// # Parameters
    /// - `width`: The width of the bucket.
    /// - `height`: The height of the bucket.
    /// - `originx`: The X-coordinate of the bucket's origin point.
    /// - `originy`: The Y-coordinate of the bucket's origin point.
    /// - `bucketid`: An identifier for the bucket.
    pub fn new(width: i32, height: i32, originx: i32, originy: i32, bucketid: i32) -> Self {
        Self {
            width,
            height,
            originx,
            originy,
            bucketid,
        }
    }
    /// Returns the coordinates of the corners of the bucket.
    ///
    /// The coordinates are returned as a tuple of four `i32` values: `(left, right, top, bottom)`.
    pub fn get_coords(&self) -> (i32, i32, i32, i32) {
        (
            self.originx,
            self.originx + self.width,
            self.originy,
            self.originy + self.height,
        )
    }
    /// Returns the coordinates of the corners of the bucket as an array of tuples.
    ///
    /// Each tuple represents the (x, y) coordinates of a corner of the bucket.
    pub fn get_corners(&self) -> [(i32, i32); 4] {
        let (x1, x2, y1, y2) = self.get_coords();
        [(x1, y1), (x2, y1), (x1, y2), (x2, y2)]
    }
    /// Determines whether the current bucket overlaps with another bucket.
    ///
    /// Buckets with different `bucketid` values are considered not to overlap.
    pub fn overlap(&self, other: &Bucket) -> bool {
        if self.bucketid != other.bucketid {
            return false;
        }
        let self_corners = self.get_corners();
        let other_corners = other.get_corners();
        !(self_corners[1].0 <= other_corners[2].0
            || self_corners[2].0 >= other_corners[1].0
            || self_corners[1].1 >= other_corners[2].1
            || self_corners[2].1 <= other_corners[1].1)
    }
    /// Determines whether the current bucket completely contains another bucket.
    ///
    /// Buckets with different `bucketid` values are considered not to contain each other.
    pub fn contains(&self, other: &Bucket) -> bool {
        if self.bucketid != other.bucketid {
            return false;
        }
        let self_corners = self.get_corners();
        let other_corners = other.get_corners();
        self_corners[0].0 <= other_corners[0].0
            && other_corners[3].0 <= self_corners[3].0
            && self_corners[0].1 <= other_corners[0].1
            && other_corners[3].1 <= self_corners[3].1
    }
    /// Calculates and returns the area of the bucket.
    pub fn area(&self) -> i32 {
        self.width * self.height
    }
}

#[cfg(test)]
mod tests {
    use super::Bucket;

    #[test]
    fn test_bucket_creation() {
        let bucket = Bucket::new(10, 20, 5, 5, 1);
        assert_eq!(bucket.width, 10);
        assert_eq!(bucket.height, 20);
        assert_eq!(bucket.originx, 5);
        assert_eq!(bucket.originy, 5);
        assert_eq!(bucket.bucketid, 1);
    }

    #[test]
    fn test_get_coords() {
        let bucket = Bucket::new(10, 20, 5, 5, 1);
        assert_eq!(bucket.get_coords(), (5, 15, 5, 25));
    }

    #[test]
    fn test_get_corners() {
        let bucket = Bucket::new(10, 20, 5, 5, 1);
        assert_eq!(bucket.get_corners(), [(5, 5), (15, 5), (5, 25), (15, 25)]);
    }

    #[test]
    fn test_overlap() {
        let bucket1 = Bucket::new(10, 10, 0, 0, 1);
        let bucket2 = Bucket::new(10, 10, 5, 5, 1);
        assert!(bucket1.overlap(&bucket2));
    }

    #[test]
    fn test_overlap_bucket() {
        let bucket1 = Bucket::new(10, 10, 0, 0, 1);
        let bucket2 = Bucket::new(10, 10, 5, 5, 0);
        assert!(!bucket1.overlap(&bucket2));
    }

    #[test]
    fn test_no_overlap() {
        let bucket1 = Bucket::new(10, 10, 0, 0, 1);
        let bucket2 = Bucket::new(10, 10, 10, 10, 1);
        assert!(!bucket1.overlap(&bucket2));
    }

    #[test]
    fn test_contains() {
        let bucket1 = Bucket::new(10, 10, 0, 0, 1);
        let bucket2 = Bucket::new(5, 5, 2, 2, 1);
        assert!(bucket1.contains(&bucket2));
    }

    #[test]
    fn test_not_contains() {
        let bucket1 = Bucket::new(10, 10, 0, 0, 1);
        let bucket2 = Bucket::new(5, 5, 6, 6, 1);
        assert!(!bucket1.contains(&bucket2));
    }

    #[test]
    fn test_area() {
        let bucket = Bucket::new(10, 20, 5, 5, 1);
        assert_eq!(bucket.area(), 200);
    }
}
