#![feature(type_alias_impl_trait)]

use colorbrewer::{get_color_ramp, Palette as ColorPalette};
use pixel_canvas::{
    canvas::CanvasInfo,
    input::{Event, WindowEvent},
    Canvas, Color, Image, XY,
};
use rayon::prelude::*;
use std::cell::RefCell;
use structopt::StructOpt;
use winit::event::{ElementState, MouseButton};

#[derive(Copy, Clone, Debug)]
struct Point(f64, f64);

impl Point {
    fn next(Point(u, v): Point, Point(x, y): Point) -> Point {
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

trait Generator {
    type Output: Iterator<Item = Point>;
    fn generate(&self, p: Point) -> Self::Output;
}

struct Mandelbrot {
    z: Point,
}

impl Mandelbrot {
    fn new(z: Point) -> Self {
        Self {
            z,
        }
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

struct Julia {
    z: Point,
}

impl Julia {
    fn new(z: Point) -> Self {
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

trait Palette {
    type Item: Copy;
    type Output: Iterator<Item = Self::Item>;
    fn get(&self) -> Self::Output;
}

struct CharPalette;

const CHAR_PALETTE: &str = "  ,.'\"~:;o-!|?/<>X+={^0#%&@8*$";

impl Palette for CharPalette {
    type Item = char;
    type Output = impl Iterator<Item = char>;

    fn get(&self) -> Self::Output {
        CHAR_PALETTE.chars()
    }
}

#[derive(Clone)]
struct RGBPalette {
    ramp: Vec<String>,
}

impl RGBPalette {
    pub fn new() -> Self {
        Self {
            ramp: get_color_ramp(ColorPalette::OrRd, 9)
                .unwrap()
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }
}

impl Palette for RGBPalette {
    type Item = Color;
    type Output = impl Iterator<Item = Color>;

    fn get(&self) -> Self::Output {
        let x = self.ramp.clone().into_iter();
        let x2 = x.clone().rev();
        x.chain(x2).map(|color_str| {
            let colors: Vec<_> = color_str
                .chars()
                .skip(1)
                .collect::<Vec<_>>()
                .chunks(2)
                .map(|chunk| {
                    let chunks = &[chunk[0] as u8, chunk[1] as u8];
                    let color_str = String::from_utf8_lossy(chunks);
                    u8::from_str_radix(&color_str, 16).unwrap()
                })
                .collect();
            let colors: Vec<Color> = colors
                .chunks(3)
                .map(|chunk| Color::rgb(chunk[0], chunk[1], chunk[2]))
                .collect::<Vec<_>>();
            colors[0]
        })
    }
}

fn make_image<G: Generator, P: Palette>(generator: &G, palette: &P, p: Point) -> P::Item
where
    P::Item: Copy,
{
    choose_color(palette.get(), generator.generate(p))
}

#[derive(Debug)]
struct Grid<C>(Vec<Vec<C>>);

impl Grid<Point> {
    fn new(
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

trait Renderer {
    type Item: Copy;
    fn render(&mut self, grid: Grid<Self::Item>);
}

struct CharRenderer;

impl Renderer for CharRenderer {
    type Item = char;

    fn render(&mut self, grid: Grid<char>) {
        for row in grid.0 {
            println!("{}", row.iter().collect::<String>())
        }
    }
}

struct RGBRenderer<'a> {
    image: &'a mut Image,
}

impl<'a> RGBRenderer<'a> {
    fn new(image: &'a mut Image) -> Self {
        Self { image }
    }
}

impl<'a> Renderer for RGBRenderer<'a> {
    type Item = Color;

    fn render(&mut self, grid: Grid<Color>) {
        let width = self.image.width() as usize;
        self.image.par_iter_mut().enumerate().for_each(|(i, pixel)| {
            let y = i / width;
            let x = i % width;
            *pixel = grid.0[y][x];
        });
        // self.image.chunks_mut(width).into_par_iter().enumerate().for_each(|(y, row)| {
        //     for (x, pixel) in row.iter_mut().enumerate() {
        //         *pixel = grid.0[y][x];
        //     }
        // })
    }
}

fn draw<G: Generator, P: Palette, R: Renderer<Item = P::Item>>(
    generator: &G,
    palette: &P,
    renderer: &mut R,
    points: &Grid<Point>,
) where G: Sync, P: Sync, P::Item: Send {
    renderer.render(sample(points, generator, palette))
}

#[derive(Copy, Clone, Debug)]
struct Position {
    virtual_x: i32,
    virtual_y: i32,
    x: i32,
    y: i32,
}

impl Position {
    fn new() -> Self {
        Self {
            virtual_x: 0,
            virtual_y: 0,
            x: 0,
            y: 0,
        }
    }
}

struct DraggingState {
    initial_click: Position,
    current: Position,
    dims: CanvasDims,
    image: Image,
}

#[derive(Copy, Clone, Debug)]
struct CanvasDims {
    min: Point,
    max: Point,
}

enum RenderState {
    Dragging(DraggingState),
    Recalc(Point, Point),
    Done(Point, Point, Position, Image),
}

struct CanvasState {
    initial_dims: CanvasDims,
    render_state: RefCell<RenderState>,
}

impl CanvasState {
    fn new(min: Point, max: Point) -> Self {
        Self {
            initial_dims: CanvasDims { min, max },
            render_state: RefCell::new(RenderState::Recalc(min, max)),
        }
    }

    fn handle_input(info: &CanvasInfo, state: &mut Self, event: &Event<()>) -> bool {
        let window_event: &WindowEvent;
        if let Event::WindowEvent { event, .. } = event {
            window_event = event;
        } else {
            return false;
        }

        let res = match (window_event, state.render_state.get_mut()) {
            // Enter zoom selection mode
            (
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                },
                RenderState::Done(min, max, position, image),
            ) => {
                state.render_state = RefCell::new(RenderState::Dragging(DraggingState {
                    initial_click: *position,
                    current: *position,
                    dims: CanvasDims {
                        min: *min,
                        max: *max,
                    },
                    image: image.clone(),
                }));
                // nothing to draw in dragging state ... for now ;)
                false
            }
            // update cursor for redrawing
            (
                WindowEvent::CursorMoved { position, .. },
                RenderState::Dragging(DraggingState { current: pos, .. }),
            )
            | (WindowEvent::CursorMoved { position, .. }, RenderState::Done(_, _, pos, _)) => {
                let (x, y): (i32, i32) = (*position).into();
                pos.virtual_x = x;
                pos.virtual_y = y;
                pos.x = (x as f64 * info.dpi) as i32;
                pos.y = ((info.height as i32 - y) as f64 * info.dpi) as i32;
                // don't redraw on cursor movement
                false
            }
            // setup state for recomputing a new scene
            (
                WindowEvent::MouseInput {
                    state: ElementState::Released,
                    button: MouseButton::Left,
                    ..
                },
                RenderState::Dragging(dragging_state),
            ) => {
                let (min_x, max_x) = if dragging_state.current.x < dragging_state.initial_click.x {
                    (dragging_state.current.x, dragging_state.initial_click.x)
                } else {
                    (dragging_state.initial_click.x, dragging_state.current.x)
                };

                let (min_y, max_y) = if dragging_state.current.y < dragging_state.initial_click.y {
                    (dragging_state.current.y, dragging_state.initial_click.y)
                } else {
                    (dragging_state.initial_click.y, dragging_state.current.y)
                };

                let x_ratio =
                    (dragging_state.dims.max.0 - dragging_state.dims.min.0) / info.width as f64;
                let y_ratio =
                    (dragging_state.dims.max.1 - dragging_state.dims.min.1) / info.height as f64;

                let min_x = min_x as f64 * x_ratio + dragging_state.dims.min.0;
                let max_x = max_x as f64 * x_ratio + dragging_state.dims.min.0;
                let min_y = min_y as f64 * y_ratio + dragging_state.dims.min.1;
                let max_y = max_y as f64 * y_ratio + dragging_state.dims.min.1;

                state.render_state = RefCell::new(RenderState::Recalc(
                    Point(min_x, min_y),
                    Point(max_x, max_y),
                ));
                true
            }
            // reset to original view
            (
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Right,
                    ..
                },
                render_state @ RenderState::Done(..),
            ) => {
                *render_state = RenderState::Recalc(
                    state.initial_dims.min,
                    state.initial_dims.max,
                );
                true
            }
            _ => false,
        };

        res
    }
}

// Because Image doesn't have Clone implemented \(-_-)/
trait ManualClone {
    fn clone(&self) -> Self;
    fn clone_onto(&self, other: &mut Self);
}

impl ManualClone for Image {
    fn clone(&self) -> Self {
        let width = self.width();
        let mut image = Image::new(width, self.height());
        self.clone_onto(&mut image);
        image
    }

    fn clone_onto(&self, other: &mut Self) {
        assert!(self.width() == other.width() && self.height() == other.height());
        for (row_clone, row) in other.chunks_mut(self.width()).zip(self.chunks(self.width())) {
            for (pixel_clone, pixel) in row_clone.iter_mut().zip(row.iter()) {
                *pixel_clone = *pixel;
            }
        }

    }
}

const TERM_WIDTH: usize = 99;
const TERM_HEIGHT: usize = 37;

const RGB_WIDTH: usize = 960;
const RGB_HEIGHT: usize = 640;

fn zoomable_canvas_render<G: Generator, P: Palette<Item = Color>>(
    generator: G, palette: P
) -> impl FnMut(&mut CanvasState, &mut Image)
    where G: Sync, P: Sync
{
    move |canvas_state, image| {
        match &*canvas_state.render_state.borrow() {
            RenderState::Dragging(DraggingState {
                initial_click,
                current,
                image: previous_image,
                ..
            }) => {
                previous_image.clone_onto(image);
                let highlight_color = Color::rgb(0, 255, 0);
                let (start_x, end_x) = if initial_click.x < current.x {
                    (initial_click.x as usize, current.x as usize)
                } else {
                    (current.x as usize, initial_click.x as usize)
                };
                let (start_y, end_y) = if initial_click.y < current.y {
                    (initial_click.y as usize, current.y as usize)
                } else {
                    (current.y as usize, initial_click.y as usize)
                };

                // draw a box in the highlight color
                for x in start_x .. end_x {
                    image[XY(x, start_y)] = highlight_color;
                    image[XY(x, end_y)]   = highlight_color;
                }
                for y in start_y .. end_y {
                    image[XY(start_x, y)] = highlight_color;
                    image[XY(end_x,   y)] = highlight_color;
                }
            },
            RenderState::Recalc(min, max) => {
                let grid = Grid::new(RGB_WIDTH, RGB_HEIGHT, *min, *max);
                let mut renderer = RGBRenderer::new(image);
                draw(&generator, &palette, &mut renderer, &grid);
            },
            _ => {},
        }

        let min;
        let max;
        if let RenderState::Recalc(rcmin, rcmax) = &*canvas_state.render_state.borrow() {
            min = *rcmin;
            max = *rcmax;
        } else {
            return;
        }

        *canvas_state.render_state.borrow_mut() = RenderState::Done(min, max, Position::new(), image.clone());
    }
}

#[allow(dead_code)]
#[derive(StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    julia: Option<JuliaOpt>,
    #[structopt(short, long, conflicts_with("julia"))]
    mandelbrot: Option<Option<f64>>,
    #[structopt(short, long, conflicts_with("image"))]
    text: bool,
    #[structopt(short, long, conflicts_with("text"))]
    image: bool,
}

#[derive(StructOpt)]
enum JuliaOpt {
    Julia {
        #[structopt(short, default_value = "0.32")]
        p: f64,
        #[structopt(short, default_value = "0.043")]
        z: f64
    }
}

fn main() {
    let opt = Opt::from_args();
    if opt.image {
        let canvas = Canvas::new(RGB_WIDTH, RGB_HEIGHT);
        if let Some(JuliaOpt::Julia { z, p }) = opt.julia {
            canvas.title("Julia")
                .state(CanvasState::new(Point(-1.5, -1.5), Point(1.5, 1.5)))
                .input(CanvasState::handle_input)
                .render(zoomable_canvas_render(Julia::new(Point(p, z)), RGBPalette::new()))
        } else {
            let z = opt.mandelbrot.unwrap_or(Some(0.0)).unwrap_or(0.0);
            canvas.title("Mandelbrot")
                .state(CanvasState::new(Point(-2.25, -1.5), Point(0.75, 1.5)))
                .input(CanvasState::handle_input)
                .render(zoomable_canvas_render(Mandelbrot::new(Point(z, z)), RGBPalette::new()))
        }
    } else {
        let palette = CharPalette;
        let mut renderer = CharRenderer;
        if let Some(JuliaOpt::Julia { z, p }) = opt.julia {
            let grid = Grid::new(TERM_WIDTH, TERM_HEIGHT, Point(-1.5, -1.5), Point(1.5, 1.5));
            draw(&Julia::new(Point(p, z)), &palette, &mut renderer, &grid);
        } else {
            let z = opt.mandelbrot.unwrap_or(Some(0.0)).unwrap_or(0.0);
            let grid = Grid::new(TERM_WIDTH, TERM_HEIGHT, Point(-2.25, -1.5), Point(0.75, 1.5));
            draw(&Mandelbrot::new(Point(z, z)), &palette, &mut renderer, &grid);
        }
    }
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
