#!/usr/bin/env python3
"""Export PyBoy song/SFX reference WAVs; use --help for options."""

import argparse
import sys
import os
import wave
import numpy as np

try:
    import pyboy
except ImportError:
    print("Error: pyboy not installed in current environment.", file=sys.stderr)
    sys.exit(1)

def export_pyboy_song(rom_path, song_id, seconds, out_path):
    pb = pyboy.PyBoy(rom_path, sound_emulated=True, window="null")
    # Let game boot and sound subsystem initialize (approx 120 frames)
    for _ in range(120):
        pb.tick()

    # Clear any residual requests and request song
    pb.memory[0xA468] = song_id

    total_frames = int(seconds * 60)
    audio_frames = []
    for f in range(total_frames):
        pb.tick()
        audio_frames.append(pb.sound.ndarray.copy())
    pb.stop()

    write_wav(audio_frames, 48000, out_path)
    print(f"Exported PyBoy Song {song_id} to {out_path} ({seconds:.1f}s)")

def export_pyboy_sfx(rom_path, slot, sfx_id, frames, out_path):
    pb = pyboy.PyBoy(rom_path, sound_emulated=True, window="null")
    # Let game boot
    for _ in range(120):
        pb.tick()

    req_addr = {0: 0xA460, 1: 0xA470, 2: 0xA478}.get(slot)
    if req_addr is None:
        raise ValueError(f"Invalid slot {slot}")
    pb.memory[req_addr] = sfx_id

    audio_frames = []
    for _ in range(frames):
        pb.tick()
        audio_frames.append(pb.sound.ndarray.copy())
    pb.stop()

    write_wav(audio_frames, 48000, out_path)
    print(f"Exported PyBoy SFX slot {slot} id {sfx_id} to {out_path} ({frames} frames)")

def write_wav(audio_frames, sample_rate, out_path):
    raw_audio = np.concatenate(audio_frames, axis=0) # shape (N, 2), int8

    # Apply DC blocking high-pass filter:
    # Game Boy analog sound output is AC-coupled via capacitors (~15-20 Hz cutoff)
    filtered = np.zeros_like(raw_audio, dtype=np.float32)
    cap = np.zeros(2, dtype=np.float32)
    for i in range(len(raw_audio)):
        sample = raw_audio[i].astype(np.float32)
        cap += (sample - cap) * 0.002
        filtered[i] = sample - cap

    # Normalize to 16-bit PCM
    peak = np.abs(filtered).max()
    if peak > 0:
        pcm16 = (filtered / peak * 32000.0).astype(np.int16)
    else:
        pcm16 = np.zeros_like(filtered, dtype=np.int16)

    os.makedirs(os.path.dirname(os.path.abspath(out_path)), exist_ok=True)
    with wave.open(out_path, "wb") as wf:
        wf.setnchannels(2)
        wf.setsampwidth(2)
        wf.setframerate(sample_rate)
        wf.writeframes(pcm16.tobytes())

def main():
    parser = argparse.ArgumentParser(description="Dump audio from PyBoy")
    parser.add_argument("--rom", default="decomp/baserom.gb", help="Path to baserom.gb")
    parser.add_argument("--song", type=int, help="Song ID (1..42)")
    parser.add_argument("--sfx", nargs=2, type=int, metavar=("SLOT", "ID"), help="SFX slot (0..2) and ID")
    parser.add_argument("--seconds", type=float, default=10.0, help="Duration for song in seconds")
    parser.add_argument("--frames", type=int, default=120, help="Frames for SFX")
    parser.add_argument("--out", required=True, help="Output WAV path")

    args = parser.parse_args()

    if args.song is not None:
        export_pyboy_song(args.rom, args.song, args.seconds, args.out)
    elif args.sfx is not None:
        export_pyboy_sfx(args.rom, args.sfx[0], args.sfx[1], args.frames, args.out)
    else:
        parser.error("Must specify either --song or --sfx")

if __name__ == "__main__":
    main()
