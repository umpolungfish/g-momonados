"""Render the carrier's measured pairing as nested frames and mark clear sites."""
import json
import os
from pathlib import Path
import re
import subprocess

root = Path(__file__).resolve().parent
workspace = root.parents[2]
os.environ['MPLCONFIGDIR'] = str(root / '.matplotlib')
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle
from matplotlib.font_manager import FontProperties

text = (root / 'active_base_control_91.stderr').read_text()
word = re.search(r'^carrier word=(\S+)', text, re.M).group(1)
run = subprocess.run([str(workspace / 'Vox/target/release/vox'), 'pairs', word],
                     capture_output=True, text=True, check=True, timeout=30)
pairs = [(int(a), int(b)) for a, b in re.findall(r'^(\d+)\s+(\d+)\s+\d+', run.stdout, re.M)]
parents = {pair: min((other for other in pairs if other[0] < pair[0] and pair[1] < other[1]),
                    key=lambda other: other[1]-other[0], default=None) for pair in pairs}
children = {pair: sorted(child for child in pairs if parents[child] == pair) for pair in pairs}
font_path = subprocess.check_output(['fc-match', '-f', '%{file}', 'Everson Mono'], text=True)
font = FontProperties(fname=font_path)

def body(pair):
    start, end = pair
    position = start + 1
    entries = []
    for child in children[pair]:
        if position < child[0]: entries.append(('marks', word[position:child[0]]))
        entries.append(('frame', child))
        position = child[1] + 1
    if position < end: entries.append(('marks', word[position:end]))
    return entries

def height(pair):
    return 1.2 + sum(0.65 if kind == 'marks' else height(value) + 0.2 for kind, value in body(pair))

outer = next(pair for pair in pairs if parents[pair] is None)
total = height(outer)
fig, ax = plt.subplots(figsize=(9, 11))
ax.set_xlim(0, 12)
ax.set_ylim(total + 2, -1.2)
ax.axis('off')
ax.text(0.4, -0.65, 'Source carrier: nested frames and clear sites', fontsize=16)

def marks(line, x, y):
    for index, mark in enumerate(line):
        ax.text(x + index*0.48, y, mark, fontproperties=font, fontsize=17,
                color='#b3261e' if mark == '≺' else '#152c43', va='center')

def draw(pair, depth, y):
    x = 0.5 + depth*0.65
    h = height(pair)
    ax.add_patch(Rectangle((x, y), 10.8-depth*0.65, h, facecolor='none',
                          edgecolor='#2e6f92', linewidth=1.2))
    ax.text(x+0.13, y+0.27, '∈', fontproperties=font, fontsize=16, va='center')
    ax.text(x+0.8, y+0.27, f'cells {pair[0]}–{pair[1]}', fontsize=8, color='#476775', va='center')
    cursor = y + 0.7
    for kind, value in body(pair):
        if kind == 'marks': marks(value, x+0.25, cursor); cursor += 0.65
        else: draw(value, depth+1, cursor-0.15); cursor += height(value)+0.2
    ax.text(x+0.13, y+h-0.25, '∋', fontproperties=font, fontsize=16, va='center')

marks(word[:outer[0]], 0.5, -0.1)
draw(outer, 0, 0.35)
marks(word[outer[1]+1:], 0.5, total+0.9)
ax.text(0.5, total+1.6, 'Red marks locate clears. Boxes follow Vox pairing of this same word.', fontsize=9)
fig.tight_layout()
for extension in ['png', 'svg']:
    fig.savefig(workspace / f'ig-docs/factor_source_carrier_frames.{extension}', dpi=180)
(root / 'source_carrier_pairing.json').write_text(json.dumps(dict(
    operator_word=word, pairs=pairs, instrument='Vox pairs', stdout=run.stdout),
    ensure_ascii=False, indent=2)+'\n')
print('Rendered the measured source-carrier frame hierarchy.')
