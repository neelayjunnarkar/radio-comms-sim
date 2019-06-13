#[macro_use]
extern crate serde_derive;
extern crate bincode;

#[macro_use]
extern crate lazy_static;

use std::sync::Mutex;

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


lazy_static! {
    static ref FRAME_ID: Mutex<u8> = Mutex::new(0);
}

pub fn start() {
    let mut frame_id = FRAME_ID.lock().unwrap();
    *frame_id = 0;
}

pub fn transmit(s: String) -> Result<(),()> {
    tx::transmit(s)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
