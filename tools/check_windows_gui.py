#!/usr/bin/env python3
"""Check built Windows GUI binaries for their subsystem and embedded app icons."""
from pathlib import Path
import struct
import sys


def check(path: Path) -> None:
    data = path.read_bytes()
    if data[:2] != b'MZ':
        raise ValueError('not a Windows executable')
    pe = struct.unpack_from('<I', data, 0x3c)[0]
    if data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('missing PE header')
    sections = struct.unpack_from('<H', data, pe + 6)[0]
    optional_size = struct.unpack_from('<H', data, pe + 20)[0]
    optional = pe + 24
    magic = struct.unpack_from('<H', data, optional)[0]
    directory = {0x10b: 96, 0x20b: 112}.get(magic)
    if directory is None:
        raise ValueError('unsupported PE optional header')
    subsystem = struct.unpack_from('<H', data, optional + 68)[0]
    if subsystem != 2:  # IMAGE_SUBSYSTEM_WINDOWS_GUI
        raise ValueError(f'expected Windows GUI subsystem, found {subsystem}')
    resource_rva, resource_size = struct.unpack_from('<II', data, optional + directory + 16)
    if not resource_rva or not resource_size:
        raise ValueError('missing Windows resources')
    resource_offset = None
    for index in range(sections):
        at = optional + optional_size + index * 40
        virtual_size, virtual_address, raw_size, raw_offset = struct.unpack_from('<IIII', data, at + 8)
        if virtual_address <= resource_rva < virtual_address + max(virtual_size, raw_size):
            resource_offset = raw_offset + resource_rva - virtual_address
            break
    if resource_offset is None:
        raise ValueError('resource directory does not map to a PE section')
    named, ids = struct.unpack_from('<HH', data, resource_offset + 12)
    types = {
        struct.unpack_from('<I', data, resource_offset + 16 + index * 8)[0]
        for index in range(named + ids)
    }
    if not {3, 14} <= types:  # RT_ICON, RT_GROUP_ICON
        raise ValueError('missing icon or icon-group resource')

    # Check every resolution from the shipped icon, rather than accepting a
    # generic icon supplied by a dependency. Resource compilers copy each image.
    icon = (Path(__file__).resolve().parents[1] / 'assets/icon.ico').read_bytes()
    reserved, kind, count = struct.unpack_from('<HHH', icon)
    if reserved != 0 or kind != 1 or not count:
        raise ValueError('invalid source icon')
    for index in range(count):
        size, offset = struct.unpack_from('<II', icon, 6 + index * 16 + 8)
        image = icon[offset:offset + size]
        if not size or len(image) != size or image not in data:
            raise ValueError(f'missing application icon image {index + 1}')
    print(f'{path.name}: Windows GUI subsystem, all {count} application icon sizes embedded')


def main() -> int:
    if len(sys.argv) < 2:
        print('usage: check_windows_gui.py EXE [EXE ...]', file=sys.stderr)
        return 2
    for argument in sys.argv[1:]:
        path = Path(argument)
        try:
            check(path)
        except (OSError, ValueError, struct.error) as error:
            print(f'{path}: {error}', file=sys.stderr)
            return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
