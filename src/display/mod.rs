use core::{cell::RefCell, sync::atomic::Ordering};

use defmt::warn;
use dsp_core::controls::{
    api_to_cv::{self, APIToCVModes},
    clock::Clock,
};
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

use crate::{
    MODE, NETWORK_CONNECTED, RUNNING, SharedAPIToCV, SharedClock,
    display::{
        clocked::ClockedLayout, loading::LoadingLayout, mode::ModeSelectionLayout,
        real_time::RealTimeLayout,
    },
};

pub mod clocked;
pub mod footer;
mod loading;
pub mod mode;
mod real_time;
pub mod shared;

pub type Display = Ssd1306Async<
    I2CInterface<I2c<'static, I2C1, i2c::Async>>,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize128x64>,
>;
pub type DisplayError = <Display as DrawTarget>::Error;

#[derive(Debug, Clone)]
pub enum Page {
    Loading,
    ModeSelection,
    RealTime,
    Clocked,
}

#[derive(Debug, Clone, Copy)]
pub struct UIState {
    navigation_location: usize,
    mode: APIToCVModes,
    running: bool,
    network_connected: bool,
}

pub static DISPLAY_REFRESH: Channel<CriticalSectionRawMutex, bool, 2> = Channel::new();
pub static LOADING_MESSAGE: Channel<CriticalSectionRawMutex, &str, 4> = Channel::new();
pub static PAGE: Watch<CriticalSectionRawMutex, Page, 2> = Watch::new();
pub static NAVIGATION_LOCATION: Watch<CriticalSectionRawMutex, usize, 2> = Watch::new();

#[embassy_executor::task]
pub async fn display_handler(
    mut display: Display,
    api_to_cv: &'static SharedAPIToCV,
    clock: &'static SharedClock,
) {
    let display_area = display.bounding_box();
    let mut page_rx = PAGE.receiver().unwrap();
    let mut navigation_rx = NAVIGATION_LOCATION.receiver().unwrap();

    loop {
        let _ = DISPLAY_REFRESH.receive().await;
        display.clear_buffer();
        let page = page_rx.get().await;
        let navigation_location = navigation_rx.get().await;

        let mode = match MODE.load(Ordering::Relaxed) {
            1 => APIToCVModes::Clocked,
            _ => APIToCVModes::RealTime,
        };
        let state = UIState {
            navigation_location,
            mode,
            running: RUNNING.load(Ordering::Relaxed),
            network_connected: NETWORK_CONNECTED.load(Ordering::Relaxed),
        };

        let d = match page {
            Page::Loading => {
                let message = match LOADING_MESSAGE.try_peek() {
                    Ok(message) => message,
                    Err(_) => "Loading...",
                };
                LoadingLayout::new(display_area, message).draw(&mut display)
            }
            Page::ModeSelection => ModeSelectionLayout::new(display_area, state).draw(&mut display),
            Page::RealTime => RealTimeLayout::new(display_area, state).draw(&mut display),
            Page::Clocked => {
                let clock_clone = clock.lock(|c: &RefCell<Clock>| c.borrow().clone());
                ClockedLayout::new(display_area, state, clock_clone).draw(&mut display)
            }
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
