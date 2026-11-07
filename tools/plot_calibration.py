# Gráficos da calibração: uma linha por faixa (energia, componentes, batidas, seções, drops).
# Uso: python tools/plot_calibration.py corpus/_out [saida.png] [ids separados por vírgula]
import json, sys, os, glob
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
d = sys.argv[1]
out = sys.argv[2] if len(sys.argv) > 2 else os.path.join(d, 'calibracao.png')
ids = sys.argv[3].split(',') if len(sys.argv) > 3 else None
files = [f for f in sorted(glob.glob(os.path.join(d, '*.json'))) if not f.endswith('summary.json')]
if ids: files = [f for f in files if any(i in os.path.basename(f) for i in ids)]
fig, axs = plt.subplots(len(files), 1, figsize=(15, 2.3 * len(files)), squeeze=False)
for ax, f in zip(axs[:, 0], files):
    j = json.load(open(f, encoding='utf-8'))
    n = len(j['energy']); t = [k / j['fps'] for k in range(n)]
    ax.plot(t, j['raw'], color='#aaa', lw=0.8, label='energia bruta')
    ax.plot(t, j['D']['loud'], color='#36c', lw=0.6, alpha=0.6, label='volume')
    ax.plot(t, j['beatConf'], color='#2a2', lw=0.8, alpha=0.8, label='clareza do pulso')
    if 'cont' in j['D']: ax.plot(t, j['D']['cont'], color='#c80', lw=0.6, alpha=0.6, label='continuidade')
    ax.plot(t, j['build'], color='#a3c', lw=1.2, label='subida')
    ax.plot(t, j['energy'], color='#d22', lw=2, label='energia')
    if 'live' in j: ax.plot(j['live']['t'], j['live']['E'], color='#f90', lw=1, ls='--', label='energia ao vivo')
    for s in j['sections'][1:]: ax.axvline(s['t0'], color='#888', ls=':', lw=0.8)
    for dr in j['drops']: ax.axvline(dr['t'], color='k', lw=1 + 2 * dr['mag'])
    bt = [b['t'] for b in j['beats'] if b['strength'] > 0.05]
    ax.vlines(bt, -0.12, -0.04, color='#2a2', lw=0.3)
    db = [b['t'] for b in j['beats'] if b['downbeat'] and b['strength'] > 0.05]
    ax.vlines(db, -0.14, -0.02, color='#060', lw=0.8)
    ax.axhline(0.45, ls=':', c='#ccc', lw=0.6); ax.axhline(0.75, ls=':', c='#ccc', lw=0.6)
    ax.set_ylim(-0.15, 1.05); ax.set_xlim(0, j['duration'])
    ax.set_title(f"{j['genre']}  |  {j['bpm']:.0f} BPM  |  clareza {j['beatConfidence']:.2f}", fontsize=9, loc='left')
    ax.tick_params(labelsize=7)
axs[0, 0].legend(ncol=8, fontsize=7, loc='upper right')
plt.tight_layout(); plt.savefig(out, dpi=72)
print(out)
