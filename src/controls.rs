use defmt::info;
use embassy_rp::gpio::Input;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Instant, Timer};

const DEBOUNCE: Duration = Duration::from_millis(250);

pub static CONTROL_BUFFER: Channel<CriticalSectionRawMutex, EncoderEvent, 16> = Channel::new();

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
pub async fn control_handler() {
    let buffer_rx = CONTROL_BUFFER.receiver();

    loop {
        let event = buffer_rx.receive().await;
        match event {
            EncoderEvent::Clockwise => info!("clockwise"),
            EncoderEvent::Counterclockwise => info!("counterclockwise"),
            EncoderEvent::Click => info!("click"),
        }
    }
}
