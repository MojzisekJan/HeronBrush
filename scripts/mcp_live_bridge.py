#!/usr/bin/env python3
"""Bridge stdio MCP to a private live editor invitation in HERON_MCP_ENDPOINT."""
import os
import socket
import sys
import threading


def main():
    endpoint = os.environ.get('HERON_MCP_ENDPOINT', '')
    address, separator, token = endpoint.partition('#')
    host, colon, port = address.rpartition(':')
    if not separator or not colon or host != '127.0.0.1' or len(token) != 64 or any(c not in '0123456789abcdef' for c in token):
        raise ValueError('Set HERON_MCP_ENDPOINT to the private invitation copied from the editor')
    with socket.create_connection((host, int(port)), timeout=5) as connection:
        connection.settimeout(None)
        connection.sendall(f'HERON-MCP {token}\n'.encode('ascii'))

        def send():
            try:
                while chunk := sys.stdin.buffer.read1(65536):
                    connection.sendall(chunk)
                connection.shutdown(socket.SHUT_WR)
            except OSError:
                pass

        threading.Thread(target=send, daemon=True).start()
        while chunk := connection.recv(65536):
            sys.stdout.buffer.write(chunk)
            sys.stdout.buffer.flush()


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError):
        print('Could not connect to the live editor. Check its private MCP invitation.', file=sys.stderr)
        sys.exit(1)
