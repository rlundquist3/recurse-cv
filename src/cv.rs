use alloc::vec::Vec;
use defmt::{info, warn};
use dsp_core::controls::api_to_cv::APIToCV;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

#[derive(Debug)]
pub struct CVMessage {
    pub input_index: usize,
    pub value: f32,
}

pub static OSC_TO_CV_BUFFER: Channel<CriticalSectionRawMutex, CVMessage, 16> = Channel::new();

#[embassy_executor::task]
pub async fn cv_output_handler() {
    let receiver = OSC_TO_CV_BUFFER.receiver();

    let mut api_to_cv = APIToCV::new();

    loop {
        let message = receiver.receive().await;

        let value = message.value.clamp(-1.0, 1.0);
        api_to_cv.receive_input(message.input_index, value);
        info!("sent to the thing");
    }

    // TODO: output cv to pins
}
