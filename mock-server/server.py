# /// script
# requires-python = '>=3.13'
# dependencies = [
#   'httpserver==1.1.0',
# ]
# ///

from functools import cached_property
from http.server import BaseHTTPRequestHandler, HTTPServer


class Handler(BaseHTTPRequestHandler):
    @cached_property
    def post_data(self):
        content_length = int(self.headers.get('Content-Length', 0))
        return self.rfile.read(content_length)

    def do_POST(self):
        self.send_response(200)
        self.end_headers()
        print(self.post_data.decode('UTF-8'))


if __name__ == '__main__':
    server = HTTPServer(('0.0.0.0', 8080), Handler)
    server.serve_forever()
