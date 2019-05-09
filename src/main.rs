#[macro_use]
extern crate serde_derive;
extern crate bincode;

extern crate rand;

use bincode::{deserialize, serialize};
use itertools::Itertools;
use rand::distributions::{Distribution, Uniform};
use std::collections::VecDeque;
use std::sync::mpsc;

use portaudio as pa;
use std::f64::consts::PI;

const CHANNELS: i32 = 2;
const SAMPLE_RATE: f64 = 10000.0;
const FRAMES_PER_BUFFER: u32 = 64;
const TABLE_SIZE: usize = 10000;
const SEND_INTERVAL_MS: u64 = 10;

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
    tx: mpsc::Sender<Vec<u8>>,
    frame_id: u8,
}

impl Radio {
    fn new(tx: mpsc::Sender<Vec<u8>>) -> Radio {
        Radio {
            tx: tx,
            frame_id: 0,
        }
    }

    fn packetize(&mut self, s: String) -> Option<Vec<Packet>> {
        let mut packets = Vec::<Packet>::new();
        for chunk in &s.chars().chunks(PACKET_MAX_LEN) {
            packets.push(Packet {
                to: 1,
                from: 0,
                id: self.frame_id,
                payload_len: 0, // set in transmit, after encoded
                payload: chunk.collect::<String>(),
                checksum: 0, // set in transmit, after encoded
            });
            self.frame_id += 1;
        }
        Some(packets)
    }

    fn transmit(&mut self, s: String) -> Result<(), ()> {
        if let Some(packets) = self.packetize(s) {
            let encoded_packets = packets
                .iter()
                .map(|p| serialize(p))
                .map(|encoded_p_res| {
                    // set payload len
                    encoded_p_res.map(|mut encoded_p| {
                        encoded_p[PACKET_LEN_IDX] = (encoded_p.len() - 5) as u8;
                        encoded_p
                    })
                })
                .map(|encoded_p_res| {
                    // set checksum
                    encoded_p_res.map(|mut encoded_p| {
                        let len = encoded_p.len();
                        encoded_p[len - 1] = encoded_p.iter().fold(0, |acc, x| acc ^ x);
                        // println!("checksum: {}", encoded_p[len -1]);
                        encoded_p
                    })
                })
                .map(|encoded_p_res| {
                    encoded_p_res.map(|encoded_p| {
                        encoded_p
                            .iter()
                            .map(|&byte| unpack_byte(byte))
                            .collect::<Vec<[u8; 8]>>()
                    })
                })
                .map(|encoded_p_res| encoded_p_res.map(|encoded_p| flatten(encoded_p)));
            for encoded_packet_res in encoded_packets {
                if let Ok(encoded_packet) = encoded_packet_res {
                    if let Err(_) = self.tx.send(encoded_packet) {
                        return Err(());
                    }
                }
            }
            Ok(())
        } else {
            Err(())
        }
    }
}

