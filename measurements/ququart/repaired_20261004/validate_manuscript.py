"""Check rendered canonical numeral words and the repaired closure table."""
from pathlib import Path
import hashlib,json,subprocess
DOCS=Path('/home/mrnob0dy666/imsgct/ig-docs')
RECORD=Path(__file__).resolve().parent
pdf=DOCS/'ququart_membranes.pdf'
rendered=subprocess.check_output(['pdftotext','-layout',str(pdf),'-'],text=True)
(RECORD/'manuscript_rendered.txt').write_text(rendered)
flat=''.join(rendered.split())
evidence=json.loads((DOCS/'ququart_membranes.evidence.json').read_text())
words=[evidence['cases'][0][key] for key in ['source_word','p_word','q_word']]
assert all(word in flat for word in words)
assert 'Ququartic Computational Membranes' in rendered
assert '??' not in rendered
for entry in evidence['source_ladder_repair']['results']:
    assert str(entry['bits']) in rendered
    assert f"{entry['elapsed_seconds']:.2f}" in rendered
source_digest=hashlib.sha256((DOCS/'ququart_membranes.tex').read_bytes()).hexdigest()
assert source_digest==evidence['manuscript_source_sha256']
assert pdf.read_bytes()==(DOCS/'001_PDFS/ququart_membranes.pdf').read_bytes()
validation=json.loads((DOCS/'ququart_membranes.validation.json').read_text())
validation.update(pdf_sha256=hashlib.sha256(pdf.read_bytes()).hexdigest(),source_ladder_repair_present=True,
                  source_and_factor_words_recovered_exactly_from_pdf=True,source_sha256_matches_evidence=True)
(DOCS/'ququart_membranes.validation.json').write_text(json.dumps(validation,indent=2)+'\n')
(RECORD/'manuscript_validation.json').write_text(json.dumps(validation,indent=2)+'\n')
print('Rendered numeral words and repaired source results verified')
