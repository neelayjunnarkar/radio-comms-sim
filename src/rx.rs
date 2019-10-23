use super::{Packet, FFT_BUF_LEN, FRAMES_PER_BUFFER, IN_BUF_LEN, PACKET_LEN_IDX, PACKET_MAX_LEN, RADIO};
use std::sync::mpsc;
use rustfft::algorithm::Radix4;
use rustfft::FFT;
use rustfft::num_complex::Complex;
use rustfft::num_traits::Zero;

pub fn start(
    audio_in_rx: mpsc::Receiver<[f32; FRAMES_PER_BUFFER as usize]>,
    packet_tx: mpsc::Sender<String>,
) -> Result<(), String> {
    while let Ok(recv_buf) = audio_in_rx.recv() {
        let mut fft_buf = None;
        {
            let mut radio_guard = RADIO.lock().expect("Failed to lock radio mutex");
            let ref mut radio = (radio_guard.as_mut()).expect("Radio not initialized");

            add_to_buf(&mut radio.audio_in_buf, &mut radio.in_buf_next, &recv_buf);

            if radio.in_buf_next * FRAMES_PER_BUFFER as usize >= FFT_BUF_LEN {
                let pos = radio.in_buf_next - FFT_BUF_LEN / FRAMES_PER_BUFFER as usize;
                let mut buf = [Complex::zero(); FFT_BUF_LEN];
                for i in 0..FFT_BUF_LEN {
                    buf[i] = Complex::new(radio.audio_in_buf[pos * FRAMES_PER_BUFFER as usize + i], 0.0);
                }
                fft_buf = Some(buf);
            }
        }
        if let Some(mut fft_buf) = fft_buf {
            let present_tones = get_present_tones(&mut fft_buf);
            println!("{:?}", present_tones);
        }
    }
    Ok(())
}

pub fn receive() -> Option<String> {
    let mut radio_guard = RADIO.lock().expect("Failed to lock radio mutex");
    let ref mut radio = (radio_guard.as_mut()).expect("Radio not initialized");
    radio.packet_rx.try_recv().ok()
}

fn add_to_buf(
    dst: &mut [f32; IN_BUF_LEN],
    pos: &mut usize,
    src: &[f32; FRAMES_PER_BUFFER as usize],
) {
    if *pos >= IN_BUF_LEN / FRAMES_PER_BUFFER as usize {
        // make room for new data
        for i in 0..(IN_BUF_LEN - FRAMES_PER_BUFFER as usize) {
            (*dst)[i] = (*dst)[i + FRAMES_PER_BUFFER as usize];
        }
        *pos = IN_BUF_LEN / FRAMES_PER_BUFFER as usize - 1;
    }
    for i in 0..FRAMES_PER_BUFFER as usize {
        (*dst)[FRAMES_PER_BUFFER as usize * *pos + i] = (*src)[i];
    }
    *pos += 1;
}

fn get_present_tones(buf: &mut [Complex<f32>]) -> Vec<f32> {
    let fft = Radix4::new(FFT_BUF_LEN, false);
    let mut output: Vec<Complex<f32>> = vec![Complex::zero(); FFT_BUF_LEN]; 
    fft.process(buf, &mut output);
    output.iter().map(|s| s.norm()).collect()
}
