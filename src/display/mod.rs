use defmt::warn;
use embassy_rp::{
    i2c::{self, I2c},
    peripherals::I2C1,
};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, watch::Watch};
use embassy_time::Timer;
use embedded_graphics::{
    mono_font::{
        MonoTextStyle,
        ascii::{FONT_6X10, FONT_10X20},
    },
    pixelcolor::BinaryColor,
    prelude::*,
    text::Text,
};
use ssd1306::{Ssd1306, Ssd1306Async, prelude::*};

use crate::display::loading::LoadingLayout;

mod loading;

pub type Display = Ssd1306Async<
    I2CInterface<I2c<'static, I2C1, i2c::Async>>,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize128x64>,
>;
pub type DisplayError = <Display as DrawTarget>::Error;

#[derive(Debug, Clone)]
pub enum Page {
    Loading,
}

pub static DISPLAY_REFRESH: Channel<CriticalSectionRawMutex, bool, 1> = Channel::new();
pub static PAGE: Watch<CriticalSectionRawMutex, Page, 2> = Watch::new();

pub const DEFAULT_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> =
    MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
pub const LARGE_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> =
    MonoTextStyle::new(&FONT_10X20, BinaryColor::On);

#[embassy_executor::task]
pub async fn display_handler(mut display: Display) {
    let display_area = display.bounding_box();
    let mut page_rx = PAGE.receiver().unwrap();

    loop {
        DISPLAY_REFRESH.receive().await;
        display.clear_buffer();

        let page = page_rx.get().await;

        let d = match page {
            Page::Loading => LoadingLayout::new(display_area, "Loading...").draw(&mut display),
        };
        if d.is_err() {
            warn!("failed to draw");
        }

        if display.flush().await.is_err() {
            Timer::after_millis(2).await;
            display.flush().await.is_ok();
        }
    }
}
