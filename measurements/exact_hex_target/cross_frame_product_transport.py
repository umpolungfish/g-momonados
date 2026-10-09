"""Share alternative positioned-product returns across retained source framings."""
from pathlib import Path
import argparse
import ast
import gzip
import hashlib
import json
import re
import subprocess
import time

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
EMPTY, FILLED = '≻⋈∈⊤∋', '≻⋈∈⊥∋'
ONE = (0,)
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('parent', type=Path)
args = parser.parse_args()
parent = args.parent.resolve()
assert parent.parent == HERE
revision = 1
while (HERE/f'cross_frame_product_transport_{revision}.records.jsonl.gz').exists():
    revision += 1
name = f'cross_frame_product_transport_{revision}'
native_log = gzip.open(HERE/(name+'.native.log.gz'), 'wt')
records = gzip.open(HERE/(name+'.records.jsonl.gz'), 'wt')
started = time.monotonic()
stats = dict(component_products=0, component_words=0, component_fold_tests=0,
             fold_joins=0, overlay_joins=0, cross_width_comparisons=0,
             cross_width_exact_returns=0)
word_ids, catalog, source_returns = {}, {}, {}

# Reuse the checked cell arithmetic and certificate gate without running its CLI.
instrument = HERE/'cross_frame_collection.py'
names = {'emit','status','call','field','word','native_support','width','dense_signed',
         'combine','add','mul','wid','product_certificate','equation_certificate',
         'register','known_factors','return_source','join_fold','folded','kernel',
         'complete_fold','fold_closes','structural_returns'}
tree = ast.parse(instrument.read_text())
nodes = [node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name in names]
assert {node.name for node in nodes} == names
exec(compile(ast.Module(body=nodes, type_ignores=[]), str(instrument), 'exec'), globals())
tree = ast.parse((HERE/'placement_full_return.py').read_text())
node = next(node for node in tree.body if isinstance(node, ast.FunctionDef) and node.name == 'frame')
exec(compile(ast.Module(body=[node], type_ignores=[]), 'placement_full_return.py', 'exec'), globals())


def enrich(source, rows):
    classes, transports = {}, []
    for w, epsilon, tag, k, q, sign, r in rows:
        product = mul(k, q)
        key = (sign if r else 0, r)
        if key in classes:
            assert classes[key]['product'] == product
        else:
            classes[key] = dict(product=product, returns=[], rows=[])
        path = dict(operation='positioned-fold-product', width=w, epsilon=epsilon,
                    form=tag, correction_sign=sign, correction=wid(r))
        classes[key]['returns'].append(path)
        classes[key]['rows'].append((w,epsilon,tag,k,q,sign,r))
        register(product, k, q, path)
        transports.append((product,k,q,path))
    # A component of either operand is also a component of its positioned product.
    # Read both orientations; preserve the complementary component as well.
    for product,k,q,path in transports:
        for operand,other,side in ((k,q,'placement'),(q,k,'quotient')):
            for d,(cofactor,child_path) in known_factors(operand).items():
                register(product,d,mul(cofactor,other),
                         dict(operation='component-through-positioned-product', parent=path,
                              side=side, child=child_path))
    connected = 0
    derived_rows, checked = [], set()
    for (sign,r),group in classes.items():
        widths = {path['width'] for path in group['returns']}
        if len(widths) > 1:
            connected += 1
            emit('equal_positioned_product', product=wid(group['product']),
                 correction_sign=sign, correction=wid(r), returns=group['returns'])
            # Equal product returns connect an observed placement in one framing
            # to the other framing's quotient. The native return retains either
            # its exact quotient or the nonzero remainder of that comparison.
            for w,epsilon,tag,k,q,rsign,residual in group['rows']:
                for v,_,_,other_k,_,_,_ in group['rows']:
                    if w == v:
                        continue
                    for d in known_factors(other_k):
                        if (q,d) in checked:
                            continue
                        checked.add((q,d))
                        out=frame(word(q),'divmod',word(d))
                        quotient=native_support(field(out,'result'))
                        remainder=native_support(field(out,'remainder'))
                        equation_certificate(q,d,quotient,1 if remainder else 0,remainder)
                        stats['cross_width_comparisons'] += 1
                        path=dict(operation='equal-product-quotient-transport',widths=[w,v],
                                  dividend=wid(q),observed_component=wid(d),
                                  quotient=wid(quotient),remainder=wid(remainder))
                        emit('cross_width_quotient_return',**path)
                        if remainder:
                            continue
                        stats['cross_width_exact_returns'] += 1
                        register(q,d,quotient,path)
                        joined=mul(k,d)
                        register(group['product'],joined,quotient,path)
                        equation_certificate(source,joined,quotient,rsign,residual)
                        derived_rows.append((w,epsilon,'cross-width-transport',joined,
                                             quotient,rsign,residual))
                        structural_returns(joined)
                        structural_returns(quotient)
    for w, epsilon, tag, k, q, sign, r in rows+derived_rows:
        join_fold(source,k,q,sign,r,dict(operation='transported-product-collection',
                                        width=w,epsilon=epsilon,form=tag))
    return dict(distinct_positioned_products=len(classes),cross_width_classes=connected,
                transported_source_equations=len(derived_rows))


