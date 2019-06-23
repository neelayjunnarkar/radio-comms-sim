#[macro_use]
extern crate serde_derive;
extern crate bincode;

#[macro_use]
extern crate lazy_static;

use std::sync::{mpsc, Mutex};

mod audio;
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

const PACKET_LEN_IDX: usize = 3;
const PACKET_MAX_LEN: usize = 63; // floor(255/4) = 63 where max utf-8 char len is 4 bytes and 255 is max number representable by u8

const SEND_INTERVAL_MS: u64 = 10;

const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 10000.0;
const FRAMES_PER_BUFFER: u32 = 64;
const TABLE_SIZE: usize = 10000;

const PREAMBLE: [u8; 256] = [1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 1, 0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 0, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 1];

struct Radio {
    frame_id: u8,
    tx: mpsc::Sender<Vec<u8>>,
    audio_join_handle: std::thread::JoinHandle<Result<(), String>>,
}

lazy_static! {
    static ref RADIO: Mutex<Option<Radio>> = Mutex::new(None);
}

pub fn start() -> Result<(), String> {
    if let Ok(mut radio_guard) = RADIO.lock() {
        if (*radio_guard).is_none() {
            let (tx, rx) = mpsc::channel();
            let audio_thread = std::thread::spawn(move || audio::start(rx));
            *radio_guard = Some(Radio {
                frame_id: 0,
                tx: tx,
                audio_join_handle: audio_thread,
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
