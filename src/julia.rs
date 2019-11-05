use crate::fractal::{Generator, Point};

pub struct Julia {
    z: Point,
}

impl Julia {
    pub fn new(z: Point) -> Self {
        Self { z }
    }
}

impl Generator for Julia {
    type Output = impl Iterator<Item = Point>;

    fn generate(&self, p: Point) -> Self::Output {
        let z = self.z;
        (0u32..).scan(p, move |acc, _| {
            *acc = Point::next(z, *acc);
            Some(*acc)
        })
    }
}
