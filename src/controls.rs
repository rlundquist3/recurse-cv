use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, Ordering},
};

use dsp_core::controls::clock::Clock;
use embassy_rp::gpio::Input;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant, Timer};

use crate::{
    MODE, RUNNING, SharedAPIToCV, SharedClock,
    display::{
        NAVIGATION_LOCATION, PAGE, Page, clocked::CLOCK_PARAM_COUNT, footer::FOOTER_ITEM_COUNT,
        mode::MODE_COUNT,
    },
};

const DEBOUNCE: Duration = Duration::from_millis(250);

pub static CONTROL_BUFFER: Channel<CriticalSectionRawMutex, EncoderEvent, 16> = Channel::new();

static IS_EDITING: AtomicBool = AtomicBool::new(false);

const REALTIME: usize = FOOTER_ITEM_COUNT;
const CLOCKED: usize = FOOTER_ITEM_COUNT + 1;
const PER_MEASURE: usize = FOOTER_ITEM_COUNT;
const BASE: usize = FOOTER_ITEM_COUNT + 1;
const BPM: usize = FOOTER_ITEM_COUNT + 2;
const DIVISION: usize = FOOTER_ITEM_COUNT + 3;

#[derive(Debug)]
pub enum EncoderEvent {
    Clockwise,
    Counterclockwise,
    Click,
}

#[embassy_executor::task]
pub async fn encoder_rotate_handler(mut clk: Input<'static>, dt: Input<'static>) {
    let mut start = Instant::now();
    let mut last_clk_state = clk.get_level();

    loop {
        clk.wait_for_any_edge().await;
        let clk_state = clk.get_level();

        if last_clk_state != clk_state {
            let dt_state = dt.get_level();

            let encoder_event = match clk_state == dt_state {
                true => EncoderEvent::Counterclockwise,
                false => EncoderEvent::Clockwise,
            };

            if start.elapsed() >= DEBOUNCE {
                CONTROL_BUFFER.send(encoder_event).await;
                start = Instant::now();
            }
        }

        last_clk_state = clk_state;
    }
}

#[embassy_executor::task]
pub async fn encoder_click_handler(mut sw: Input<'static>) {
    loop {
        sw.wait_for_low().await;
        CONTROL_BUFFER.send(EncoderEvent::Click).await;

        Timer::after(DEBOUNCE).await;
    }
}

#[embassy_executor::task]
pub async fn control_handler(_api_to_cv: &'static SharedAPIToCV, clock: &'static SharedClock) {
    let buffer_rx = CONTROL_BUFFER.receiver();
    let mut page_rx = PAGE.receiver().unwrap();
    let page = page_rx.get().await;

    loop {
        let encoder_event = buffer_rx.receive().await;
        match page {
            Page::ModeSelection => {
                mode_handler(encoder_event).await;
            }
            Page::RealTime => {
                real_time_handler(encoder_event).await;
            }
            Page::Clocked => {
                clocked_handler(encoder_event, clock).await;
            }
            _ => {}
        }
    }
}

fn footer_click_handler(navigation_location: usize) {
    let page_tx = PAGE.sender();
    match navigation_location {
        0 => match MODE.load(Ordering::Relaxed) {
            1 => page_tx.send(Page::Clocked),
            _ => page_tx.send(Page::RealTime),
        },
        1 => page_tx.send(Page::ModeSelection),
        2 => RUNNING.store(!RUNNING.load(Ordering::Relaxed), Ordering::Relaxed),
        3 => {} // TODO: network settings
        _ => {}
    }
}

