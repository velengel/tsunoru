"""A connection that verified identity must not reconnect to a replacement server."""
from contextlib import closing
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import sys
import threading

sys.dont_write_bytecode = True
from verification_connection import BoundHTTPConnection

foreign_writes = []
identity = {'public_id': 'test-identity', 'name': 'owned database'}

class Foreign(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_POST(self):
        foreign_writes.append(self.path)
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b'{}')

class Owned(BaseHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def do_GET(self):
        # Close only the listening socket; this established response can finish.
        owned.socket.close()
        replacement.server_bind()
        replacement.server_activate()
        replacement_thread.start()
        self.send_response(200)
        self.send_header('Connection', 'close')
        self.end_headers()
        self.wfile.write(json.dumps(identity).encode())

with ThreadingHTTPServer(('127.0.0.1', 0), Owned) as owned:
    replacement = ThreadingHTTPServer(owned.server_address, Foreign, bind_and_activate=False)
    owned_thread = threading.Thread(target=owned.serve_forever, daemon=True)
    replacement_thread = threading.Thread(target=replacement.serve_forever, daemon=True)
    owned_thread.start()
    try:
        with closing(BoundHTTPConnection(*owned.server_address, timeout=5)) as connection:
            connection.request('GET', '/api/events/get')
            assert json.loads(connection.getresponse().read()) == identity
            failure = None
            try:
                connection.request('POST', '/api/events/create', body='{}')
                connection.getresponse().read()
            except OSError as error:
                failure = error
            assert not foreign_writes, 'Replacement listener received a mutation'
            assert failure is not None, 'Closed verified connection must fail instead of reconnecting'
            print('PASS Python: listener handover receives zero writes after a valid identity response')
    finally:
        owned.shutdown()
        if replacement_thread.ident is not None:
            replacement.shutdown()
            replacement_thread.join(timeout=5)
        replacement.server_close()
        owned_thread.join(timeout=5)
