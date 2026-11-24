
use dsp_core::controls::api_to_cv::APIToCVModes;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    pixelcolor::BinaryColor,
    primitives::Rectangle,
    text::Text,
};
use embedded_layout::{
    align::{Align, horizontal, vertical},
    layout::linear::{LinearLayout, spacing},
    object_chain::Chain,
};

use crate::display::{
    UIState,
    footer::{FOOTER_ITEM_COUNT, Footer},
    shared::{LARGE_TEXT_STYLE, SelectableItem},
};

pub const MODE_COUNT: usize = 2;

pub struct ModeSelectionLayout {
    display_area: Rectangle,
    state: UIState,
}

impl ModeSelectionLayout {
    pub fn new(display_area: Rectangle, state: UIState) -> Self {
        ModeSelectionLayout {
            display_area,
            state,
        }
    }
}

impl Drawable for ModeSelectionLayout {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let title_text = Text::new("Mode", Point::zero(), LARGE_TEXT_STYLE);

        let realtime_text = Text::new("Real-time", Point::zero(), LARGE_TEXT_STYLE);
        let realtime = SelectableItem::new(
            &realtime_text,
            Size {
                width: 40,
                height: 20,
            },
            self.state.navigation_location == FOOTER_ITEM_COUNT,
            self.state.mode == APIToCVModes::RealTime,
        );

        let clocked_text = Text::new("Clocked", Point::zero(), LARGE_TEXT_STYLE);
        let clocked = SelectableItem::new(
            &clocked_text,
            Size {
                width: 40,
                height: 20,
            },
            self.state.navigation_location == FOOTER_ITEM_COUNT + 1,
            self.state.mode == APIToCVModes::Clocked,
        );

        let options = LinearLayout::horizontal(Chain::new(realtime).append(clocked))
            .with_alignment(vertical::Center)
            .with_spacing(spacing::DistributeFill(100))
            .arrange();

        let footer = Footer::new(self.state);

        LinearLayout::vertical(Chain::new(title_text).append(options).append(footer))
            .with_alignment(horizontal::Center)
            .with_spacing(spacing::DistributeFill(60))
            .arrange()
            .align_to(&self.display_area, horizontal::Center, vertical::Center)
            .draw(target)
    }
}
