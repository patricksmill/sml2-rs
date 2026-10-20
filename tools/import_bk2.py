#!/usr/bin/env python3
"""Convert a power-on GB BizHawk movie to native held-button frames (no retiming)."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import zipfile

# Native mask order, deliberately independent of the movie's LogKey order.
BUTTONS = {'Right': 1, 'Left': 2, 'Up': 4, 'Down': 8,
           'A': 16, 'B': 32, 'Start': 64, 'Select': 128}


def convert(data):
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        # TASVideos sometimes wraps the actual .bk2 in another ZIP.
        if 'Header.txt' not in archive.namelist():
            movies = [n for n in archive.namelist() if n.lower().endswith('.bk2')]
            if len(movies) != 1:
                raise ValueError('expected one BK2 movie in download')
            return convert(archive.read(movies[0]))
        names = archive.namelist()
        if any('savestate' in n.lower() or 'saveram' in n.lower() for n in names):
            raise ValueError('savestate/SRAM-start movies are unsupported; use a power-on movie')
        header = dict(line.split(' ', 1) for line in
                      archive.read('Header.txt').decode('utf-8-sig').splitlines()
                      if ' ' in line)
        if header.get('Platform') != 'GB':
            raise ValueError('only single Game Boy movies are supported')
        if any(header.get(k, '').lower() in ('true', '1')
               for k in ('StartsFromSavestate', 'StartsFromSaveRam')):
            raise ValueError('movie must start from power-on')
        lines = archive.read('Input Log.txt').decode('utf-8-sig').splitlines()
        keys = None
        frames = []
        for line in lines:
            if line.startswith('LogKey:'):
                keys = line[len('LogKey:'):].replace('#', '').strip('|').split('|')
                # GBHawk prefixes its single controller; reject any other player.
                keys = [key.removeprefix('P1 ') for key in keys]
                if len(keys) != len(set(keys)) or set(keys) - (set(BUTTONS) | {'Power', 'Reset'}):
                    raise ValueError(f'unsupported/duplicate controls: {keys}')
                if not set(BUTTONS).issubset(keys):
                    raise ValueError('movie is missing Game Boy controls')
            elif line.startswith('|'):
                if keys is None:
                    raise ValueError('input row precedes LogKey')
                row = line.strip('|')
                if len(row) != len(keys):
                    raise ValueError(f'bad input width at frame {len(frames)}')
                mask = 0
                for name, value in zip(keys, row):
                    if value not in ('.', ' '):
                        if name in ('Power', 'Reset'):
                            raise ValueError(f'{name} at frame {len(frames)} is unsupported')
                        mask |= BUTTONS[name]
                frames.append(mask)
        if not frames:
            raise ValueError('empty movie')
        return {'format': 'sml2-tas-v1', 'author': header.get('Author', 'unknown'),
                'game': header.get('GameName', ''), 'rom_sha1': header.get('SHA1', '').lower(),
                'emulator': header.get('emuVersion', ''), 'frames': frames}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('movie', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--rom', type=Path, required=True, help='verify the exact ROM SHA1')
    args = parser.parse_args()
    movie = convert(args.movie.read_bytes())
    actual = hashlib.sha1(args.rom.read_bytes()).hexdigest()
    if not movie['rom_sha1'] or actual != movie['rom_sha1']:
        parser.error(f"ROM SHA1 mismatch: movie={movie['rom_sha1']}, supplied={actual}")
    args.output.write_text(json.dumps(movie) + '\n')
    print(f"{len(movie['frames'])} frames, author {movie['author']}; ROM verified; no frames removed")


if __name__ == '__main__':
    main()
