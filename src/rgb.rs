use crate::fractal::{Grid, Palette, Renderer};
use colorbrewer::{get_color_ramp, Palette as ColorPalette};
use pixel_canvas::{Color, Image};
use rayon::prelude::*;

#[derive(Clone, Default)]
pub struct RGBPalette {
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

pub struct RGBRenderer<'a> {
    image: &'a mut Image,
}

impl<'a> RGBRenderer<'a> {
    pub fn new(image: &'a mut Image) -> Self {
        Self { image }
    }
}

impl<'a> Renderer for RGBRenderer<'a> {
    type Item = Color;

    fn render(&mut self, grid: Grid<Color>) {
        let width = self.image.width() as usize;
        self.image
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, pixel)| {
                let y = i / width;
                let x = i % width;
                *pixel = grid.0[y][x];
            });
    }
}
