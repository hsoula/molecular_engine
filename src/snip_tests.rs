#[derive(Debug, PartialEq)]
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    // Scalar product (dot product)
    fn scalar_product(&self, other: &Point) -> f64 {
        self.x * other.x + self.y * other.y
    }

    // Vector norm (magnitude)
    fn norm(&self) -> f64 {
        (self.x.powi(2) + self.y.powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_approx_eq::assert_approx_eq; // For floating point comparisons

    #[test]
    fn test_scalar_product() {
        let p1 = Point { x: 3.0, y: 4.0 };
        let p2 = Point { x: 2.0, y: 5.0 };
        assert_approx_eq!(p1.scalar_product(&p2), 26.0);

        let p3 = Point { x: -1.0, y: 2.0 };
        let p4 = Point { x: 3.0, y: -4.0 };
        assert_approx_eq!(p3.scalar_product(&p4), -11.0);

        let p5 = Point { x: 0.0, y: 0.0 };
        let p6 = Point { x: 5.0, y: 7.0 };
        assert_approx_eq!(p5.scalar_product(&p6), 0.0);
    }

    #[test]
    fn test_norm() {
        let p1 = Point { x: 3.0, y: 4.0 };
        assert_approx_eq!(p1.norm(), 5.0);

        let p2 = Point { x: -3.0, y: 4.0 };
        assert_approx_eq!(p2.norm(), 5.0);

        let p3 = Point { x: 0.0, y: 0.0 };
        assert_approx_eq!(p3.norm(), 0.0);

        let p4 = Point { x: 1.0, y: 1.0 };
        assert_approx_eq!(p4.norm(), 2.0f64.sqrt());
    }
}