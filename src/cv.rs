use crate::{RUNNING, SharedAPIToCV, SharedClock, SharedPwmOutputs};
use alloc::vec::Vec;
use core::{
    cell::RefCell,
    sync::atomic::{AtomicBool, AtomicU16, Ordering},
};
use defmt::{info, warn};
use dsp_core::controls::{
    api_to_cv::{self, APIToCV, APIToCVModes, OUTPUT_COUNT},
    clock::Clock,
};
use embassy_rp::pwm::{Pwm, SetDutyCycle};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::{Duration, Ticker, Timer};

pub static TICK_MS: AtomicU16 = AtomicU16::new(0);
pub static TICK_CHANGED: AtomicBool = AtomicBool::new(false);

const TICK_CHECK_TIMEOUT: u64 = 500;
const ZERO_CV: [f32; OUTPUT_COUNT] = [0.0; OUTPUT_COUNT];

#[derive(Debug)]
pub struct CVMessage {
    pub input_index: usize,
    pub value: f32,
}

pub static OSC_TO_CV_BUFFER: Channel<CriticalSectionRawMutex, CVMessage, 16> = Channel::new();

const PRECISION_MULT: u16 = 10_000;
fn set_pwm_duty_cycles(pwm_outputs: &'static SharedPwmOutputs, values: [f32; OUTPUT_COUNT]) {
    pwm_outputs.lock(|pwm: &RefCell<[Pwm<'static>; 4]>| {
        pwm.borrow_mut()
            .iter_mut()
            .enumerate()
            .for_each(|(i, pwm_output)| {
                // multiply by 10k for 4 sig figs instead of 2
                let num = (values[i] * PRECISION_MULT as f32) as u16;
                let denom = PRECISION_MULT;
                info!("setting duty cycle {:?}/{:?}", num, denom);
                pwm_output.set_duty_cycle_fraction(num, denom).ok();
            });
    });
}

pub async fn trigger(pwm_outputs: &'static SharedPwmOutputs, output_values: [f32; OUTPUT_COUNT]) {
    set_pwm_duty_cycles(pwm_outputs, output_values);
    info!("real-time trigger sent");
    // 5ms trigger
    // TODO: add settings for gate
    Timer::after_millis(5).await;
    set_pwm_duty_cycles(pwm_outputs, ZERO_CV);
}

#[embassy_executor::task]
pub async fn cv_message_handler(
    pwm_outputs: &'static SharedPwmOutputs,
    api_to_cv: &'static SharedAPIToCV,
) {
    let receiver = OSC_TO_CV_BUFFER.receiver();

    loop {
        let message = receiver.receive().await;
        if RUNNING.load(Ordering::Relaxed) {
            let value = message.value.clamp(-1.0, 1.0);

            if let Some(output_values) = api_to_cv.lock(|a: &RefCell<APIToCV>| {
                let mut api_to_cv = a.borrow_mut();
                api_to_cv.receive_input(message.input_index, value);
                match api_to_cv.get_mode() {
                    APIToCVModes::RealTime => Some(api_to_cv.realtime_output(message.input_index)),
                    APIToCVModes::Clocked => None,
                }
            }) {
                trigger(pwm_outputs, output_values).await;
                info!("sent real-time trigger event");
            } else {
                info!("input buffered for clocked output");
            }
        }
    }
}

#[embassy_executor::task]
pub async fn clock_change_handler(clock: &'static SharedClock) {
    loop {
        clock.lock(|c: &RefCell<Clock>| {
            let incoming = c.borrow().get_tick_ms();
            let current = TICK_MS.load(Ordering::Relaxed);
            if incoming != current {
                TICK_MS.store(incoming, Ordering::Relaxed);
                TICK_CHANGED.store(true, Ordering::Relaxed);
            }
        });
        Timer::after_millis(TICK_CHECK_TIMEOUT).await;
    }
}

#[embassy_executor::task]
pub async fn clocked_cv_output_handler(
    pwm_outputs: &'static SharedPwmOutputs,
    api_to_cv: &'static SharedAPIToCV,
) {
    let mut ticker = Ticker::every(Duration::from_millis(TICK_MS.load(Ordering::Relaxed) as u64));

    loop {
        let output_values = api_to_cv.lock(|a: &RefCell<APIToCV>| a.borrow_mut().tick());
        trigger(pwm_outputs, output_values).await;
        info!("sent clocked trigger event");

        if TICK_CHANGED.load(Ordering::Relaxed) {
            ticker = Ticker::every(Duration::from_millis(TICK_MS.load(Ordering::Relaxed) as u64));
        }

        ticker.next().await;
    }
    // TODO: call tick
    // TODO: set_pwm_duty_cycles(pwm_outputs, output_values);
}
