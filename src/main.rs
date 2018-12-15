#[macro_use]
extern crate serde_derive;
extern crate bincode;

extern crate rand;

use bincode::{deserialize, serialize};
use itertools::Itertools;
use rand::distributions::{Bernoulli, Distribution};
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
        for chunk in &s.chars().chunks(63) {
            // floor(255/4) = 63 where max utf-8 char len is 4 bytes and 255 is max number representable by u8
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
                .map(|encoded_p_res| { // set payload len
                    encoded_p_res.map(|mut encoded_p| {
                        encoded_p[PACKET_LEN_IDX] = (encoded_p.len() - 5) as u8;
                        encoded_p
                    })
                })
                .map(|encoded_p_res| { // set checksum
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
    let (tx, rx) = mpsc::channel();
    let mut radio = Radio::new(tx);

    let rx_thread = std::thread::spawn(move || {
        for received in rx {
            if received.iter().fold(0, |acc, x| acc ^ x) == 0 {
                if let Ok(decoded_p) = deserialize::<Packet>(&received) {
                    // println!("{:?}", received);
                    println!("{:?}", decoded_p);
                } else {
                    println!("failed to deserialize");
                }
            } else {
                println!("Error in received message");
                println!("{:?}", received);
            }
        }
    });

    radio.transmit("abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890 abcdefghijklmnopqrstuvwxyz1234567890".to_string()).expect("failed tx");
    rx_thread.join().expect("failed to join rx thread");
}
