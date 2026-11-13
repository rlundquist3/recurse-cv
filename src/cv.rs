use crate::SharedPwmOutputs;
use alloc::vec::Vec;
use core::cell::RefCell;
use defmt::{info, warn};
use dsp_core::controls::api_to_cv::{APIToCV, APIToCVModes, OUTPUT_COUNT};
use embassy_rp::pwm::{Pwm, SetDutyCycle};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};
use embassy_time::Timer;

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

#[embassy_executor::task]
pub async fn cv_message_handler(pwm_outputs: &'static SharedPwmOutputs) {
    let receiver = OSC_TO_CV_BUFFER.receiver();

    let mut api_to_cv = APIToCV::new();

    loop {
        let message = receiver.receive().await;

        let value = message.value.clamp(-1.0, 1.0);
        api_to_cv.receive_input(message.input_index, value);
        info!("sent to the thing");
        match api_to_cv.get_mode() {
            APIToCVModes::RealTime => {
                let output_values = api_to_cv.realtime_output(message.input_index);
                set_pwm_duty_cycles(pwm_outputs, output_values);

                // 5ms trigger
                // TODO: add settings for gate
                Timer::after_millis(5).await;
                set_pwm_duty_cycles(pwm_outputs, ZERO_CV);
            }
            APIToCVModes::Clocked => {
                info!("input buffered for clocked output");
            }
        };
    }
}

#[embassy_executor::task]
pub async fn clocked_cv_output_handler(pwm_outputs: &'static SharedPwmOutputs) {
    // TODO: call tick
    // TODO: set_pwm_duty_cycles(pwm_outputs, output_values);
}
