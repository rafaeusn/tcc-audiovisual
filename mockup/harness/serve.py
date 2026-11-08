"""Servidor local da prévia do mockup.

Serve a pasta mockup/ sem cache e responde /_blob/<id> com as capas de assets/covers,
do mesmo jeito que o artefato do claude.ai entrega os arquivos enviados.

Uso (na raiz do repositório):  python mockup/harness/serve.py
Depois abra: http://127.0.0.1:5510/harness/preview.html?f=Main.dc.html
"""
import http.server
import pathlib

ROOT = pathlib.Path(__file__).resolve().parent.parent
COVERS = ROOT / 'assets' / 'covers'
PORT = 5510

# id do arquivo no artefato -> nome da capa em assets/covers
BLOBS = {
    '17613e56726a02a4b36f3b70361bc8f5': 'silk', '2cb9540638c85e386799867991ef5c51': 'flow', '661b0f7026e786c1c43401a0e6d62cca': 'contour',
    '34c3ad39e324c89a98d2de5b472c0e94': 'halftone', '19cf1c50aa183a2ab64961ca9302c51f': 'glow', 'f27c00f2d027013e5f731c2af051fe12': 'sphere',
    '54a9efb67d74db525f3412c600f2d1ed': 'moire', '4d5fdabd2bbc015a9673fbe10d7025b0': 'dither', 'f8707a86d958127b4899ab522fdd5c25': 'bars',
    '796de4d71cf70eea38fe5327a53746b3': 'warpgrid', 'c1a440d1a082a78e0d91701d6b325098': 'scan', 'c9f074f4fce36ddffa15862fee772518': 'orbit',
}


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k):
        super().__init__(*a, directory=str(ROOT), **k)

    def do_GET(self):
        if self.path.startswith('/_blob/'):
            name = BLOBS.get(self.path.split('/')[-1].split('?')[0])
            if name:
                data = (COVERS / f'{name}.jpg').read_bytes()
                self.send_response(200)
                self.send_header('Content-Type', 'image/jpeg')
                self.send_header('Content-Length', str(len(data)))
                self.end_headers()
                self.wfile.write(data)
                return
            self.send_error(404)
            return
        super().do_GET()

    def end_headers(self):
        self.send_header('Cache-Control', 'no-store')
        super().end_headers()

    def log_message(self, *a):
        pass


if __name__ == '__main__':
    print(f'Prévia do mockup em http://127.0.0.1:{PORT}/harness/preview.html?f=Main.dc.html')
    http.server.ThreadingHTTPServer(('127.0.0.1', PORT), Handler).serve_forever()
