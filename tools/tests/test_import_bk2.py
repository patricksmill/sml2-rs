"""Movie import must preserve every row, button and boot-state assumption."""
import io
from pathlib import Path
import sys
import unittest
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from import_bk2 import convert


def archive(log, header='Platform GB\nAuthor Tester\nSHA1 abc\n', extra=None):
    out = io.BytesIO()
    with zipfile.ZipFile(out, 'w') as z:
        z.writestr('Header.txt', header)
        z.writestr('Input Log.txt', log)
        if extra:
            z.writestr(extra, b'unsupported state')
    return out.getvalue()


LOG_KEY = 'LogKey:#Up|Down|Left|Right|Start|Select|B|A|Power|\n'


class ImportMovieTests(unittest.TestCase):
    def test_neutral_rows_and_simultaneous_buttons_are_not_retimed(self):
        movie = convert(archive('[Input]\n' + LOG_KEY +
                                '|.........|\n|...R..BA.|\n|....S....|\n|.........|\n[/Input]'))
        self.assertEqual(movie['frames'], [0, 49, 64, 0])
        self.assertEqual(movie['author'], 'Tester')

    def test_uses_logkey_names_rather_than_fixed_column_positions(self):
        log = 'LogKey:#A|B|Select|Start|Right|Left|Down|Up|Power|\n|AB..R....|'
        self.assertEqual(convert(archive(log))['frames'], [49])

    def test_tasvideos_outer_zip(self):
        inner = archive(LOG_KEY + '|.........|')
        outer = io.BytesIO()
        with zipfile.ZipFile(outer, 'w') as z:
            z.writestr('author-game.bk2', inner)
        self.assertEqual(convert(outer.getvalue())['frames'], [0])

    def test_rejects_reset_state_start_unknown_keys_and_bad_rows(self):
        cases = [
            archive(LOG_KEY + '|........P|'),
            archive(LOG_KEY + '|.........|', extra='Core.bin.Savestate'),
            archive(LOG_KEY + '|.........|', extra='SaveRam.bin'),
            archive(LOG_KEY + '|.........|', header='Platform GB\nStartsFromSavestate True\n'),
            archive(LOG_KEY + '|.........|', header='Platform NES\n'),
            archive(LOG_KEY.replace('Power', 'Unknown') + '|.........|'),
            archive(LOG_KEY.replace('Power', 'A') + '|.........|'),
            archive(LOG_KEY + '|........|'),
            archive('|.........|'),
            archive(LOG_KEY),
        ]
        for data in cases:
            with self.subTest(data=data):
                with self.assertRaises(ValueError):
                    convert(data)


if __name__ == '__main__':
    unittest.main()
