use crate::fractal::{Grid, Palette, Renderer};

pub struct CharPalette;

const CHAR_PALETTE: &str = "  ,.'\"~:;o-!|?/<>X+={^0#%&@8*$";

impl Palette for CharPalette {
    type Item = char;
    type Output = impl Iterator<Item = char>;

    fn get(&self) -> Self::Output {
        CHAR_PALETTE.chars()
    }
}

pub struct CharRenderer;

impl Renderer for CharRenderer {
    type Item = char;

    fn render(&mut self, grid: Grid<char>) {
        for row in grid.0 {
            println!("{}", row.iter().collect::<String>())
        }
    }
}