fn main() {
    let (tx_radio1, rx_medium) = mpsc::channel();
    let (tx_medium, rx_radio2) = mpsc::channel::<u8>();
    let mut radio = Radio::new(tx_radio1);

    let preamble: [u8; PREAMBLE_LEN] = {
        let mut preamble: [u8; PREAMBLE_LEN] = [0; PREAMBLE_LEN];
        let d = Uniform::new_inclusive(0u8, 1);
        for i in 0..preamble.len() {
            preamble[i] = d.sample(&mut rand::thread_rng());
        }
        preamble
    };

    let medium_thread = std::thread::spawn(move || {
        let note_0 = 440.0;
        let note_1 = 560.0;
        
        let mut sine_0 = [0.0; TABLE_SIZE];
        let mut sine_1 = [0.0; TABLE_SIZE];
        for i in 0..TABLE_SIZE {
            sine_0[i] = (i as f64 / TABLE_SIZE as f64 * PI * 2.0 * note_0 as f64).sin() as f32;
            sine_1[i] = (i as f64 / TABLE_SIZE as f64 * PI * 2.0 * note_1 as f64).sin() as f32;
        }
        let sine_0 = sine_0;
        let sine_1 = sine_1;

        let mut phase = 0;
        let pa = pa::PortAudio::new().expect("faillll on port audio");
        let settings = pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER)
            .expect("fail on settings");

        let sines = [sine_0, sine_1];
        let mut sines_idx = 0;

        let (tx_note, rx_note) = mpsc::channel();

        let mut last_note_idx = 0;
        let cb = move |pa::OutputStreamCallbackArgs {buffer, frames, ..}| {
            
            last_note_idx = if let Ok(sines_idx) = rx_note.try_recv() {
                sines_idx
            } else {
                last_note_idx
            };

            let mut idx = 0;
            for _ in 0..frames {
                buffer[idx] = sines[last_note_idx][phase];
                buffer[idx+1] = sines[last_note_idx][phase];
                phase += 1;
                if phase >= TABLE_SIZE {
                    phase -= TABLE_SIZE;
                }
                idx += 2;
            }
            pa::Continue
        };

        let mut stream = pa.open_non_blocking_stream(settings, cb).expect("faill on stream");
        stream.start();

        #[derive(PartialEq)]
        enum TxState {
            Random,
            Preamble,
            Packet,
        }
        let distr = Uniform::new_inclusive(0u8, 1);
        let mut tx_state = TxState::Random;
        let mut counter = 0;
        let mut buf: VecDeque<Vec<u8>> = VecDeque::new();
        let mut curr_packet: Option<Vec<u8>> = None;
        loop {
            tx_note.send(sines_idx);
            while let Ok(packet) = rx_medium.try_recv() {
                buf.push_back(packet);
            }

            if tx_state == TxState::Random {
                if let Some(packet) = buf.pop_front() {
                    curr_packet = Some(packet);
                    tx_state = TxState::Preamble;
                    counter = 0;
                }
            }
            match tx_state {
                TxState::Random => {
                    let num = distr.sample(&mut rand::thread_rng());
                    sines_idx = num as usize;
                    tx_medium
                        .send(num)
                        .unwrap_or(());
                }
                TxState::Preamble => {
                    sines_idx = preamble[counter] as usize;
                    while tx_medium.send(preamble[counter]).is_err() {}
                    counter += 1;
                    if counter >= preamble.len() {
                        counter = 0;
                        tx_state = TxState::Packet;
                    }
                }
                TxState::Packet => {
                    if let Some(ref packet) = curr_packet {
                        sines_idx = (*packet)[counter] as usize;
                        while tx_medium.send((*packet)[counter]).is_err() {}
                        counter += 1;
                        if counter >= packet.len() {
                            counter = 0;
                            tx_state = TxState::Random;
                        }
                    } else {
                        tx_state = TxState::Random;
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(SEND_INTERVAL_MS));
        }
    });

    let rx_thread = std::thread::spawn(move || {
        enum RxState {
            PreambleCheck,
            Packet,
        }
        let mut byte_buf: [u8; 8] = [0; 8];
        let mut buf: [u8; 2048] = [0; 2048];
        let mut curr_buf_idx = 0;
        let mut curr_byte_idx = 0;
        let mut rx_state = RxState::PreambleCheck;
        for received in rx_radio2 {
            match rx_state {
                RxState::PreambleCheck => {
                    curr_buf_idx = if received == preamble[curr_buf_idx] {
                        curr_buf_idx + 1
                    } else {
                        0
                    };
                    if curr_buf_idx == preamble.len() {
                        rx_state = RxState::Packet;
                        curr_buf_idx = 0;
                        curr_byte_idx = 0;
                    }
                }
                RxState::Packet => {
                    byte_buf[curr_byte_idx] = received;
                    curr_byte_idx += 1;
                    if curr_byte_idx == 8 {
                        buf[curr_buf_idx] = pack_byte(byte_buf);
                        curr_buf_idx += 1;
                        curr_byte_idx = 0;
                    }
                    if curr_buf_idx > PACKET_LEN_IDX
                        && curr_buf_idx >= (buf[PACKET_LEN_IDX] as usize) + 5
                    {
                        if buf.iter().take(curr_buf_idx).fold(0, |acc, x| acc ^ x) == 0 {
                            if let Ok(decoded_packet) = deserialize::<Packet>(&buf[0..curr_buf_idx])
                            {
                                println!("{:?}", decoded_packet);
                            } else {
                                println!("failed to deserialize");
                            }
                        } else {
                            println!("checksum failed");
                        }
                        rx_state = RxState::PreambleCheck;
                        curr_buf_idx = 0;
                        curr_byte_idx = 0;
                    }
                }
            }
        }
    });

    radio
        .transmit("transmissions begin now".to_string())
        .expect("failed tx");

    radio
        .transmit(
            "abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890
    abcdefghijklmnopqrstuvwxyz1234567890 "
                .to_string(),
        )
        .expect("failed tx");

    radio
        .transmit("transmissions end now".to_string())
        .expect("failed tx");

    std::thread::sleep(std::time::Duration::from_millis(1000));
    rx_thread.join().expect("failed to join rx thread");
    medium_thread.join().expect("failed to join medium thread");
}

fn unpack_byte(input: u8) -> [u8; 8] {
    [
        (input >> 0) & 1,
        (input >> 1) & 1,
        (input >> 2) & 1,
        (input >> 3) & 1,
        (input >> 4) & 1,
        (input >> 5) & 1,
        (input >> 6) & 1,
        (input >> 7) & 1,
    ]
}

fn pack_byte(arr: [u8; 8]) -> u8 {
    (arr[0] << 0)
        | (arr[1] << 1)
        | (arr[2] << 2)
        | (arr[3] << 3)
        | (arr[4] << 4)
        | (arr[5] << 5)
        | (arr[6] << 6)
        | (arr[7] << 7)
}

fn flatten(vec: Vec<[u8; 8]>) -> Vec<u8> {
    let mut out = Vec::with_capacity(vec.len() * 8);
    for arr in vec.iter() {
        for &byte in arr.iter() {
            out.push(byte);
        }
    }
    out
}
