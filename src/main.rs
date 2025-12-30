use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Context;
use hound::WavReader;
use serde::Deserialize;

fn client_func(file: WavFile, server_address: String, port: u16) -> anyhow::Result<()> {
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

    let mut stream = TcpStream::connect((server_address, port))?;
    stream.write_all(b"CONFIG\n")?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;

    let mut buf = [0u8; 200];
    let n = stream.read(&mut buf)?;
    let response = std::str::from_utf8(&buf[..n])?;
    let max_size: usize = response.split_whitespace().next().unwrap_or("0").parse()?;

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
        std::thread::sleep(Duration::from_millis(1000));
        counter += 1;
    }
}

fn main() -> anyhow::Result<()> {
    let config: Config = serde_yaml::from_reader(File::open("config.yml")?)?;

    for file in config.files {
        let server = config.server.clone();
        let port = config.port;
        let file_clone = file.clone();

        std::thread::spawn(move || {
            let error = format!("Failed for {:?}", &file_clone.path);
            client_func(file_clone, server, port).expect(&error);
        });
    }

    std::thread::sleep(Duration::MAX);
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
struct Config {
    files: Vec<WavFile>,
    server: String,
    port: u16,
}

#[derive(Clone, Debug, Deserialize)]
struct WavFile {
    path: PathBuf,
    #[serde(default)]
    start_offset: usize,
    // Percentage volume assignment
    #[serde(default = "default_adjustment")]
    volume_adjustment: f32,
}

fn default_adjustment() -> f32 {
    1.0
}
