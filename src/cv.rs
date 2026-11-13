use alloc::vec::Vec;
use defmt::{info, warn};
use dsp_core::controls::api_to_cv::{APIToCV, APIToCVModes, OUTPUT_COUNT};
use embassy_rp::pwm::{Pwm, SetDutyCycle};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

#[derive(Debug)]
pub struct CVMessage {
    pub input_index: usize,
    pub value: f32,
}

pub static OSC_TO_CV_BUFFER: Channel<CriticalSectionRawMutex, CVMessage, 16> = Channel::new();

fn set_pwm_duty_cycles(
    mut pwm_outputs: &mut [Pwm<'static>; OUTPUT_COUNT],
    values: [f32; OUTPUT_COUNT],
) {
    pwm_outputs
        .iter_mut()
        .enumerate()
        .for_each(|(i, pwm_output)| {
            // v * 100 / 10_000 for 4-digit percentage
            let num = (values[i] * 100.0) as u16;
            let denom = 10_000;
            pwm_output.set_duty_cycle_fraction(num, denom).ok();
        });
}

#[embassy_executor::task]
pub async fn cv_message_handler(mut pwm_outputs: [Pwm<'static>; 4]) {
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
                set_pwm_duty_cycles(&mut pwm_outputs, output_values);
            }
            APIToCVModes::Clocked => {
                info!("input buffered for clocked output");
            }
        };
    }
}

#[embassy_executor::task]
pub async fn clocked_cv_output_handler() {
    // TODO: call tick
    // TODO: set_pwm_duty_cycles(pwm_outputs, output_values);
}
