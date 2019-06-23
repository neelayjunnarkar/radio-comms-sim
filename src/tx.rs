use super::{Packet, PACKET_LEN_IDX, PACKET_MAX_LEN, RADIO};
use bincode::serialize;
use itertools::Itertools;

fn packetize(s: String, to: u8, frame_id: &mut u8) -> Result<Vec<Packet>, String> {
    let mut packets = Vec::<Packet>::new();
    for chunk in &s.chars().chunks(PACKET_MAX_LEN) {
        let payload = chunk.collect::<String>();

        packets.push(Packet {
            to: to,
            from: 0,
            id: *frame_id,
            payload_len: 0, // set in transmit, after encoded
            payload: payload,
            checksum: 0, // set in transmit, after encoded
        });
        *frame_id += 1;
    }
    Ok(packets)
}

pub fn transmit(s: String, to: u8) -> Result<(), String> {
    if let Ok(mut radio_guard) = RADIO.lock() {
        if let Some(ref mut radio) = *radio_guard {
            if let Ok(packets) = packetize(s, to, &mut (*radio).frame_id) {
                println!("ok packets from packetization");
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
                        println!("sending packet to audio thread");
                        if let Err(_) = (*radio).tx.send(encoded_packet) {
                            return Err(
                                "Error: failed to send encoded packet across channel".to_owned()
                                );
                        }
                    }
                }
                Ok(())
            } else {
                Err("Error: failed to packetize string".to_owned())
            }
        } else {
            Err("Error: radio not initialized".to_owned())
        }
    } else {
        Err("Error: failed to lock radio mutex".to_owned())
    }
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

fn flatten(vec: Vec<[u8; 8]>) -> Vec<u8> {
    let mut out = Vec::with_capacity(vec.len() * 8);
    for arr in vec.iter() {
        for &byte in arr.iter() {
            out.push(byte);
        }
    }
    out
}
