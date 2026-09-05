"""A verified TCP connection may fail, but must never reconnect a mutation."""
from http.client import HTTPConnection


class BoundHTTPConnection(HTTPConnection):
    connected_once = False

    def connect(self):
        if self.connected_once:
            raise ConnectionError('Verified connection closed; refusing to reconnect a mutation')
        self.connected_once = True
        return super().connect()
