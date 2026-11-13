"""Gera as fontes estáticas que o app embute, a partir das fontes variáveis do Google Fonts.

O GPUI escolhe a fonte por família + peso; ele não mexe nos eixos de uma fonte variável (largura, por exemplo).
Por isso cada combinação usada na interface vira um arquivo estático:

- Instrument Sans 400/500/600/700: o texto da interface.
- Instrument Sans Titulo: largura 80% e peso 700, registrada como família própria de peso normal, para os
  títulos (no mockup: font-stretch 78% a 88%).
- Doto: peso 800, registrada como peso normal, para os números em matriz de pontos (BPM, relógio).
- Fragment Mono já é estática.

Uso (precisa de `pip install fonttools`):
    cd app/recursos/fontes
    curl -LO "https://raw.githubusercontent.com/google/fonts/main/ofl/instrumentsans/InstrumentSans%5Bwdth%2Cwght%5D.ttf"
    curl -LO "https://raw.githubusercontent.com/google/fonts/main/ofl/doto/Doto%5BROND%2Cwght%5D.ttf"
    python gerar.py "InstrumentSans[wdth,wght].ttf" "Doto[ROND,wght].ttf"

Licença das fontes: SIL Open Font License 1.1 (arquivos OFL-*.txt nesta pasta).
"""

import sys

from fontTools.ttLib import TTFont
from fontTools.varLib import instancer


def renomear(fonte, familia, estilo="Regular", peso=400):
    """Registra a fonte como `familia` + `estilo`, para o sistema achar por família e peso sem ambiguidade."""
    nomes = fonte["name"]
    for nid in (1, 2, 3, 4, 6, 16, 17, 21, 22, 25):
        nomes.removeNames(nameID=nid)
    completo = familia if estilo == "Regular" else f"{familia} {estilo}"
    if estilo in ("Regular", "Bold"):
        registros = [(1, familia), (2, estilo)]
    else:
        # Pesos fora de Regular/Bold: família "de compatibilidade" própria e a família tipográfica em 16/17.
        registros = [(1, completo), (2, "Regular"), (16, familia), (17, estilo)]
    registros += [(3, f"{completo};tcc-player"), (4, completo), (6, completo.replace(" ", ""))]
    for nid, texto in registros:
        nomes.setName(texto, nid, 3, 1, 0x409)
        nomes.setName(texto, nid, 1, 0, 0)
    os2 = fonte["OS/2"]
    os2.usWeightClass = peso
    os2.usWidthClass = 5
    negrito = estilo == "Bold"
    os2.fsSelection = (os2.fsSelection & ~0b1100001) | (0b100000 if negrito else 0b1000000)
    fonte["head"].macStyle = 1 if negrito else 0
    if "STAT" in fonte:
        del fonte["STAT"]


def main(instrument, doto):
    for nome, peso in (("Regular", 400), ("Medium", 500), ("SemiBold", 600), ("Bold", 700)):
        fonte = instancer.instantiateVariableFont(TTFont(instrument), {"wght": peso, "wdth": 100})
        renomear(fonte, "Instrument Sans", nome, peso)
        fonte.save(f"InstrumentSans-{nome}.ttf")

    fonte = instancer.instantiateVariableFont(TTFont(instrument), {"wght": 700, "wdth": 80})
    renomear(fonte, "Instrument Sans Titulo")
    fonte.save("InstrumentSans-Titulo.ttf")

    fonte = instancer.instantiateVariableFont(TTFont(doto), {"wght": 800, "ROND": 0})
    renomear(fonte, "Doto")
    fonte.save("Doto-ExtraBold.ttf")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
