use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use hound::WavReader;
use rand::Rng;
use serde::Deserialize;

fn client_func(config: Config, file: WavFile) -> anyhow::Result<()> {
    let mut reader =
        WavReader::open(&file.path).context(format!("Failed to read file at {:?}", &file.path))?;
    let spec = reader.spec();
    let fps = spec.sample_rate as usize;
    let total_frames = reader.duration() as usize;

    let frame_index = file.start_offset * fps;
    if frame_index >= total_frames {
        anyhow::bail!("Offset is beyond total frames");
    }

    reader.seek(frame_index as u32)?;

    // Read samples into Vec<i16>
    let end_frame = total_frames.saturating_sub(file.end_offset * fps);
    let take_frames = end_frame.saturating_sub(frame_index);
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .take(take_frames)
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

    let mut stream = TcpStream::connect((config.server.clone(), config.port))?;
    stream.write_all(b"CONFIG\n")?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;

    let mut buf = [0u8; 200];
    let n = stream.read(&mut buf)?;
    let response = std::str::from_utf8(&buf[..n])?;
    let mut max_size: usize = response.split_whitespace().next().unwrap_or("0").parse()?;
    if config.max_sample_length > 0 {
        max_size = max_size.min(fps * config.max_sample_length);
    }

    let total_samples = samples.len();

    let mut rng = rand::rng();

    let mut counter = 1;
    loop {
        println!("Sending sample for {:?}: {counter}", &file.path);

        for (position, sample) in samples.iter().enumerate() {
            // Only send as many packets as we're allowed to
            if position >= max_size {
                break;
            }

            let value = *sample as f32 / 32768.0 * file.volume_adjustment;
            let message = format!("SMPL {} {:.7}\n", position, value);
            stream.write_all(message.as_bytes())?;
        }

        // Sleep delay
        let mut delay = config.sample_delay;
        delay += rng.random_range(0..2000);
        std::thread::sleep(Duration::from_millis(delay as u64));

        counter += 1;
    }
}

fn main() -> anyhow::Result<()> {
    let config: Config = serde_yaml::from_reader(File::open("config.yml")?)?;

    for file in config.files.clone() {
        let config = config.clone();
        let file_clone = file.clone();

        std::thread::spawn(move || {
            let error = format!("Failed for {:?}", &file_clone.path);
            client_func(config, file_clone).expect(&error);
        });
    }

    std::thread::sleep(Duration::MAX);
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
struct Config {
    server: String,
    port: u16,
    #[serde(default)]
    max_sample_length: usize,
    #[serde(default)]
    sample_delay: usize,
    files: Vec<WavFile>,
}

#[derive(Clone, Debug, Deserialize)]
struct WavFile {
    path: PathBuf,
    // Offset in seconds that're skipped at sample start.
    #[serde(default)]
    start_offset: usize,

    // Offset in seconds that're skipped at sample end.
    #[serde(default)]
    end_offset: usize,

    // Percentage volume assignment
    #[serde(default = "default_adjustment")]
    volume_adjustment: f32,
}

fn default_adjustment() -> f32 {
    1.0
}
