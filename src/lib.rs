#[macro_use]
extern crate serde_derive;
extern crate bincode;

#[macro_use]
extern crate lazy_static;

use std::sync::{mpsc, Mutex};

mod audio;
mod tx;
mod rx;

#[derive(Serialize, Deserialize, Debug)]
struct Packet {
    to: u8,
    from: u8,
    id: u8,
    payload_len: u8,
    payload: String,
    checksum: u8,
}

const IN_BUF_LEN: usize = 256 * 4;

const PACKET_LEN_IDX: usize = 3;
const PACKET_MAX_LEN: usize = 63; // floor(255/4) = 63 where max utf-8 char len is 4 bytes and 255 is max number representable by u8

const SEND_INTERVAL_MS: u64 = 10;

const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 10000.0;
const FRAMES_PER_BUFFER: u32 = 256;
const TABLE_SIZE: usize = 10000;
const INTERLEAVED: bool = true;

const PREAMBLE: [u8; 256] = [
    1, 0, 1, 0, 1, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 0, 0, 1, 0, 0, 1, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 1,
    0, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1,
    0, 0, 1, 1, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1,
    1, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 1, 0, 1,
    0, 0, 0, 1, 1, 1, 0, 1, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 0, 0, 1, 1, 1, 0, 0, 1, 0, 1, 0, 0,
    1, 1, 0, 0, 0, 1, 1, 1, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 0, 1, 1, 0, 1, 1, 0, 1,
    1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 1, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 1, 1, 0, 0, 0, 1, 1,
    0, 0, 0, 0, 1, 1, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 1,
];

struct Radio {
    frame_id: u8,
    audio_out_tx: mpsc::Sender<Vec<u8>>,
    packet_rx: mpsc::Receiver<String>,
    audio_join_handle: std::thread::JoinHandle<Result<(), String>>,
    receive_join_handle: std::thread::JoinHandle<Result<(), String>>,
    audio_in_buf: [f32; IN_BUF_LEN],
    in_buf_next: usize,
}

lazy_static! {
    static ref RADIO: Mutex<Option<Radio>> = Mutex::new(None);
}

pub fn start() -> Result<(), String> {
    let mut radio_guard = RADIO.lock().expect("Failed to acquire lock on radio");
    if (*radio_guard).is_none() {
        let (audio_out_tx, audio_out_rx) = mpsc::channel();
        let (audio_in_tx, audio_in_rx) = mpsc::channel();
        let (packet_tx, packet_rx) = mpsc::channel();
        let audio_thread = std::thread::spawn(move || audio::start(audio_out_rx, audio_in_tx));
        let receive_thread = std::thread::spawn(move || rx::start(audio_in_rx, packet_tx));
        *radio_guard = Some(Radio {
            frame_id: 0,
            audio_out_tx: audio_out_tx,
            packet_rx: packet_rx,
            audio_join_handle: audio_thread,
            receive_join_handle: receive_thread,
            audio_in_buf: [0.0; 1024],
            in_buf_next: 0,
        });
        Ok(())
    } else {
        Err("Error: radio already initialized".to_owned())
    }
}

pub fn transmit(s: String, to: u8) -> Result<(), String> {
    tx::transmit(s, to)
}

pub fn receive() -> Option<String> {
    rx::receive()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
