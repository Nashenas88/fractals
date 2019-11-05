use fractals::{draw, rgb::RGBRenderer, Generator, Grid, Palette, Point};

use pixel_canvas::{
    canvas::CanvasInfo,
    input::{Event, WindowEvent},
    Color, Image, XY,
};
use std::cell::RefCell;
use winit::event::{ElementState, MouseButton};

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

pub struct CanvasState {
    initial_dims: CanvasDims,
    render_state: RefCell<RenderState>,
}

impl CanvasState {
    pub fn new(min: Point, max: Point) -> Self {
        Self {
            initial_dims: CanvasDims { min, max },
            render_state: RefCell::new(RenderState::Recalc(min, max)),
        }
    }

    pub fn handle_input(info: &CanvasInfo, state: &mut Self, event: &Event<()>) -> bool {
        let window_event: &WindowEvent;
        if let Event::WindowEvent { event, .. } = event {
            window_event = event;
        } else {
            return false;
        }

        match (window_event, state.render_state.get_mut()) {
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
                *render_state = RenderState::Recalc(state.initial_dims.min, state.initial_dims.max);
                true
            }
            _ => false,
        }
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
        for (row_clone, row) in other
            .chunks_mut(self.width())
            .zip(self.chunks(self.width()))
        {
            for (pixel_clone, pixel) in row_clone.iter_mut().zip(row.iter()) {
                *pixel_clone = *pixel;
            }
        }
    }
}

pub const TERM_WIDTH: usize = 99;
pub const TERM_HEIGHT: usize = 37;

pub const RGB_WIDTH: usize = 960;
pub const RGB_HEIGHT: usize = 640;

pub fn zoomable_canvas_render<G: Generator, P: Palette<Item = Color>>(
    generator: G,
    palette: P,
) -> impl FnMut(&mut CanvasState, &mut Image)
where
    G: Sync,
    P: Sync,
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
                for x in start_x..end_x {
                    image[XY(x, start_y)] = highlight_color;
                    image[XY(x, end_y)] = highlight_color;
                }
                for y in start_y..end_y {
                    image[XY(start_x, y)] = highlight_color;
                    image[XY(end_x, y)] = highlight_color;
                }
            }
            RenderState::Recalc(min, max) => {
                let grid = Grid::new(RGB_WIDTH, RGB_HEIGHT, *min, *max);
                let mut renderer = RGBRenderer::new(image);
                draw(&generator, &palette, &mut renderer, &grid);
            }
            _ => {}
        }

        let min;
        let max;
        if let RenderState::Recalc(rcmin, rcmax) = &*canvas_state.render_state.borrow() {
            min = *rcmin;
            max = *rcmax;
        } else {
            return;
        }

        *canvas_state.render_state.borrow_mut() =
            RenderState::Done(min, max, Position::new(), image.clone());
    }
}
