use core::str::from_utf8;
use defmt::{error, info, warn};
use embassy_net::{
    Stack,
    dns::{self, DnsQueryType},
    tcp::{ConnectError, TcpReader, TcpSocket},
};
use embassy_time::{Duration, Timer};

struct Buffers {
    tcp_rx: [u8; 4096],
    tcp_tx: [u8; 4096],
    tls_rx: [u8; 16640],
    tls_tx: [u8; 4096],
    buf: [u8; 2048], // TODO: probably can be way smaller
}

pub async fn listen(stack: Stack<'static>) {
    let mut buffers = Buffers {
        tcp_rx: [0; 4096],
        tcp_tx: [0; 4096],
        tls_rx: [0; 16640],
        tls_tx: [0; 4096],
        buf: [0; 2048],
    };

    loop {
        let mut socket = TcpSocket::new(stack, &mut buffers.tcp_rx, &mut buffers.tcp_tx);
        socket.set_timeout(Some(Duration::from_secs(10)));

        info!("Listening on TCP:1234...");
        if let Err(e) = socket.accept(1234).await {
            warn!("accept error: {:?}", e);
            continue;
        }

        loop {
            let n = match socket.read(&mut buffers.buf).await {
                Ok(0) => {
                    warn!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(err) => {
                    warn!("read error: {:?}", err);
                    break;
                }
            };

            info!("received {}", &buffers.buf[..n]);
        }
    }
}
