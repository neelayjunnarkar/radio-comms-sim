use super::{Packet, PACKET_MAX_LEN, RADIO};
use bincode::serialize;
use itertools::Itertools;

fn packetize(s: String, to: u8) -> Result<Vec<Packet>, String> {
    if let Ok(mut radio_guard) = RADIO.lock() {
        if let Some(ref mut radio) = *radio_guard {
            let mut packets = Vec::<Packet>::new();
            for chunk in &s.chars().chunks(PACKET_MAX_LEN) {
                let payload = chunk.collect::<String>();

                if let Ok(vec) = serialize(&payload) {
                    packets.push(Packet {
                        to: 1,
                        from: 0,
                        id: (*radio).frame_id,
                        payload_len: vec.len() as u8,
                        payload: payload,
                        checksum: 0, // set in transmit, after encoded
                    });
                    (*radio).frame_id += 1;
                }
            }
            Ok(packets)
        } else {
            Err("Error: radio not initialized".to_owned())
        }
    } else {
        Err("Error: failed to lock radio mutex".to_owned())
    }
}

pub fn transmit(s: String, to: u8) -> Result<(), ()> {
    let packets = packetize(s, to);

    Err(())
}
