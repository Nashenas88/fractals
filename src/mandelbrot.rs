use crate::fractal::{Generator, Point};

pub struct Mandelbrot {
    z: Point,
}

impl Mandelbrot {
    pub fn new(z: Point) -> Self {
        Self { z }
    }
}

impl Generator for Mandelbrot {
    type Output = impl Iterator<Item = Point>;

    fn generate(&self, p: Point) -> Self::Output {
        (0u32..).scan(self.z, move |acc, _| {
            *acc = Point::next(p, *acc);
            Some(*acc)
        })
    }
}

#[test]
fn mandelbrot_test() {
    let set = Mandelbrot::new(Point(0.0, 0.0)).generate(Point(0.5, 0.0));
    for (left, right) in [0.5, 0.75, 1.0625, 1.62891]
        .iter()
        .zip(set.map(|Point(x, _)| x))
    {
        let val = left - right;
        let val = val * val;
        assert!(val < 0.001, format!("Expected {} to match {}", left, right));
    }
}
