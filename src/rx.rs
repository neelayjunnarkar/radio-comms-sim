use super::{IN_BUF_LEN, Packet, PACKET_LEN_IDX, PACKET_MAX_LEN, RADIO, FRAMES_PER_BUFFER};
use std::sync::{mpsc};

pub fn start(audio_in_rx: mpsc::Receiver<[f32; FRAMES_PER_BUFFER as usize]>, packet_tx: mpsc::Sender<String>) -> Result<(), String> {
    while let Ok(recv_buf) = audio_in_rx.recv() {
        let mut radio_guard = RADIO.lock().expect("Failed to lock radio mutex");
        let ref mut radio = (radio_guard.as_mut()).expect("Radio not initialized");
        add_to_buf(&mut radio.audio_in_buf, &mut radio.in_buf_next, &recv_buf);
    }
    Ok(())
}

pub fn receive() -> Option<String> {
    let mut radio_guard = RADIO.lock().expect("Failed to lock radio mutex");
    let ref mut radio = (radio_guard.as_mut()).expect("Radio not initialized");
    radio.packet_rx.try_recv().ok()
}

fn add_to_buf(dst: &mut [f32; IN_BUF_LEN], pos: &mut usize, src: &[f32; FRAMES_PER_BUFFER as usize]) {
    if *pos >= IN_BUF_LEN / FRAMES_PER_BUFFER as usize {
        // copy the last buffer to the first section
        for i in 0..FRAMES_PER_BUFFER as usize {
            (*dst)[i] = (*src)[i];
        }
        *pos = 1;
    }
    for i in 0..FRAMES_PER_BUFFER as usize {
        (*dst)[FRAMES_PER_BUFFER as usize * *pos  + i] = (*src)[i];
    }
    *pos += 1;
} 
