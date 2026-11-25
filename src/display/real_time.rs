use core::sync::atomic::Ordering;

use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    pixelcolor::BinaryColor,
    primitives::{Circle, Primitive, Rectangle},
    text::Text,
};
use embedded_layout::{
    align::{Align, horizontal, vertical},
    layout::linear::{LinearLayout, spacing},
    object_chain::Chain,
    view_group::Views,
};

use crate::{
    cv::INPUT_MONITOR,
    display::{
        UIState,
        footer::Footer,
        shared::{BORDERED_STYLE, FILLED_STYLE, LARGE_TEXT_STYLE},
    },
};

pub struct RealTimeLayout {
    display_area: Rectangle,
    state: UIState,
}

impl RealTimeLayout {
    pub fn new(display_area: Rectangle, state: UIState) -> Self {
        RealTimeLayout {
            display_area,
            state,
        }
    }
}

impl Drawable for RealTimeLayout {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let title_text = Text::new("Real-time", Point::zero(), LARGE_TEXT_STYLE);

        let input_monitor = INPUT_MONITOR.load(Ordering::Relaxed);

        let mut indicators = [
            Circle::new(Point::zero(), 4).into_styled(match input_monitor & 1 {
                0 => BORDERED_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match input_monitor & 2 {
                0 => BORDERED_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match input_monitor & 4 {
                0 => BORDERED_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match input_monitor & 8 {
                0 => BORDERED_STYLE,
                _ => FILLED_STYLE,
            }),
        ];

        let indicator_layout = LinearLayout::horizontal(Views::new(&mut indicators))
            .with_alignment(vertical::Center)
            .with_spacing(spacing::DistributeFill(100))
            .arrange();

        let footer = Footer::new(self.state, false);

        LinearLayout::vertical(
            Chain::new(title_text)
                .append(indicator_layout)
                .append(footer),
        )
        .with_alignment(horizontal::Center)
        .with_spacing(spacing::DistributeFill(60))
        .arrange()
        .align_to(&self.display_area, horizontal::Center, vertical::Center)
        .draw(target)
    }
}
