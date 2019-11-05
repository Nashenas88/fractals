use fractals::{
    char::{CharPalette, CharRenderer},
    draw,
    julia::Julia,
    mandelbrot::Mandelbrot,
    rgb::RGBPalette,
    Grid, Point,
};
use gui::{zoomable_canvas_render, CanvasState, RGB_HEIGHT, RGB_WIDTH, TERM_HEIGHT, TERM_WIDTH};
use pixel_canvas::Canvas;
use structopt::StructOpt;

mod gui;

#[derive(StructOpt)]
struct Opt {
    #[structopt(subcommand)]
    julia: Option<JuliaOpt>,

    #[allow(clippy::option_option)]
    #[structopt(short, long, conflicts_with("julia"))]
    mandelbrot: Option<Option<f64>>,

    #[allow(dead_code)]
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
        z: f64,
    },
}

fn main() {
    let opt = Opt::from_args();
    if opt.image {
        let canvas = Canvas::new(RGB_WIDTH, RGB_HEIGHT);
        if let Some(JuliaOpt::Julia { z, p }) = opt.julia {
            canvas
                .title("Julia")
                .state(CanvasState::new(Point(-1.5, -1.5), Point(1.5, 1.5)))
                .input(CanvasState::handle_input)
                .render(zoomable_canvas_render(
                    Julia::new(Point(p, z)),
                    RGBPalette::new(),
                ))
        } else {
            let z = opt.mandelbrot.unwrap_or(Some(0.0)).unwrap_or(0.0);
            canvas
                .title("Mandelbrot")
                .state(CanvasState::new(Point(-2.25, -1.5), Point(0.75, 1.5)))
                .input(CanvasState::handle_input)
                .render(zoomable_canvas_render(
                    Mandelbrot::new(Point(z, z)),
                    RGBPalette::new(),
                ))
        }
    } else {
        let palette = CharPalette;
        let mut renderer = CharRenderer;
        if let Some(JuliaOpt::Julia { z, p }) = opt.julia {
            let grid = Grid::new(TERM_WIDTH, TERM_HEIGHT, Point(-1.5, -1.5), Point(1.5, 1.5));
            draw(&Julia::new(Point(p, z)), &palette, &mut renderer, &grid);
        } else {
            let z = opt.mandelbrot.unwrap_or(Some(0.0)).unwrap_or(0.0);
            let grid = Grid::new(
                TERM_WIDTH,
                TERM_HEIGHT,
                Point(-2.25, -1.5),
                Point(0.75, 1.5),
            );
            draw(
                &Mandelbrot::new(Point(z, z)),
                &palette,
                &mut renderer,
                &grid,
            );
        }
    }
}
