# Sampleflood client

Multi-threaded maximum-blasting sampleflood client for the [sampleflood.de] client.

## How to

1. Copy `example_config.yml` to `config.yml`
2. `cargo build --release`
3. `./target/release/sampleflood-client`
4. Adjust the `config.yml` to your liking

## Config

```
# Server address
server: "sampleflut.de"
# Server Port
port: 8080
# A list of wav files you want to play
files:
    # The path to the wav file
  - path: ./tracks/crazy.wav
    # Offset of 40 seconds before taking samples
    start_offset: 40
    # A percentual volume adjustment (40%)
    volume_adjustment: 0.4
  - path: ./tracks/duck_song.wav
```
