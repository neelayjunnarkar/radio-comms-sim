#[macro_use]
extern crate serde_derive;
extern crate bincode;

#[macro_use]
extern crate lazy_static;

use std::sync::{mpsc, Mutex};

mod tx;

#[derive(Serialize, Deserialize, Debug)]
struct Packet {
    to: u8,
    from: u8,
    id: u8,
    payload_len: u8,
    payload: String,
    checksum: u8,
}

const PREAMBLE_LEN: usize = 256;
const PACKET_LEN_IDX: usize = 3;
const PACKET_MAX_LEN: usize = 63; // floor(255/4) = 63 where max utf-8 char len is 4 bytes and 255 is max number representable by u8

struct Radio {
    frame_id: u8,
    tx: mpsc::Sender<Vec<u8>>,
}

lazy_static! {
    static ref RADIO: Mutex<Option<Radio>> = Mutex::new(None);
}

pub fn start() -> Result<(), String> {
    if let Ok(mut radio_guard) = RADIO.lock() {
        if (*radio_guard).is_none() {
            let (tx, rx) = mpsc::channel();
            *radio_guard = Some(Radio {
                frame_id: 0,
                tx: tx,
            });
            Ok(())
        } else {
            Err("Error: radio already initialized".to_owned())
        }
    } else {
        Err("Error: on lock radio mutex".to_owned())
    }
}

pub fn transmit(s: String, to: u8) -> Result<(), String> {
    tx::transmit(s, to)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
