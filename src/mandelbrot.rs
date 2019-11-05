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
