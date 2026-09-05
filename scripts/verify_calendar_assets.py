#!/usr/bin/env python3
"""Check current calendar responses without temporary files or subprocesses."""
import re
import signal
import sys
import time
import urllib.error
import urllib.request


def response(opener, url):
    for attempt in range(16):
        try:
            with opener.open(url, timeout=5) as result:
                return result.read().decode('utf-8'), result.headers.get_content_type()
        except urllib.error.URLError as error:
            if not isinstance(error.reason, ConnectionRefusedError) or attempt == 15:
                raise
            time.sleep(1)


def verify(origin):
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    html, _ = response(opener, origin + '/')
    match = re.search(r'href="([^"]+\.css)"', html)
    if not match:
        raise ValueError('live HTML did not link a stylesheet')
    css, content_type = response(opener, origin + '/' + match[1].lstrip('/'))
    if content_type != 'text/css':
        raise ValueError('linked asset was not served as text/css')
    for marker in ('カレンダーの日を押すと', 'candidate-direct-entry'):
        if marker not in html:
            raise ValueError('live HTML is missing current marker: ' + marker)
    for marker in ('.candidate-calendar-toolbar', '.candidate-calendar-grid',
                   '.candidate-calendar-day', 'grid-template-columns:repeat(7,minmax(0,1fr))'):
        if marker not in css:
            raise ValueError('linked stylesheet is missing current marker: ' + marker)
    print(f'PASS: {origin} serves current calendar markup and stylesheet')


def main():
    def terminate(sig, _frame):
        raise SystemExit(128 + sig)
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, terminate)
    origin = (sys.argv[1] if len(sys.argv) > 1 else 'http://127.0.0.1:8083').rstrip('/')
    try:
        verify(origin)
    except (OSError, ValueError) as error:
        print(f'FAIL: {error}', file=sys.stderr)
        raise SystemExit(1) from error


if __name__ == '__main__':
    main()
