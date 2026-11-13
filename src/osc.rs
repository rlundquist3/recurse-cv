use crate::cv::{CVMessage, OSC_TO_CV_BUFFER};
use byte::ctx::{Endian, NULL, Str};
use byte::*;
use defmt::{info, warn};
use embassy_net::{Stack, tcp::TcpSocket};
use embassy_time::Duration;
use serial_line_ip::Decoder;

struct Buffers {
    tcp_rx: [u8; 64],
    tcp_tx: [u8; 64],
    input: [u8; 64],
    slip_decoded: [u8; 64],
}

pub async fn listen(stack: Stack<'static>) {
    let mut buffers = Buffers {
        tcp_rx: [0; 64],
        tcp_tx: [0; 64],
        input: [0; 64],
        slip_decoded: [0; 64],
    };

    loop {
        let mut socket = TcpSocket::new(stack, &mut buffers.tcp_rx, &mut buffers.tcp_tx);
        socket.set_timeout(Some(Duration::from_secs(10)));

        let port = 5072;
        info!("Listening on TCP:{}...", port);
        if let Err(e) = socket.accept(port).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        let mut slip_decoder = Decoder::new();
        let mut decoded_count = 0;

        loop {
            let byte_count = match socket.read(&mut buffers.input).await {
                Ok(0) => {
                    warn!("read EOF");
                    break;
                }
                Ok(count) => count,
                Err(err) => {
                    warn!("read error: {:?}", err);
                    break;
                }
            };

            info!("received {}", &buffers.input[..byte_count]);
            let mut input_slice = &buffers.input[..byte_count];

            while !input_slice.is_empty() {
                let (processed_count, slip_decoded_slice, is_end) = match slip_decoder
                    .decode(input_slice, &mut buffers.slip_decoded[decoded_count..])
                {
                    Ok(res) => res,
                    Err(_) => {
                        warn!("SLIP decode error; dropping frame");
                        slip_decoder = Decoder::new();
                        decoded_count = 0;
                        break;
                    }
                };
                decoded_count += slip_decoded_slice.len();
                input_slice = &input_slice[processed_count..];

                if is_end {
                    let packet = &buffers.slip_decoded[..decoded_count];
                    match packet.read_with::<OSCMessage>(&mut 0, Endian::Big) {
                        Ok(message) => handle_osc_message(message),
                        Err(_) => warn!("error parsing OSC packet"),
                    }
                    slip_decoder = Decoder::new();
                    decoded_count = 0;
                } else if decoded_count == buffers.slip_decoded.len() {
                    warn!("SLIP packet too large; dropping");
                    slip_decoder = Decoder::new();
                    decoded_count = 0;
                }
            }
        }
    }
}

fn handle_osc_message(message: OSCMessage<'_>) {
    info!(
        "message {:?}, {:?}, {:?}",
        message.address, message.value_type, message.value
    );

    let mut parts = message.address.split('/');
    let input_index = match (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) {
        (Some(""), Some("cv"), Some(i), Some("value"), None) => match i.parse::<usize>() {
            Ok(i @ 1..=4) => i - 1,
            _ => return warn!("CV index {:?} out of range", i),
        },
        _ => return warn!("OSC address {:?} not supported", message.address),
    };
    if message.value_type != ",f" {
        return warn!("value of type {:?} not supported", message.value_type);
    }

    match OSC_TO_CV_BUFFER.try_send(CVMessage {
        input_index,
        value: message.value,
    }) {
        Ok(()) => {}
        Err(_) => warn!("OSC buffer full"),
    };
}

#[derive(Debug)]
pub struct OSCMessage<'a> {
    pub address: &'a str,
    pub value_type: &'a str,
    pub value: f32,
}

impl<'a> TryRead<'a, Endian> for OSCMessage<'a> {
    fn try_read(bytes: &'a [u8], endian: Endian) -> Result<(Self, usize)> {
        let offset = &mut 0;

        let address = bytes.read_with::<&str>(offset, Str::Delimiter(NULL))?;
        *offset = (*offset + 3) & !3;
        let value_type = bytes.read_with::<&str>(offset, Str::Delimiter(NULL))?;
        *offset = (*offset + 3) & !3;
        let value = bytes.read_with::<f32>(offset, endian)?;

        Ok((
            OSCMessage {
                address,
                value_type,
                value,
            },
            *offset,
        ))
    }
}
