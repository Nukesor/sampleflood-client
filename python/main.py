import socket
import wave
import struct
import sys
import numpy as np


def client_func(wav_file: str, server_ip: str, port: int):
    with wave.open(wav_file, "rb") as wav_file:
        second_offset = 0
        if len(sys.argv) > 2:
            second_offset = sys.argv[2]

        fps = wav_file.getframerate()
        print(f"Current {fps} with {second_offset} seconds offset")
        print(f"total frames: {wav_file.getnframes()}")
        frame_index = int(second_offset) * int(fps)

        print(f"Frame index: {frame_index}")
        wav_file.setpos(frame_index)

        frames = wav_file.readframes(wav_file.getnframes())
        samples = np.array(
            struct.unpack("<" + "h" * (len(frames) // 2), frames),
            dtype=np.int16,
        )
        channels = wav_file.getnchannels()

        if channels == 2:
            samples = samples.reshape(-1, channels).mean(axis=1).astype(np.int16)

        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sck:
            sck.connect((server_ip, port))
            sck.send(b"CONFIG\n")
            sck.settimeout(2)
            buffer = sck.recv(200)
            max_size = int(buffer.split()[0])
            print(f"max_size: {max_size}")
            while True:
                for position in range(0, min(max_size, len(samples) - 1)):
                    sample = samples[position]
                    value = sample / 32768.0  # 16-bit PCM-values from -32768 bis 32767
                    message = f"SMPL {position} {value:.7f}\n"
                    sck.sendall(message.encode("utf-8"))


if __name__ == "__main__":
    wav_file = sys.argv[1]
    server = "sampleflut.de"
    port = 8080
    print(f"using {wav_file=} {server=} {port=}")

    client_func(wav_file=wav_file, server_ip=server, port=port)
