#[macro_use]
extern crate serde_derive;
extern crate bincode;

extern crate rand;

use bincode::{deserialize, serialize};
use itertools::Itertools;
use rand::distributions::{Bernoulli, Distribution, Uniform};
use std::collections::VecDeque;
use std::sync::mpsc;

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
                        encoded_p
                    })
                });
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

    let preamble: [u8; 256] = {
        let mut preamble: [u8; 256] = [0; 256];
        let d = Uniform::new_inclusive(0u8, 255);
        for i in 0..preamble.len() {
            preamble[i] = d.sample(&mut rand::thread_rng());
        }
        preamble
    };

    let medium_thread = std::thread::spawn(move || {
        #[derive(PartialEq)]
        enum TxState {
            Random,
            Preamble,
            Packet,
        }
        let distr = Uniform::new_inclusive(0u8, 255);
        let mut tx_state = TxState::Random;
        let mut counter = 0;
        let mut buf: VecDeque<Vec<u8>> = VecDeque::new();
        let mut curr_packet: Option<Vec<u8>> = None;

        loop {
            // push new received packets to buf
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
                    tx_medium
                        .send(distr.sample(&mut rand::thread_rng()))
                        .unwrap_or(());
                }
                TxState::Preamble => {
                    while tx_medium.send(preamble[counter]).is_err() {}
                    counter += 1;
                    if counter >= preamble.len() {
                        counter = 0;
                        tx_state = TxState::Packet;
                    }
                }
                TxState::Packet => {
                    if let Some(ref packet) = curr_packet {
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
        }
    });

    let rx_thread = std::thread::spawn(move || {
        enum RxState {
            PreambleCheck,
            Packet,
        }
        let mut buf: [u8; 2048] = [0; 2048];
        let mut curr_idx = 0;
        let mut rx_state = RxState::PreambleCheck;

        for received in rx_radio2 {
            match rx_state {
                RxState::PreambleCheck => {
                    curr_idx = if received == preamble[curr_idx] {
                        curr_idx + 1
                    } else {
                        0
                    };
                    if curr_idx == preamble.len() {
                        rx_state = RxState::Packet;
                        curr_idx = 0;
                    }
                }
                RxState::Packet => {
                    buf[curr_idx] = received;
                    curr_idx += 1;
                    if curr_idx > PACKET_LEN_IDX
                        && curr_idx >= (buf[PACKET_LEN_IDX] as usize) + 5 + 1
                    {
                        if buf.iter().take(curr_idx).fold(0, |acc, x| acc ^ x) == 0 {
                            if let Ok(decoded_packet) = deserialize::<Packet>(&buf[0..curr_idx]) {
                                println!("{:?}", decoded_packet);
                            } else {
                                println!("failed to deserialize");
                            }
                        } else {
                            println!("checksum failed");
                        }
                        rx_state = RxState::PreambleCheck;
                    }
                }
            }
            // if received.iter().fold(0, |acc, x| acc ^ x) == 0 {
            //     if let Ok(decoded_p) = deserialize::<Packet>(&received) {
            //         // println!("{:?}", received);
            //         println!("{:?}", decoded_p);
            //     } else {
            //         println!("failed to deserialize");
            //     }
            // } else {
            //     println!("Error in received message");
            //     println!("{:?}", received);
            // }
        }
    });

    radio.transmit("abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890".to_string()).expect("failed tx");
    rx_thread.join().expect("failed to join rx thread");
    medium_thread.join().expect("failed to join medium thread");
}
