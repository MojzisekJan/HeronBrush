"""Verify the actual diagnostic host's HTTP and compression behavior."""
import gzip
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import unittest
import urllib.error
import urllib.request


class WebHostCheck(unittest.TestCase):
    def test_gzip_cache_validation_and_wasm_type(self):
        script = Path(__file__).resolve().parents[1] / 'serve_web_release.py'
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            html = b'<html>Heron</html>' * 100
            wasm = b'\0asm' + b'\0' * 2000
            (root / 'index.html').write_bytes(html)
            (root / 'editor-0123456789abcdef_bg.wasm').write_bytes(wasm)
            (root / 'startup-0123456789abcdef.bin').write_bytes(b'archive' * 100)
            process = subprocess.Popen([sys.executable, '-u', str(script), directory],
                                       stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
            try:
                self.assertTrue(select.select([process.stdout], [], [], 10)[0], 'Server did not start')
                url = process.stdout.readline().strip()
                request = urllib.request.Request(url, headers={'Accept-Encoding': 'gzip'})
                with urllib.request.urlopen(request, timeout=10) as response:
                    self.assertEqual(response.headers['Content-Encoding'], 'gzip')
                    self.assertEqual(response.headers['Vary'], 'Accept-Encoding')
                    self.assertEqual(response.headers['Cache-Control'], 'no-cache')
                    self.assertEqual(gzip.decompress(response.read()), html)
                    tag = response.headers['ETag']
                with self.assertRaises(urllib.error.HTTPError) as error:
                    urllib.request.urlopen(urllib.request.Request(url, headers={
                        'Accept-Encoding': 'gzip', 'If-None-Match': tag}), timeout=10)
                self.assertEqual(error.exception.code, 304)
                self.assertEqual(error.exception.read(), b'')
                with urllib.request.urlopen(urllib.request.Request(url, headers={
                    'Accept-Encoding': 'gzip;q=0', 'If-None-Match': tag}), timeout=10) as response:
                    self.assertIsNone(response.headers['Content-Encoding'])
                    self.assertEqual(response.read(), html)
                    self.assertNotEqual(response.headers['ETag'], tag)
                with urllib.request.urlopen(url + 'editor-0123456789abcdef_bg.wasm', timeout=10) as response:
                    self.assertEqual(response.headers['Content-Type'], 'application/wasm')
                    self.assertIn('immutable', response.headers['Cache-Control'])
                    self.assertEqual(response.read(), wasm)
                with urllib.request.urlopen(url + 'startup-0123456789abcdef.bin', timeout=10) as response:
                    self.assertIn('immutable', response.headers['Cache-Control'])
                with self.assertRaises(urllib.error.HTTPError) as error:
                    urllib.request.urlopen(url + '%2e%2e/Cargo.toml', timeout=10)
                self.assertEqual(error.exception.code, 404)
            finally:
                process.terminate()
                process.wait(timeout=10)
                process.stdout.close()


if __name__ == '__main__':
    unittest.main()
