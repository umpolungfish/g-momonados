"""Measure exported coordinate shapes without interpreting native-fold accuracy."""
from pathlib import Path
import json,math,csv
OUT=Path(__file__).resolve().parent;ROOT=OUT.parents[2]
DATA=ROOT/'red-hot_rebis/measurements/global_protein_coverage_20261020'
manifest=json.loads((DATA/'manifest.json').read_text());rows=[]
for entry in manifest['entries']:
    a=entry['accession'];p=OUT/a/'vox_protein.pdb'
    points=[tuple(float(line[i:i+8]) for i in (30,38,46)) for line in p.read_text().splitlines() if line.startswith('ATOM') and line[12:16].strip()=='CA']
    n=len(points);assert n==entry['length']
    center=tuple(sum(p[d] for p in points)/n for d in range(3))
    dist=lambda a,b:math.sqrt(sum((x-y)**2 for x,y in zip(a,b)))
    rg=math.sqrt(sum(dist(p,center)**2 for p in points)/n)
    contour=sum(dist(a,b) for a,b in zip(points,points[1:]))
    spans=[max(p[d] for p in points)-min(p[d] for p in points) for d in range(3)]
    max_distance=max(dist(a,b) for i,a in enumerate(points) for b in points[i+1:])
    # Use only emitted contact observations; count realized CA proximity separately.
    carrier=json.loads(Path(str(p)+'.glut.json').read_text())['protein_glut']
    observations=[o for o in carrier['observations'] if 'contact' in o['name']]
    contact_rows=[row for o in observations if isinstance(o['values'],list) for row in o['values'] if 'left' in row and 'right' in row]
    contact_distances=[dist(points[c['left']],points[c['right']]) for c in contact_rows]
    rows.append({'emitted_contacts':len(contact_distances),'emitted_contacts_within_8A':sum(d<=8.0 for d in contact_distances),'contact_distance_cutoff_A':8.0,'accession':a,'name':entry['name'],'residues':n,'radius_of_gyration_A':rg,
                 'end_to_end_A':dist(points[0],points[-1]),'max_CA_distance_A':max_distance,
                 'CA_contour_A':contour,'bbox_spans_A':spans,'contact_observation_names':[o['name'] for o in observations]})
(OUT/'shape_audit.json').write_text(json.dumps({'cases':rows,'scope':'CA-coordinate extent only; no reference structure alignment or native-fold accuracy assessment.'},indent=2)+'\n')
for a in ['P69905','P02144','P00698','P62805','P62979','P29972','P09493','P02452']:
    r=next(r for r in rows if r['accession']==a)
    print('contacts within 8 A:',r['emitted_contacts_within_8A'],'/',r['emitted_contacts']);print(a,r['name'],r['residues'],'residues; Rg',round(r['radius_of_gyration_A'],1),'A; end-to-end',round(r['end_to_end_A'],1),'A; max CA distance',round(r['max_CA_distance_A'],1),'A',flush=True)
