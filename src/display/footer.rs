use dsp_core::controls::api_to_cv::APIToCVModes;
use embedded_graphics::{
    mono_font::MonoTextStyle,
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::Text,
};
use embedded_layout::{
    layout::linear::{LinearLayout, spacing},
    prelude::*,
};

use crate::display::{UIState, shared::DEFAULT_TEXT_STYLE};

pub const FOOTER_ITEM_COUNT: usize = 4;

type ComposedItem<'a> = Link<
    Text<'a, MonoTextStyle<'a, BinaryColor>>,
    Chain<embedded_graphics::primitives::Styled<Rectangle, PrimitiveStyle<BinaryColor>>>,
>;
pub struct Item<'a> {
    text: &'a str,
    active: bool,
}

impl<'a> Item<'a> {
    pub fn new(text: &'a str, active: bool) -> Self {
        Item { text, active }
    }

    pub fn format(self) -> ComposedItem<'a> {
        let button_style = PrimitiveStyleBuilder::new()
            .stroke_width(match self.active {
                true => 1,
                false => 0,
            })
            .stroke_color(BinaryColor::On)
            .build();

        let container = Rectangle::new(
            Point::zero(),
            Size {
                width: 24,
                height: 12,
            },
        )
        .into_styled(button_style);
        let text = Text::new(self.text, Point::zero(), DEFAULT_TEXT_STYLE);

        Chain::new(container).append(text.align_to(
            &container,
            horizontal::Center,
            vertical::Center,
        ))
    }
}

pub struct Footer {
    state: UIState,
    show_back: bool,
    position: Point,
}

impl Footer {
    pub fn new(state: UIState, show_back: bool) -> Self {
        Footer {
            state,
            show_back,
            position: Point::zero(),
        }
    }

    fn format_mode(&self) -> &str {
        match self.state.mode {
            APIToCVModes::RealTime => "RT",
            APIToCVModes::Clocked => "Clk",
        }
    }

    // TODO: make this play/stop symbols instead of text?
    fn format_running(&self) -> &str {
        match self.state.running {
            true => "on",
            false => "off",
        }
    }

    // TODO: symbols
    fn format_network(&self) -> &str {
        match self.state.network_connected {
            true => "net",
            false => "!net",
        }
    }

    fn arrange(&self) -> impl View + Drawable<Color = BinaryColor, Output = ()> {
        let back_text = if self.show_back { "<-" } else { "" };

        LinearLayout::horizontal(
            Chain::new(
                Item::new(
                    back_text,
                    self.show_back && self.state.navigation_location == 0,
                )
                .format(),
            )
            .append(Item::new(self.format_mode(), self.state.navigation_location == 1).format())
            .append(Item::new(self.format_running(), self.state.navigation_location == 2).format())
            .append(Item::new(self.format_network(), self.state.navigation_location == 3).format()),
        )
        .with_spacing(spacing::DistributeFill(124))
        .arrange()
        .translate(self.position)
    }
}

impl View for Footer {
    fn translate_impl(&mut self, by: Point) {
        self.position += by;
    }

    fn bounds(&self) -> Rectangle {
        self.arrange().bounds()
    }
}

impl Drawable for Footer {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.arrange().draw(target)
    }
}
