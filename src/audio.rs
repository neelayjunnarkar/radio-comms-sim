use super::{
    CHANNELS, FRAMES_PER_BUFFER, INTERLEAVED, PREAMBLE, SAMPLE_RATE, SEND_INTERVAL_MS, TABLE_SIZE,
};
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

pub fn start(
    audio_out_rx: mpsc::Receiver<Vec<u8>>,
    audio_in_tx: mpsc::Sender<[f32; FRAMES_PER_BUFFER as usize]>,
) -> Result<(), String> {
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

    let def_input = pa
        .default_input_device()
        .expect("failed to find input device");
    let input_info = pa.device_info(def_input).expect("failed to get input info");
    let in_latency = input_info.default_low_input_latency;
    let input_params =
        pa::StreamParameters::<f32>::new(def_input, CHANNELS, INTERLEAVED, in_latency);

    let def_output = pa
        .default_output_device()
        .expect("failed to find output device");
    let output_info = pa
        .device_info(def_output)
        .expect("failed to get output info");
    let out_latency = output_info.default_low_output_latency;
    let output_params = pa::StreamParameters::new(def_output, CHANNELS, INTERLEAVED, out_latency);

    pa.is_duplex_format_supported(input_params, output_params, SAMPLE_RATE)
        .expect("no duplex support");
    let pa_cfg =
        pa::DuplexStreamSettings::new(input_params, output_params, SAMPLE_RATE, FRAMES_PER_BUFFER);

    let sines = [sine_0, sine_1];
    let mut sines_idx = 0;

    let (tx_note, rx_note) = mpsc::channel();

    let mut last_note_idx = 0;
    let cb = move |pa::DuplexStreamCallbackArgs {
                       in_buffer,
                       out_buffer,
                       frames,
                       ..
                   }| {
        last_note_idx = if let Ok(sines_idx) = rx_note.try_recv() {
            sines_idx
        } else {
            last_note_idx
        };

        assert!(frames == FRAMES_PER_BUFFER as usize);
        audio_in_tx
            .send({
                let mut dst = [0.0; FRAMES_PER_BUFFER as usize];
                for i in 0..FRAMES_PER_BUFFER as usize {
                    dst[i] = in_buffer[2 * i];
                }
                dst
            })
            .unwrap_or(());

        let mut idx = 0;
        for _ in 0..frames {
            out_buffer[idx] = sines[last_note_idx][phase];
            out_buffer[idx + 1] = sines[last_note_idx][phase];
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

        while let Ok(packet) = audio_out_rx.try_recv() {
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
