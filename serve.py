# Serves this folder on http://localhost:8080 with caching off, so the browser always loads this build.
import http.server, functools, os, sys
class H(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Cache-Control', 'no-store')
        super().end_headers()
H.extensions_map['.wasm'] = 'application/wasm'
port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
http.server.ThreadingHTTPServer(('', port), functools.partial(H, directory=os.path.dirname(os.path.abspath(__file__)))).serve_forever()
