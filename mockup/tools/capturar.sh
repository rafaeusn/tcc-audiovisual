#!/bin/bash
# Captura uma tela da prévia em PNG com o Edge sem janela (o servidor harness/serve.py precisa estar rodando).
# Uso: bash mockup/tools/capturar.sh nome "f=Main.dc.html&momento=44&estatico=true" 1440 900
# Saída: mockup/.capturas/nome.png
# Abaixo de 500 px o Edge sem janela não encolhe a página; para 390 px use harness/frame.html
# (troque preview.html por frame.html na URL e capture numa janela de 600x900).
set -e
DIR="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$DIR/.capturas"
mkdir -p "$OUT"
EDGE="${EDGE:-/c/Program Files (x86)/Microsoft/Edge/Application/msedge.exe}"
PAGE="${PAGE:-preview.html}"
"$EDGE" --headless=new --use-angle=swiftshader --enable-unsafe-swiftshader --hide-scrollbars \
  --user-data-dir="$OUT/.perfil" --window-size="$3,$4" --virtual-time-budget="${VTB:-6000}" \
  --screenshot="$OUT/$1.png" "http://127.0.0.1:5510/harness/$PAGE?$2" >/dev/null 2>&1
echo "$OUT/$1.png"