def replay_overlay(source, row):
    terms, common = [], None
    for group in row['groups']:
        payload = parent_words[group['payload']]
        placement = tuple(group['starts'])
        positioned = tuple(sorted(i+s for s in placement for i in payload))
        options = known_factors(positioned)
        for operand,other in ((payload,placement),(placement,payload)):
            for d,(cofactor,_) in known_factors(operand).items():
                if d not in options:
                    options[d] = (None,(cofactor,other))
        options.pop(ONE,None)
        terms.append(options)
        common = set(options) if common is None else common & options.keys()
    proper = []
    for d in common or ():
        cofactor = ()
        for options in terms:
            c,path = options[d]
            cofactor = add(cofactor,c if c is not None else mul(*path))
        stats['overlay_joins'] += 1
        if return_source(source,d,cofactor,dict(operation='transported-overlay-collection',
                                               widths=row['widths'])):
            proper.append(wid(d))
    emit('overlay_replay', widths=row['widths'],
         common_components=[wid(d) for d in common or ()], proper_components=proper)


try:
    emit('contract', parent=parent.name, parent_sha256=hashlib.sha256(parent.read_bytes()).hexdigest(),
         arithmetic_sha256=hashlib.sha256(instrument.read_bytes()).hexdigest(),
         operation='Equal signed corrections identify equal positioned products; transport returned operand components across those products and collect.')
    # Controls include an exact nonzero-correction common-component collection.
    source = native_support('117')
    a,b,r = native_support('33'),native_support('3'),native_support('18')
    register(a,b,native_support('11'),dict(operation='control-component'))
    register(r,b,native_support('6'),dict(operation='control-component'))
    equation_certificate(source,a,b,1,r)
    product=mul(a,b)
    register(product,a,b,dict(operation='control-positioned-product'))
    join_fold(source,a,b,1,r,dict(operation='control-shared-component'))
    assert source_returns
    catalog.clear()
    source_returns.clear()
    stats.update({key:0 for key in stats})
    parent_words, root_rows = {}, []
    target_seen = False
    parent_summary = None
    with gzip.open(parent,'rt') as stream:
        for line in stream:
            row = json.loads(line)
            if row['kind'] == 'word':
                parent_words[row['id']] = tuple(row['support'])
            elif row['kind'] == 'target':
                target_seen = True
                source = parent_words[row['source']]
                assert word(source) == row['source_word']
            elif target_seen and row['kind'] == 'component_product':
                value,a,b = (parent_words[row[key]] for key in ('source','left','right'))
                path = dict(operation='parent-certified-component',parent=parent.name,
                            source_id=row['source'],left_id=row['left'],right_id=row['right'])
                catalog.setdefault(value,{})[a] = (b,path)
                catalog[value][b] = (a,path)
            elif target_seen and row['kind'] == 'root_fold':
                k = parent_words[row['placement']]
                for tag in ('original','collected'):
                    shape = row[tag]
                    root_rows.append((row['width'],row['epsilon'],tag,k,
                                      parent_words[shape['quotient']],shape['sign'],
                                      parent_words[shape['residual']]))
            elif row['kind'] == 'summary':
                parent_summary = row
    assert target_seen and parent_summary is not None
    assert len(root_rows) == 4*(width(source)-1)
    assert source == native_support((HERE/'source.txt').read_text().strip())
    emit('target', source=wid(source), source_word=word(source))
    status('parent_loaded', component_words=len(catalog), root_equations=len(root_rows))
    classes = enrich(source,root_rows)
    status('product_transport_complete',**classes,**stats,
           proper_source_products=len(source_returns))
    replayed = 0
    # Reuse the parent's lossless fragment groups instead of regenerating cuts.
    with gzip.open(parent,'rt') as stream:
        target_seen = False
        for line in stream:
            row=json.loads(line)
            if row['kind'] == 'target':
                target_seen=True
            elif target_seen and row['kind'] == 'overlay':
                replay_overlay(source,row)
                replayed += 1
                if replayed % 20000 == 0:
                    status('overlay_replay',pairs=replayed)
    assert replayed == (width(source)-1)*(width(source)-2)//2
    summary=dict(**classes,**stats,overlay_pairs=replayed,
                 proper_source_products=len(source_returns),
                 factor_extraction_evidence='True' if source_returns else 'None',
                 elapsed_seconds=round(time.monotonic()-started,3))
    emit('summary',**summary)
    (HERE/(name+'.md')).write_text('# Cross-frame positioned-product transport\n\n'
        +'Parent: `'+parent.name+'`.\n\n'
        +'Complete component paths and cross-width classes: `'+name+'.records.jsonl.gz`. '
        +'Native certificates: `'+name+'.native.log.gz`.\n\n'
        +'```json\n'+json.dumps(summary,indent=2)+'\n```\n')
    status('complete',**summary)
except BaseException as error:
    emit('interrupted',error=repr(error),stats=stats)
    raise
finally:
    records.close()
    native_log.close()
