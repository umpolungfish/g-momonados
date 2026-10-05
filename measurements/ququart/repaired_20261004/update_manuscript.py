"""Apply observed repaired closures and bounded phase progress to the local manuscript."""
from pathlib import Path
import hashlib,json,subprocess
ROOT=Path(__file__).resolve().parents[3]
RECORD=Path(__file__).resolve().parent
DOCS=Path('/home/mrnob0dy666/imsgct/ig-docs')
commit=subprocess.check_output(['git','-C',str(ROOT),'rev-parse','--short','HEAD'],text=True).strip()
results=json.loads((RECORD/'executions.json').read_text())['results']
closed=next(r for r in results if r['bits']==224)
assert closed['factor_extraction_verified'] and closed['matches_certified_reference']
phase=json.loads((RECORD/'phase_debug_summary.json').read_text())
source=DOCS/'ququart_membranes.tex'
s=source.read_text()
def replace(old,new):
    global s
    assert s.count(old)==1,old[:70]
    s=s.replace(old,new)
replace('A further independently certified ladder closes fresh sources at 128, 160, and 192 bits; its 224-bit execution is stopped at the cutoff in recursive arithmetic allocation.',
        'A further independently certified ladder closes fresh sources at 128, 160, 192, and 224 bits through the native arm. A 256-bit source reaches internally checked phase and SIC readouts in bounded ququart-selected executions. Recursive cofactor caching and support-interval bounds identify concrete traversal boundaries.')
replace('An optional native attempt has a bounded child lifetime and is killed and reaped on expiry, after which execution can continue to the phase arm. Each call is bounded separately, so the cofactor check contributes its own potential native work.',
        'Native child lifetimes are bounded, and expiry kills and reaps the child before the next included arm. The current native schedule includes a direct MPQS entry bounded at sixty-five seconds, followed by the default engine schedule bounded at five seconds. The direct entry skips initial ECM and Rho/SQUFOF stages while retaining final ECM; the full default schedule remains included. The cofactor call has a separate ten-second budget. A canonical preparation selector permits direct inspection of the resident ququart continuation.')
anchor='Vox is applied to the actual prepared ELF executable.'
replace(anchor,r'''Constant addition uses subtraction pullback on output coordinates. If the remaining constant and incoming borrow are zero, each selected bit addresses its original child and propagates zero borrow. Spectator wires remain unchanged, so the resident subtree can return immediately. Nested digit arms share cofactor answers keyed by subtree and source wire. Nodes remain immutable and are reclaimed only after the arithmetic operation returns.

Interval partitioning also checks support bounds inside each recursive suffix. Let $m$ and $M$ bound the remaining register suffix, let $v$ be the remaining threshold, and let $b$ be the borrow from the consumed prefix. The complete suffix lies below the threshold when $M<v$, or when $M=v$ and $b=1$. It lies at or above the threshold when $m>v$, or when $m=v$ and $b=0$. These sufficient conditions return an entire resident arm without further splitting; an uncertain interval continues through the exact transducer.

'''+anchor)
start=s.index(r'\begin{tabular}{lll}',s.index(r'\section*{Increasing certified source widths}'))
end=s.index(r'\end{tabular}',start)+len(r'\end{tabular}')
rows=['128 & 4 & 0.59 & Verified native closure\\\\','160 & 4 & 0.80 & Verified native closure\\\\','192 & 4 & 2.51 & Verified native closure\\\\',f"224 & 4 & {closed['elapsed_seconds']:.2f} & Verified native closure\\\\"]
for r in phase['selected_ququart_runs']:
    rows.append(f"256 & {r['work_radix']} & {r['elapsed_seconds']:.2f} & {r['completed_prior_readouts_inferred']} readouts; stopped\\\\")
s=s[:start]+ '\n'.join([r'\begin{tabular}{llll}',r'\toprule',r'Width (bits) & Work radix & Elapsed (s) & Outcome\\',r'\midrule',*rows,r'\bottomrule',r'\end{tabular}'])+s[end:]
s=s.replace('The three completed results pass source-bound', 'The completed factor results pass source-bound')
old='The 224-bit execution is interrupted and its inferior killed within the ninety-second closure window. Its stopped stack reaches the allocator through a fixed-support bitset clone, decision-node interning, and recursive constant addition. The process mappings relocate those live addresses to the actual executable, and Vox completely decodes the corresponding symbol regions. This observation identifies the arithmetic allocation path active at the cutoff. A generated 256-bit source remains queued after the ladder stops at this first unclosed execution.'
new=r'''The retained 224-bit native-to-phase handoff identifies expiry of the original ten-second child budget. The completed 224-bit case retains a seventy-second default-schedule source budget and a separate cofactor budget. Its returned factor words match the independently certified pair. The exact acceptance gate remains common to all producers.

The 256-bit inclusive attempts exhaust their native budgets and reach the ququart continuation. Debugger markers at successive controlled multipliers establish phase progress: the executed shot loop advances only after the preceding phase measurement, Gram/SIC validation, phase accumulation, and reset return successfully. The reported readout counts are inferred from that executed control flow and its complete Vox disassembly. They do not constitute a terminal factor report or an independent verification of a complete phase ledger.

Direct ququart inspection selects the phase arm with a canonical preparation word while retaining the native code. Its ninety-second cutoff captures the full recursive stack. Equal-source work-radix variants reuse the exact source-bound Fourier contraction and regenerate scaled modular powers and work operations. The listed runs are individual observations; they do not establish a statistical radix-speed comparison. No 256-bit factor report is released in these retained executions.'''
replace(old,new)
anchor=r'The local software snapshot is G-mOMonadOS commit \texttt{00351b0}.'
replace(anchor,anchor+r' The supplementary arithmetic and phase-debug records are retained in commit \texttt{'+commit+'}, identified in the companion evidence manifest.')
source.write_text(s)
p=DOCS/'ququart_membranes.evidence.json';e=json.loads(p.read_text())
e.update(source_ladder_repair_commit=commit,source_ladder_repair=json.loads((RECORD/'executions.json').read_text()),
         source_ladder_diagnosis=json.loads((RECORD/'diagnosis.json').read_text()),phase_debug=phase,
         manuscript_source_sha256=hashlib.sha256(source.read_bytes()).hexdigest())
p.write_text(json.dumps(e,ensure_ascii=False,indent=2)+'\n')
