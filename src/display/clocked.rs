use core::fmt::Write;
use core::sync::atomic::Ordering;
use heapless::String;

use dsp_core::controls::clock::{Clock, Note};
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
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
    cv::{INPUT_MONITOR, OUTPUT_MONITOR},
    display::{
        UIState,
        footer::{FOOTER_ITEM_COUNT, Footer},
        shared::{DEFAULT_TEXT_STYLE, EMPTY_STYLE, FILLED_STYLE, SelectableItem},
    },
};

pub const CLOCK_PARAM_COUNT: usize = 4;

pub struct ClockedLayout {
    display_area: Rectangle,
    state: UIState,
    per_measure: String<8>,
    base: String<8>,
    bpm: String<8>,
    division: String<8>,
}

impl ClockedLayout {
    pub fn new(display_area: Rectangle, state: UIState, clock: Clock) -> Self {
        let mut per_measure: String<8> = String::new();
        let mut base: String<8> = String::new();
        let mut bpm: String<8> = String::new();
        let mut division: String<8> = String::new();

        let _ = write!(per_measure, "{}", clock.time_signature.per_measure);
        let _ = write!(
            base,
            "{}",
            match clock.time_signature.base {
                Note::Eighth => 8,
                _ => 4,
            }
        );
        let _ = write!(bpm, "{} bpm", clock.bpm);
        let _ = write!(division, "{:?}", clock.division);

        ClockedLayout {
            display_area,
            state,
            per_measure,
            base,
            bpm,
            division,
        }
    }
}

impl Drawable for ClockedLayout {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, target: &mut D) -> Result<Self::Output, D::Error>
    where
        D: DrawTarget<Color = Self::Color>,
    {
        let top_text = Text::new(&self.per_measure, Point::zero(), DEFAULT_TEXT_STYLE);
        let top = SelectableItem::new(
            &top_text,
            Size {
                width: 10,
                height: 10,
            },
            self.state.navigation_location == FOOTER_ITEM_COUNT,
            false,
        );
        let bottom_text = Text::new(&self.base, Point::zero(), DEFAULT_TEXT_STYLE);
        let bottom = SelectableItem::new(
            &bottom_text,
            Size {
                width: 10,
                height: 10,
            },
            self.state.navigation_location == FOOTER_ITEM_COUNT + 1,
            false,
        );
        let bpm_text = Text::new(&self.bpm, Point::zero(), DEFAULT_TEXT_STYLE);
        let bpm = SelectableItem::new(
            &bpm_text,
            Size {
                width: 20,
                height: 10,
            },
            self.state.navigation_location == FOOTER_ITEM_COUNT + 2,
            false,
        );

        // TODO: note symbols instead of text
        let division_text = Text::new(&self.division, Point::zero(), DEFAULT_TEXT_STYLE);
        let division = SelectableItem::new(
            &division_text,
            Size {
                width: 20,
                height: 10,
            },
            self.state.navigation_location == FOOTER_ITEM_COUNT + 3,
            false,
        );

        let time_signature = LinearLayout::vertical(Chain::new(top).append(bottom))
            .with_alignment(horizontal::Center)
            .arrange();

        let parameters =
            LinearLayout::horizontal(Chain::new(time_signature).append(bpm).append(division))
                .with_alignment(vertical::Center)
                .with_spacing(spacing::DistributeFill(100))
                .arrange();

        let input_monitor = INPUT_MONITOR.load(Ordering::Relaxed);
        let output_monitor = OUTPUT_MONITOR.load(Ordering::Relaxed);

        let mut input_indicators = [
            Circle::new(Point::zero(), 4).into_styled(match input_monitor & 1 {
                0 => EMPTY_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match input_monitor & 2 {
                0 => EMPTY_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match input_monitor & 4 {
                0 => EMPTY_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match input_monitor & 8 {
                0 => EMPTY_STYLE,
                _ => FILLED_STYLE,
            }),
        ];
        let mut output_indicators = [
            Circle::new(Point::zero(), 4).into_styled(match output_monitor & 1 {
                0 => EMPTY_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match output_monitor & 2 {
                0 => EMPTY_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match output_monitor & 4 {
                0 => EMPTY_STYLE,
                _ => FILLED_STYLE,
            }),
            Circle::new(Point::zero(), 4).into_styled(match output_monitor & 8 {
                0 => EMPTY_STYLE,
                _ => FILLED_STYLE,
            }),
        ];

        let input_indicator_layout = LinearLayout::horizontal(Views::new(&mut input_indicators))
            .with_alignment(vertical::Center)
            .with_spacing(spacing::DistributeFill(100))
            .arrange();
        let output_indicator_layout = LinearLayout::horizontal(Views::new(&mut output_indicators))
            .with_alignment(vertical::Center)
            .with_spacing(spacing::DistributeFill(100))
            .arrange();

        let footer = Footer::new(self.state);

        LinearLayout::vertical(
            Chain::new(parameters)
                .append(input_indicator_layout)
                .append(output_indicator_layout)
                .append(footer),
        )
        .with_alignment(horizontal::Center)
        .with_spacing(spacing::DistributeFill(60))
        .arrange()
        .align_to(&self.display_area, horizontal::Center, vertical::Center)
        .draw(target)
    }
}
