"""Verify private invitation forwarding without launching the GUI."""
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import threading
import unittest

BRIDGE = Path(__file__).resolve().parents[1] / 'mcp_live_bridge.py'


class BridgeTest(unittest.TestCase):
    def test_authentication_precedes_json_and_stdout_contains_only_response(self):
        token = 'a' * 64
        request = b'{"jsonrpc":"2.0","id":1,"method":"initialize"}\n'
        response = b'{"jsonrpc":"2.0","id":1,"result":{}}\n'
        observed = []
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            listener.listen()
            listener.settimeout(5)

            def server():
                with listener.accept()[0] as connection:
                    connection.settimeout(5)
                    reader = connection.makefile('rb')
                    observed.extend([reader.readline(), reader.readline()])
                    connection.sendall(response)

            worker = threading.Thread(target=server)
            worker.start()
            result = subprocess.run([sys.executable, str(BRIDGE)], input=request,
                                    capture_output=True, timeout=8,
                                    env=dict(os.environ, HERON_MCP_ENDPOINT=f'127.0.0.1:{listener.getsockname()[1]}#{token}'))
            worker.join(timeout=6)
        self.assertEqual(observed, [f'HERON-MCP {token}\n'.encode(), request])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, response)
        self.assertNotIn(token.encode(), result.stderr)

    def test_invalid_invitation_is_rejected_without_echoing_secret(self):
        result = subprocess.run([sys.executable, str(BRIDGE)], capture_output=True,
                                env=dict(os.environ, HERON_MCP_ENDPOINT='invalid#private'), timeout=5)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, b'')
        self.assertNotIn(b'invalid#private', result.stderr)


if __name__ == '__main__':
    unittest.main()
