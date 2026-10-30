"""Exact archive-boundary compilation controls; no factor-production inputs."""
import gzip
import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest

from resident_embedded_archives import embed_prefixes


class PrefixCompilationTest(unittest.TestCase):
    def test_only_the_bound_prefix_enters_the_compiled_carrier(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as folder:
            root = Path(folder)
            source = root / 'source.records'
            rows = []
            for value in [(0, []), (0, [3, 2, 1])]:
                record = gzip.compress(json.dumps(value).encode())
                rows.append(struct.pack('<Q', len(record)) + record)
            prefix = b''.join(rows)
            source.write_bytes(prefix + b'following execution records')
            out = root / 'compiled'
            out.mkdir()
            declaration, manifest = embed_prefixes(out, [{'banks': {
                'external_archive': str(source), 'bytes': len(prefix), 'records': 2,
                'encoding': 'length-u64le-gzip-json'}}])
            bound = manifest['prefixes'][0]
            self.assertEqual(Path(bound['embedded_file']).read_bytes(), prefix)
            self.assertEqual(bound['sha256'], hashlib.sha256(prefix).hexdigest())
            self.assertIn('include_bytes!', declaration)
            self.assertIn('EMBEDDED_ARCHIVES', declaration)

    def test_truncated_or_inconsistent_prefix_is_rejected(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as folder:
            root = Path(folder)
            source = root / 'source.records'
            source.write_bytes(struct.pack('<Q', 10) + b'short')
            for index, (boundary, count) in enumerate([(18, 1), (8, 1), (13, 0)]):
                out = root / str(index)
                out.mkdir()
                with self.assertRaises(ValueError):
                    embed_prefixes(out, [{'external_archive': str(source), 'bytes': boundary,
                                         'records': count, 'encoding': 'length-u64le-gzip-json'}])


if __name__ == '__main__':
    unittest.main()
