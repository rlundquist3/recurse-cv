use embedded_graphics::{
    Drawable, draw_target::DrawTarget, geometry::Point, pixelcolor::BinaryColor,
    primitives::Rectangle, text::Text,
};
use embedded_layout::{
    align::{Align, horizontal, vertical},
    layout::linear::{LinearLayout, spacing},
    object_chain::Chain,
};

use crate::display::shared::{LARGE_TEXT_STYLE, SMALL_TEXT_STYLE};

pub struct LoadingLayout<'a> {
    display_area: Rectangle,
    loading_text: &'a str,
}

impl<'a> LoadingLayout<'a> {
    pub fn new(display_area: Rectangle, loading_text: &'a str) -> Self {
        LoadingLayout {
            display_area,
            loading_text,
        }
    }
}

impl<'a> Drawable for LoadingLayout<'a> {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let title_text = Text::new("Recurse CV", Point::zero(), LARGE_TEXT_STYLE);
        let loading_text = Text::new(self.loading_text, Point::zero(), SMALL_TEXT_STYLE);

        LinearLayout::vertical(Chain::new(title_text).append(loading_text))
            .with_alignment(horizontal::Center)
            .with_spacing(spacing::DistributeFill(60))
            .arrange()
            .align_to(&self.display_area, horizontal::Center, vertical::Center)
            .draw(target)
    }
}
