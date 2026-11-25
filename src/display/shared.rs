use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    mono_font::{
        MonoTextStyle,
        ascii::{FONT_4X6, FONT_6X10, FONT_9X18},
    },
    pixelcolor::BinaryColor,
    primitives::{Circle, Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};
use embedded_layout::{
    View,
    align::{Align, horizontal, vertical},
    object_chain::Chain,
};

pub const EMPTY_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyle::new();
pub const BORDERED_STYLE: PrimitiveStyle<BinaryColor> =
    PrimitiveStyle::with_stroke(BinaryColor::On, 1);
pub const FILLED_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyle::with_fill(BinaryColor::On);

pub const DEFAULT_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> =
    MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
pub const LARGE_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> =
    MonoTextStyle::new(&FONT_9X18, BinaryColor::On);
pub const SMALL_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> =
    MonoTextStyle::new(&FONT_4X6, BinaryColor::On);

pub struct SelectableItem<'a> {
    text: &'a Text<'a, MonoTextStyle<'a, BinaryColor>>,
    size: Size,
    selected: bool,
    on: bool,
    position: Point,
}

impl<'a> SelectableItem<'a> {
    pub fn new(
        text: &'a Text<'a, MonoTextStyle<'a, BinaryColor>>,
        size: Size,
        selected: bool,
        on: bool,
    ) -> Self {
        SelectableItem {
            text,
            size,
            selected,
            on,
            position: Point::zero(),
        }
    }

    fn arrange(&self) -> impl View + Drawable<Color = BinaryColor, Output = ()> {
        let container = Rectangle::new(Point::zero(), self.size).into_styled(match self.selected {
            true => BORDERED_STYLE,
            false => EMPTY_STYLE,
        });
        let toggle = Circle::new(Point::zero(), 4).into_styled(match self.on {
            true => FILLED_STYLE,
            false => EMPTY_STYLE,
        });

        Chain::new(container)
            .append(
                self.text
                    .align_to(&container, horizontal::Center, vertical::Center),
            )
            .append(toggle.align_to(&container, horizontal::Center, vertical::Bottom))
            .translate(self.position)
    }
}

impl<'a> View for SelectableItem<'a> {
    fn translate_impl(&mut self, by: Point) {
        self.position += by;
    }

    fn bounds(&self) -> Rectangle {
        self.arrange().bounds()
    }
}

impl<'a> Drawable for SelectableItem<'a> {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        self.arrange().draw(target)
    }
}