async fn mode_handler(encoder_event: EncoderEvent) {
    let page_tx = PAGE.sender();
    let navigation_tx = NAVIGATION_LOCATION.sender();
    let mut navigation_rx = NAVIGATION_LOCATION.receiver().unwrap();

    let navigation_location = navigation_rx.get().await;
    match encoder_event {
        EncoderEvent::Clockwise => {
            if navigation_location < FOOTER_ITEM_COUNT + MODE_COUNT {
                navigation_tx.send(navigation_location + 1);
            } else {
                navigation_tx.send(0);
            }
        }
        EncoderEvent::Counterclockwise => {
            if navigation_location > 0 {
                navigation_tx.send(navigation_location - 1);
            } else {
                navigation_tx.send(FOOTER_ITEM_COUNT + MODE_COUNT - 1);
            }
        }
        EncoderEvent::Click => {
            if navigation_location < FOOTER_ITEM_COUNT {
                footer_click_handler(navigation_location);
            } else {
                match navigation_location {
                    REALTIME => {
                        MODE.store(0, Ordering::Relaxed);
                        page_tx.send(Page::RealTime);
                    }
                    CLOCKED => {
                        MODE.store(1, Ordering::Relaxed);
                        page_tx.send(Page::Clocked);
                    }
                    _ => {}
                }
            }
        }
    }
}

async fn real_time_handler(encoder_event: EncoderEvent) {
    let _page_tx = PAGE.sender();
    let navigation_tx = NAVIGATION_LOCATION.sender();
    let mut navigation_rx = NAVIGATION_LOCATION.receiver().unwrap();

    let navigation_location = navigation_rx.get().await;
    match encoder_event {
        EncoderEvent::Clockwise => {
            if navigation_location < FOOTER_ITEM_COUNT {
                navigation_tx.send(navigation_location + 1);
            } else {
                navigation_tx.send(0);
            }
        }
        EncoderEvent::Counterclockwise => {
            if navigation_location > 0 {
                navigation_tx.send(navigation_location - 1);
            } else {
                navigation_tx.send(FOOTER_ITEM_COUNT - 1);
            }
        }
        EncoderEvent::Click => {
            if navigation_location < FOOTER_ITEM_COUNT {
                footer_click_handler(navigation_location);
            }
        }
    }
}

async fn clocked_handler(encoder_event: EncoderEvent, clock: &'static SharedClock) {
    let _page_tx = PAGE.sender();
    let navigation_tx = NAVIGATION_LOCATION.sender();
    let mut navigation_rx = NAVIGATION_LOCATION.receiver().unwrap();

    let navigation_location = navigation_rx.get().await;

    let is_editing = IS_EDITING.load(Ordering::Relaxed);

    match encoder_event {
        EncoderEvent::Clockwise => match is_editing {
            false => {
                if navigation_location < FOOTER_ITEM_COUNT + CLOCK_PARAM_COUNT {
                    navigation_tx.send(navigation_location + 1);
                } else {
                    navigation_tx.send(0);
                }
            }
            true => {
                clock.lock(|c: &RefCell<Clock>| {
                    let mut clock = c.borrow_mut();

                    match navigation_location {
                        PER_MEASURE => {
                            let _ = clock.increment_time_signature_per_measure();
                        }
                        BASE => {
                            let _ = clock.increment_time_signature_base();
                        }
                        BPM => {
                            let _ = clock.increment_bpm();
                        }
                        DIVISION => {
                            let _ = clock.increment_division();
                        }
                        _ => {}
                    };
                });
            }
        },
        EncoderEvent::Counterclockwise => match is_editing {
            false => {
                if navigation_location > 0 {
                    navigation_tx.send(navigation_location - 1);
                } else {
                    navigation_tx.send(FOOTER_ITEM_COUNT + CLOCK_PARAM_COUNT - 1);
                }
            }
            true => {
                clock.lock(|c: &RefCell<Clock>| {
                    let mut clock = c.borrow_mut();

                    match navigation_location {
                        PER_MEASURE => {
                            let _ = clock.decrement_time_signature_per_measure();
                        }
                        BASE => {
                            let _ = clock.decrement_time_signature_base();
                        }
                        BPM => {
                            let _ = clock.decrement_bpm();
                        }
                        DIVISION => {
                            let _ = clock.decrement_division();
                        }
                        _ => {}
                    };
                });
            }
        },
        EncoderEvent::Click => {
            if navigation_location < FOOTER_ITEM_COUNT {
                footer_click_handler(navigation_location);
            } else {
                IS_EDITING.store(!is_editing, Ordering::Relaxed);
            }
        }
    }
}
