use super::{Packet, PACKET_LEN_IDX, PACKET_MAX_LEN, RADIO};
use std::sync::{mpsc, Mutex};

pub fn start(audio_in_rx: mpsc::Receiver<Vec<f32>>, packet_tx: mpsc::Sender<String>) -> Result<(), String> {
    while let recv_buf_res = audio_in_rx.recv() {
        if recv_buf_res.is_err() {
            continue;
        }
        let recv_buf = recv_buf_res.expect("somehow still an error");
        println!("{:?}", recv_buf);
    }
    Ok(())
}

pub fn receive() -> Option<String> {
    let mut radio_guard = RADIO.lock().expect("Failed to lock radio mutex");
    let ref mut radio = (radio_guard.as_mut()).expect("Radio not initialized");
    radio.packet_rx.try_recv().ok()
}
