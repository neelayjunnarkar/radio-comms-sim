use super::{CHANNELS, FRAMES_PER_BUFFER, PREAMBLE, SAMPLE_RATE, SEND_INTERVAL_MS, TABLE_SIZE};
use portaudio as pa;
use std::collections::VecDeque;
use std::f64::consts::PI;
use std::sync::mpsc;

#[derive(PartialEq)]
enum TxState {
    Preamble,
    Packet,
    NoTx,
}

pub fn start(rx: mpsc::Receiver<Vec<u8>>) -> Result<(), String> {
    let mut buf: VecDeque<Vec<u8>> = VecDeque::new();
    let mut curr_packet: Option<Vec<u8>> = None;
    let mut tx_state: TxState = TxState::NoTx;
    let mut counter = 0;

    /* set up portaudio */
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
    let pa = pa::PortAudio::new().expect("failed to initialize portaudio");

    let pa_cfg = pa
        .default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER)
        .expect("Failed to create portaudio stream settings");

    let sines = [sine_0, sine_1];
    let mut sines_idx = 0;

    let (tx_note, rx_note) = mpsc::channel();

    let mut last_note_idx = 0;
    let cb = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        last_note_idx = if let Ok(sines_idx) = rx_note.try_recv() {
            sines_idx
        } else {
            last_note_idx
        };

        let mut idx = 0;
        for _ in 0..frames {
            buffer[idx] = sines[last_note_idx][phase];
            buffer[idx + 1] = sines[last_note_idx][phase];
            phase += 1;
            if phase >= TABLE_SIZE {
                phase -= TABLE_SIZE;
            }
            idx += 2;
        }
        pa::Continue
    };

    let mut stream = pa
        .open_non_blocking_stream(pa_cfg, cb)
        .expect("Failed to open non-blocking stream");

    stream.start().expect("Failed to start stream");

    loop {
        tx_note.send(sines_idx).unwrap_or(());

        while let Ok(packet) = rx.try_recv() {
            println!("received packet to send");
            buf.push_back(packet);
        }

        if tx_state == TxState::NoTx {
            if let Some(packet) = buf.pop_front() {
                curr_packet = Some(packet);
                counter = 0;
                tx_state = TxState::Preamble;
            }
        }

        match tx_state {
            TxState::Preamble => {
                sines_idx = PREAMBLE[counter] as usize;
                counter += 1;
                if counter >= PREAMBLE.len() {
                    counter = 0;
                    tx_state = TxState::Packet;
                }
            }
            TxState::Packet => {
                if let Some(ref packet) = curr_packet {
                    sines_idx = (*packet)[counter] as usize;
                    counter += 1;
                    if counter >= packet.len() {
                        counter = 0;
                        tx_state = TxState::NoTx;
                    }
                } else {
                    tx_state = TxState::NoTx;
                }
            }
            TxState::NoTx => {}
        }
        std::thread::sleep(std::time::Duration::from_millis(SEND_INTERVAL_MS));
    }
}
