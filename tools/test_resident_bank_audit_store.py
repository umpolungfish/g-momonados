"""Lossless bank indexing, interruption recovery and corrupted-cache controls."""
from array import array
import bisect
import gc
import json
from pathlib import Path
import tempfile
import unittest

from resident_bank_audit_store import MappedBankStore,HEADER


class MappedBankAuditTest(unittest.TestCase):
    def test_bank_values_parent_fibres_and_empty_overlay_survive_reopen(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as folder:
            root=Path(folder);binding={'input':'unchanged-source'}
            store=MappedBankStore(root,binding)
            store.append(0,array('Q',[1,3]),bytearray([1,2]))
            store.append(1,array('Q',[4,1<<40]),bytearray([0,255]))
            self.assertEqual(list(store.parents),[0,0,1])
            self.assertEqual(list(store.keys[2]),[4,1<<40])
            self.assertEqual(bisect.bisect_left(store.keys[1],3),1)
            del store;gc.collect()
            store=MappedBankStore(root,binding)
            self.assertEqual(store.reused,2)
            self.assertEqual(list(store.values[2]),[0,255])
            store.append(2,array('Q'),bytearray())
            self.assertEqual(store.parents[3],2)
            self.assertEqual(len(store.keys[3]),0)
            del store;gc.collect()

    def test_checksum_and_input_changes_are_rejected(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as folder:
            root=Path(folder);binding={'input':'original'}
            store=MappedBankStore(root,binding);store.append(0,array('Q',[2]),bytearray([1]))
            del store;gc.collect()
            with self.assertRaisesRegex(ValueError,'input or validator'):
                MappedBankStore(root,{'input':'different'})
            gc.collect()
            path=root/'00000001.bank'
            with path.open('r+b') as stream:
                stream.seek(HEADER.size);stream.write(b'\x03')
            with self.assertRaisesRegex(ValueError,'content changed'):
                MappedBankStore(root,binding)
            gc.collect()

    def test_committed_data_before_index_publication_is_recovered(self):
        with tempfile.TemporaryDirectory(dir=Path(__file__).parent) as folder:
            root=Path(folder);binding={'input':'same-record-prefix'}
            store=MappedBankStore(root,binding)
            store.append(0,array('Q',[1]),bytearray([1]))
            before=(root/'index.json').read_text()
            store.append(1,array('Q',[3]),bytearray([2]))
            del store;gc.collect()
            # Model an interruption after the second bank's atomic rename but
            # before its new index was published. Revalidate that input bank.
            (root/'index.json').write_text(before)
            store=MappedBankStore(root,binding)
            self.assertEqual(store.reused,1)
            store.append(1,array('Q',[3]),bytearray([2]))
            self.assertEqual(len(store.parents),3)
            self.assertEqual(len(json.loads((root/'index.json').read_text())['banks']),2)
            del store;gc.collect()


if __name__=='__main__':unittest.main()
