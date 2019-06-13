use itertools::Itertools;
use super::{Packet, FRAME_ID, PACKET_MAX_LEN};

fn packetize(s: String) -> Vec<Packet> {
    let mut packets = Vec::<Packet>::new();
    for chunk in &s.chars().chunks(PACKET_MAX_LEN) {
        let mut frame_id = FRAME_ID.lock().unwrap();

        packets.push(Packet {
            to: 1,
            from: 0,
            id: *frame_id,
            payload_len: 0, // set in transmit, after encoded
            payload: chunk.collect::<String>(),
            checksum: 0, // set in transmit, after encoded
        });
        *frame_id += 1;
    }
    packets
}

pub fn transmit(s: String) -> Result<(), ()> {
    Err(())
}
