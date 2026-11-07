"""Servidor local da prévia, sem cache (módulos JS sempre atualizados)."""
import http.server
import os

PORT = 5500


class NoCacheHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


if __name__ == "__main__":
    os.chdir(os.path.dirname(os.path.abspath(__file__)))
    with http.server.ThreadingHTTPServer(("127.0.0.1", PORT), NoCacheHandler) as httpd:
        print(f"Prévia em http://127.0.0.1:{PORT}")
        httpd.serve_forever()
