#[macro_use]
extern crate serde_derive;
extern crate bincode;

extern crate rand;

use bincode::{deserialize, serialize};
use rand::distributions::{Bernoulli, Distribution};
use std::sync::mpsc;

#[derive(Serialize, Deserialize, Debug)]
struct Packet {
    to: u8,
    from: u8,
    id: u8,
    len: u8,
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

    fn transmit(&mut self, s: String) -> Result<(), String> {
        let p = Packet {
            to: 1,
            from: 0,
            id: self.frame_id,
            len: 0,
            payload: s,
            checksum: 0,
        };

        if let Ok(mut encoded_p) = serialize(&p) {
            encoded_p[PACKET_LEN_IDX] = encoded_p.len() as u8 - 5; // whole length must be < 256
                                                                   //TODO: break s into blocks and packetize and blocks individually

            let len = encoded_p.len();
            encoded_p[len - 1] = encoded_p.iter().fold(0, |acc, x| acc ^ x);

            if let Err(_) = self.tx.send(encoded_p) {
                return Err(String::from("failed to send"));
            }

            self.frame_id += 1;

            Ok(())
        } else {
            Err(String::from("failed to serialize"))
        }
    }
}

fn main() {
    let (tx, rx) = mpsc::channel();
    let mut radio = Radio::new(tx);

    std::thread::spawn(move || {
        for received in rx {
            if received.iter().fold(0, |acc, x| acc ^ x) == 0 {
                if let Ok(decoded_p) = deserialize::<Packet>(&received) {
                    println!("{:?}", received);
                    println!("{:?}", decoded_p);
                }
            } else {
                println!("Error in received message");
                println!("{:?}", received);
            }
        }
    });

    radio.transmit("why not".to_string()).expect("failed tx");
}
