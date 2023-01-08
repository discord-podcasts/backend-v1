use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket},
    ops::Range,
    sync::Arc,
};

use crate::App;

static PORT_RANGE: Range<u16> = 42000..42100;

pub struct AudioServer {
    socket: Arc<UdpSocket>,
    pub port: u16,
    pub host_address: Option<SocketAddr>, // UDP socket address of the host
}

impl AudioServer {
    pub fn create(app: Arc<App>) -> Option<AudioServer> {
        let used_ports = app.podcasts(|sessions| {
            sessions
                .values()
                .map(|podcast| podcast.audio_server.socket.local_addr())
                .filter(|address| address.is_ok())
                .map(|address| address.unwrap().port())
                .collect::<Vec<u16>>()
        });
        let free_ports = PORT_RANGE
            .clone()
            .filter(|port| !used_ports.contains(port))
            .collect::<Vec<u16>>();

        for port in free_ports {
            let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), port);
            let server = UdpSocket::bind(address);
            match server {
                Ok(socket) => {
                    let port = socket.local_addr().unwrap().port();
                    println!("Created audio server at 127.0.0.1:{}", port);

                    let audio_server = AudioServer {
                        socket: Arc::new(socket),
                        port,
                        host_address: None,
                    };
                    return Some(audio_server);
                }
                Err(_) => continue,
            }
        }

        None
    }
}
