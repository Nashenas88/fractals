use rayon::prelude::*;

#[derive(Copy, Clone, Debug)]
pub struct Point(pub f64, pub f64);

impl Point {
    pub fn next(Point(u, v): Point, Point(x, y): Point) -> Point {
        Point(x * x - y * y + u, 2.0 * x * y + v)
    }

    fn fairly_close(Point(u, v): Point) -> bool {
        (u * u + v * v) < 100.0
    }
}

fn choose_color<C: Copy>(palette: impl Iterator<Item = C>, iter: impl Iterator<Item = Point>) -> C {
    let mut palette = palette.peekable();
    let first = *palette.peek().unwrap();
    palette
        .zip(iter)
        .take_while(|(_, p)| Point::fairly_close(*p))
        .fold(first, |_, (c, _)| c)
}

pub trait Generator {
    type Output: Iterator<Item = Point>;
    fn generate(&self, p: Point) -> Self::Output;
}

pub trait Palette {
    type Item: Copy;
    type Output: Iterator<Item = Self::Item>;
    fn get(&self) -> Self::Output;
}

fn make_image<G: Generator, P: Palette>(generator: &G, palette: &P, p: Point) -> P::Item
where
    P::Item: Copy,
{
    choose_color(palette.get(), generator.generate(p))
}

#[derive(Debug)]
pub struct Grid<C>(pub Vec<Vec<C>>);

impl Grid<Point> {
    pub fn new(
        col: usize,
        row: usize,
        Point(x_min, y_min): Point,
        Point(x_max, y_max): Point,
    ) -> Grid<Point> {
        assert!(x_min < x_max);
        assert!(y_min < y_max);
        let y_spread = (y_max - y_min) / (row - 1) as f64;
        let x_spread = (x_max - x_min) / (col - 1) as f64;
        let mut rows = vec![];
        rows.reserve(row);

        for r in 0..row {
            let mut curr_row = vec![];
            curr_row.reserve(col);
            let y = y_spread * r as f64 + y_min;

            for c in 0..col {
                let x = x_spread * c as f64 + x_min;
                curr_row.push(Point(x, y));
            }

            rows.push(curr_row);
        }

        Grid(rows)
    }
}

fn sample<G: Generator, P: Palette>(grid: &Grid<Point>, generator: &G, palette: &P) -> Grid<P::Item>
where
    P::Item: Copy + Send,
    G: Sync,
    P: Sync,
{
    Grid(
        grid.0
            .clone()
            .into_par_iter()
            .map(|c| {
                c.into_par_iter()
                    .map(|p| make_image(generator, palette, p))
                    .collect()
            })
            .collect(),
    )
}

pub trait Renderer {
    type Item: Copy;
    fn render(&mut self, grid: Grid<Self::Item>);
}

pub fn draw<G: Generator, P: Palette, R: Renderer<Item = P::Item>>(
    generator: &G,
    palette: &P,
    renderer: &mut R,
    points: &Grid<Point>,
) where
    G: Sync,
    P: Sync,
    P::Item: Send,
{
    renderer.render(sample(points, generator, palette))
}

#[test]
fn color_tests() {
    for (i, expected) in [0, 0, 1, 2, 3, 3].iter().enumerate() {
        let mut points = vec![Point(0.0, 0.0); i];
        points.push(Point(8.0, 8.0));
        assert_eq!(
            expected,
            choose_color([0, 1, 2, 3].iter(), points.into_iter())
        );
    }
}
