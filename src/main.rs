use std::env;
use std::io::{Read, Write};
use std::net::TcpStream;

use hound::WavReader;

fn client_func(
    wav_path: &str,
    server_ip: &str,
    port: u16,
    second_offset: usize,
) -> anyhow::Result<()> {
    let mut reader = WavReader::open(wav_path)?;
    let spec = reader.spec();
    let fps = spec.sample_rate as usize;
    let total_frames = reader.duration() as usize;

    let frame_index = second_offset * fps;
    if frame_index >= total_frames {
        anyhow::bail!("Offset is beyond total frames");
    }

    reader.seek(frame_index as u32)?;

    // Read samples into Vec<i16>
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .take(total_frames - frame_index)
        .collect::<Result<_, _>>()?;

    // If stereo, average channels
    let samples = if spec.channels == 2 {
        let mut mono = Vec::with_capacity(samples.len() / 2);
        for chunk in samples.chunks(2) {
            // Catch the case where there's a trailing byte
            if chunk.len() == 1 {
                continue;
            }
            let avg = ((chunk[0] as i32 + chunk[1] as i32) / 2) as i16;
            mono.push(avg);
        }
        mono
    } else {
        samples
    };

    let mut stream = TcpStream::connect((server_ip, port))?;
    stream.write_all(b"CONFIG\n")?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;

    let mut buf = [0u8; 200];
    let n = stream.read(&mut buf)?;
    let response = std::str::from_utf8(&buf[..n])?;
    let max_size: usize = response.split_whitespace().next().unwrap_or("0").parse()?;

    let mut counter = 1;
    loop {
        println!("Sending sample: {counter}");
        for (position, sample) in samples.iter().enumerate() {
            // Only send as many packets as we're allowed to
            if position >= max_size {
                break;
            }

            let value = *sample as f32 / 32768.0;
            let message = format!("SMPL {} {:.7}\n", position, value);
            stream.write_all(message.as_bytes())?;
        }
        counter += 1;
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        anyhow::bail!("Usage: {} <wav_file> [second_offset]", args[0]);
    }

    let wav_file = &args[1];
    let second_offset = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let server = "sampleflut.de";
    let port = 8080;

    client_func(wav_file, server, port, second_offset)?;
    Ok(())
}
