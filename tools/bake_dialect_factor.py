#!/usr/bin/env python3
"""Compile the live dialect tables and existing CPU resident into a static image."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import time
import gzip

ROOT = Path(__file__).resolve().parents[1]


def run(argv, log):
    with log.open('wb') as output:
        subprocess.run([str(x) for x in argv], cwd=ROOT, stdout=output,
                       stderr=subprocess.STDOUT, check=True)


def bake(source, output, kernel, keep_fibres, compose_idempotent, hop, product_frame, resume_return=None, resume_verification=None, cofactor_feedback=False, support_newton=False, promotion_return=False, support_secant=False, support_parity=False, support_frame_point=False, environment_carry=False, mixed_environment_operations=False):
    if mixed_environment_operations and not environment_carry:
        raise ValueError("mixed environment operations require retained environment coordinates")
    if environment_carry and not promotion_return:
        raise ValueError("environment carry requires canonical coordinate tables and retained promotion return")
    if support_frame_point and not support_parity:
        raise ValueError("frame-bound support requires the retained support parity operation")
    if support_parity and not support_secant:
        raise ValueError("support parity return requires the two-point support operation")
    if support_secant and not promotion_return:
        raise ValueError("two-point support requires retained promotion transport")
    if promotion_return and not (keep_fibres and product_frame and support_newton):
        raise ValueError("promotion return requires exact fibres, the carried product frame and support Newton")
    if support_newton and not (cofactor_feedback and product_frame):
        raise ValueError("support Newton requires the carried product frame and cofactor feedback")
    started = time.monotonic()
    source_paths=[Path(__file__),ROOT/'src/dialect_factor_arrow.hpp',ROOT/'src/dialect_factor_runner.hpp',
                  ROOT/'src/idempotent_gpu.cu',ROOT/'src/idempotent_cpu_word.hpp',ROOT/'src/dialect_factor_export.rs']
    source_snapshots={p:p.read_bytes() for p in source_paths}
    arrow_source=source_snapshots[ROOT/'src/dialect_factor_arrow.hpp'].decode()
    runner_source=source_snapshots[ROOT/'src/dialect_factor_runner.hpp'].decode()
    source, output, kernel = source.resolve(), output.resolve(), kernel.resolve()
    project = Path(str(output) + '.source')
    project.mkdir(parents=True, exist_ok=False)
    tables_path = project / 'dialect_tables.json'
    run([kernel, 'dialect-factor-export', source, tables_path], project / 'export.log')
    data = json.loads(tables_path.read_text())
    source_word = source.read_text().strip()
    if data['source_word'] != source_word or data['source_bits'] < 200:
        raise ValueError('source binding failed')
    width = data['source_bits'] + 10
    # Recover the certified Y and word arithmetic from the retained construction.
    saved = ROOT / 'measurements/idempotent_baked_20261015/locked_512_sic_zeta_paradox_silent_gpu_r2.membrane-source/membrane.cu'
    cuda = saved.read_text()
    header = cuda[:cuda.index('// Every operand')]
    header = re.sub(r'#define W \d+', f'#define W {width}', header)
    cells = [9 if c == '⊥' else 8 for c in source_word if c in '⊤⊥']
    if len(cells) != data['source_bits']:
        raise ValueError('source numeral packet dimensions disagree')
    cells += [8] * 10
    header = re.sub(r'__constant__ unsigned char SOURCE_ARMS\[\]=\{[^}]+\};',
                    '__constant__ unsigned char SOURCE_ARMS[]={' + ','.join(map(str, cells)) + '};', header)
    arithmetic = cuda[cuda.index('// n is odd, so binary gcd'):cuda.index('// Uses the existing warp-distributed')]
    body = (ROOT / 'src/idempotent_gpu.cu').read_text()
    body = re.sub(r'__device__ void load_word\([\s\S]*?\n}',
                  '__device__ void load_word(const arm *p,Word &w){std::copy(p,p+W,w.v);}', body)
    body = re.sub(r'__device__ void save_word\([\s\S]*?\n}',
                  '__device__ void save_word(const Word &w,arm *p){std::copy(w.v,w.v+W,p);}', body)
    for before, after in [('int i=32*k+lane()', 'int i=k'), ('i+=32', '++i'),
                          ('if(lane()==i%32)', ''), ('if(lane()==bit%32)', ''),
                          ('.v[i/32]', '.v[i]'), ('.v[bit/32]', '.v[bit]')]:
        body = body.replace(before, after)
    base = f'#define W {width}\n' + (ROOT / 'src/idempotent_cpu_word.hpp').read_text() + '\n' + header + arithmetic
    prepare = r'''
int main(){
 std::vector<arm>s(64*14*W,T);std::vector<int>c(64*6146,0);std::vector<unsigned long long>r(650,0);
 for(int i=0;i<Y_NODE_COUNT;++i){c[3*i]=Y_TAG[i];c[3*i+1]=Y_LEFT[i];c[3*i+2]=Y_RIGHT[i];}
 c[6144]=Y_NODE_COUNT;c[6145]=Y_ROOT;
 idempotent_membrane(s.data(),c.data(),r.data(),1,1);
 if(r[3]!=0)return 1;
 auto emit=[](const auto&v,unsigned count){std::cout<<'[';for(unsigned i=0;i<count;++i){if(i)std::cout<<',';std::cout<<+v[i];}std::cout<<']';};
 std::cout<<"{\"state\":";emit(s,14*W);std::cout<<",\"continuation\":";emit(c,6146);
 std::cout<<",\"receipt\":";std::vector<unsigned long long>member(r.begin()+10,r.begin()+20);emit(member,10);std::cout<<"}\n";
}
'''
    (project / 'prepare.cpp').write_text(base + body + prepare)
    flags = ['g++', '-std=c++17', '-O2', '-static', '-Wall', '-Wextra', '-Werror', '-Wno-unknown-pragmas']
    run(flags + [project / 'prepare.cpp', '-o', project / 'prepare'], project / 'prepare_build.log')
    run([project / 'prepare'], project / 'prepared.json')
    prepared = json.loads((project / 'prepared.json').read_text())
    run([project / 'prepare'], project / 'prepared_replay.json')
    if (project / 'prepared.json').read_bytes() != (project / 'prepared_replay.json').read_bytes():
        raise ValueError('independent preparation replay differs')
    if len(prepared['state']) != 14*width or len(prepared['continuation']) != 6146:
        raise ValueError('resident prepared banks have wrong dimensions')
    table = ['struct DialectCell{unsigned address,conflicts;bool gates,time;};',
             f'constexpr unsigned DIALECT_COUNT={len(data["tables"])};',
             f'constexpr unsigned DIALECT_CRYSTAL_RADIX={data["crystal_radix"]};',
             f'constexpr bool DIALECT_KEEP_FIBRES={str(keep_fibres).lower()};',
             f'constexpr bool DIALECT_COMPOSE_IDEMPOTENT={str(compose_idempotent).lower()};',
             f'constexpr bool DIALECT_HOP={str(hop).lower()};',
             f'constexpr bool DIALECT_PRODUCT_FRAME={str(product_frame).lower()};',
             f'constexpr bool DIALECT_COFACTOR_FEEDBACK={str(cofactor_feedback).lower()};',
             f'constexpr bool DIALECT_SUPPORT_SECANT={str(support_secant).lower()};',
             f'constexpr bool DIALECT_SUPPORT_PARITY={str(support_parity).lower()};',
             f'constexpr bool DIALECT_SUPPORT_FRAME_POINT={str(support_frame_point).lower()};',
             f'constexpr bool DIALECT_ENVIRONMENT_CARRY={str(environment_carry).lower()};',
             f'constexpr bool DIALECT_MIXED_ENVIRONMENT={str(mixed_environment_operations).lower()};',
             f'constexpr unsigned DIALECT_INSTRUMENT_SWEEPS={3 if mixed_environment_operations else 2 if environment_carry else 1};',
             f'constexpr bool DIALECT_SUPPORT_NEWTON={str(support_newton).lower()};',
             f'constexpr bool DIALECT_PROMOTION_RETURN={str(promotion_return).lower()};',
             'static const DialectCell DIALECT_ALGEBRA[][3][256]={']
    for dialect in data['tables']:
        table.append('{')
        for operation in dialect['operations']:
            table.append('{' + ','.join('{' + ','.join(str(v).lower() for v in cell) + '}' for cell in operation) + '},')
        table.append('},')
    table.append('};')
    if promotion_return:
        target=data['promotion_target_indices']
        cards=[len(values) for values in data['axis_values']]
        strides=[1]*12
        for i in range(10,-1,-1):strides[i]=strides[i+1]*cards[i+1]
        if sum(a*b for a,b in zip(target,strides))!=data['promotion_target_address']:
            raise ValueError('promotion target Crystal binding differs')
    else:target=[0]*12;cards=[1]*12;strides=[1]*12
    for name,values in [('TARGET',target),('CARDS',cards),('STRIDES',strides)]:
        table.append('static const unsigned DIALECT_PROMOTION_'+name+'[]={'+','.join(map(str,values))+'};')
    table.append('struct DialectAxisCell{unsigned coordinate;bool conflict;};')
    table.append('static const DialectAxisCell DIALECT_ENVIRONMENT_ALGEBRA[][3][12][25]={')
    for dialect in data['tables'] if environment_carry else [None]:
        table.append('{')
        for op in range(3):
            table.append('{')
            for axis in range(12):
                cells=dialect['axis_operations'][op][axis] if dialect else [[0,False]]
                if dialect and (len(cells)!=cards[axis]**2 or any(not 0<=cell[0]<cards[axis] for cell in cells)):
                    raise ValueError('environment primitive table domain differs')
                table.append('{'+','.join('{'+str(cell[0])+','+str(cell[1]).lower()+'}' for cell in cells)+'},')
            table.append('},')
        table.append('},')
    table.append('};')
    for key, ctype in [('state', 'arm'), ('continuation', 'int'), ('receipt', 'unsigned long long')]:
        table.append(f'static const {ctype} PREPARED_DIALECT_{key.upper()}[]={{' + ','.join(map(str, prepared[key])) + '};')
    parent_sha='';parent_hex='';frame_words=[];frame_ready=[]
    if resume_return:
        if not resume_verification:raise ValueError('resume requires the independent return verification')
        parent_raw=resume_return.read_bytes();parent_sha=hashlib.sha256(parent_raw).hexdigest()
        certificate=json.loads(resume_verification.read_text())
        if certificate['receipt_sha256']!=parent_sha:raise ValueError('resume return does not match its verification')
        parent=json.loads(parent_raw)
        if parent['closed'] or parent['source_word']!=source_word:raise ValueError('resume requires the same open source')
        for key,value in [('keep_fibres',keep_fibres),('compose_idempotent',compose_idempotent),('successive_dialect_hops',hop)]:
            if parent.get(key,False)!=value:raise ValueError('resume changed the retained fibre/continuation interpretation')
        members=sorted(parent['members'],key=lambda m:(m['dialect'],m['operation']))
        if len(members)!=3*len(data['tables']):raise ValueError('resume family is incomplete')
        for i,m in enumerate(members):
            if (m['dialect'],m['operation'])!=divmod(i,3) or m['receipt'][3]!=0:raise ValueError('resume family identity/status differs')
            frame=m.get('product_frame_state',{})
            ready=frame.get('initialized',False);frame_ready.append(int(ready))
            if ready:
                words=frame['matrix']+[frame['left'],frame['right'],frame.get('anchor_left',m['state'][3*width:4*width]),frame.get('anchor_right',m['state'][4*width:5*width])]
                if len(words)!=8 or any(len(word)!=width for word in words):raise ValueError('retained product frame dimensions differ')
                frame_words.extend(value for word in words for value in word)
            else:frame_words.extend([8]*(8*width))
        for key,ctype,size in [('state','arm',14*width),('continuation','int',6146),('receipt','unsigned long long',10)]:
            if any(len(m[key])!=size for m in members):raise ValueError('resume bank dimensions changed')
            flat=[value for m in members for value in m[key]]
            table.append(f'static const {ctype} RESUMED_{key.upper()}[]={{'+','.join(map(str,flat))+'};')
            table.append(f'const {ctype} *dialect_initial_{key}(unsigned slot){{return RESUMED_{key.upper()}+slot*{size};}}')
        metadata=[m[k] for m in members for k in ('conflicts','changed','gate_admitted','t_admitted')]
        table.append('static const unsigned long long RESUMED_METADATA[]={'+','.join(map(str,metadata))+'};')
        table.append('const unsigned long long *dialect_initial_metadata(unsigned slot){return RESUMED_METADATA+4*slot;}')
        parent_hex=gzip.compress(parent_raw,compresslevel=6,mtime=0).hex()
        (project/'parent_return.json.gz').write_bytes(bytes.fromhex(parent_hex))
        (project/'parent_verification.json').write_bytes(resume_verification.read_bytes())
    else:
        for key,ctype in [('state','arm'),('continuation','int'),('receipt','unsigned long long')]:
            table.append(f'const {ctype} *dialect_initial_{key}(unsigned){{return PREPARED_DIALECT_{key.upper()};}}')
        table.append('const unsigned long long *dialect_initial_metadata(unsigned){static const unsigned long long values[4]={};return values;}')
    environment_words=[];environment_offsets=[0]
    for member in members if resume_return else [{} for _ in range(3*len(data['tables']))]:
        coordinates=member.get('environment_coordinates',[])
        if coordinates and (not environment_carry or len(coordinates)!=(data['source_bits']+3)//4 or any(not 0<=a<data['crystal_radix'] for a in coordinates)):
            raise ValueError('cannot discard or change retained environment coordinates')
        environment_words.extend(coordinates);environment_offsets.append(len(environment_words))
    table.append('static const unsigned RESUMED_ENVIRONMENT_COORDINATES[]={'+','.join(map(str,environment_words or [0]))+'};')
    table.append('static const unsigned RESUMED_ENVIRONMENT_OFFSETS[]={'+','.join(map(str,environment_offsets))+'};')
    table.append('std::vector<unsigned> dialect_initial_environment(unsigned slot){return {RESUMED_ENVIRONMENT_COORDINATES+RESUMED_ENVIRONMENT_OFFSETS[slot],RESUMED_ENVIRONMENT_COORDINATES+RESUMED_ENVIRONMENT_OFFSETS[slot+1]};}')
    if any(frame_ready):
        table.append('static const arm RESUMED_FRAME_WORDS[]={'+','.join(map(str,frame_words))+'};')
        table.append('static const unsigned char RESUMED_FRAME_READY[]={'+','.join(map(str,frame_ready))+'};')
        table.append(f'const arm *dialect_initial_frame(unsigned slot){{return RESUMED_FRAME_WORDS+slot*{8*width};}}')
        table.append('bool dialect_initial_frame_ready(unsigned slot){return RESUMED_FRAME_READY[slot]!=0;}')
    else:
        table.append('const arm *dialect_initial_frame(unsigned){return nullptr;}')
        table.append('bool dialect_initial_frame_ready(unsigned){return false;}')
    table.append('static const char PARENT_RETURN_SHA256[]='+json.dumps(parent_sha)+';')
    # Embed retained data through the assembler, without making GCC parse one
    # unbounded string literal or millions of numeral-cell initializers.
    # The static image owns the bytes; no runtime file is opened.
    parent_payload=project/'parent_return.hex'
    parent_payload.write_bytes(parent_hex.encode('ascii')+b'\0')
    table.append('extern "C" const char PARENT_RETURN_GZIP_HEX[];')
    # Divisor readout is separated from transport. The transport wrapper only
    # retains witness words; the real source fuse rejects an in-flight call.
    fuse_declaration='__device__ bool fuse_source_arm(const Word &arm_word,const Word &n,'
    assert body.count(fuse_declaration)==1
    body=body.replace(fuse_declaration, 'static unsigned long long dialect_transport_probe_calls=0;\nstatic bool dialect_transport_active=false;\nstatic std::vector<Word> *dialect_pending_arms=nullptr;\n__device__ bool terminal_source_arm(const Word &arm_word,const Word &n,')
    body=body.replace('    Word d,q,r,product;gcd(arm_word,n,d);',
        '    if(dialect_transport_active){++dialect_transport_probe_calls;throw std::runtime_error("source readout attempted during transport");}\n    Word d,q,r,product;gcd(arm_word,n,d);')
    wrapper=r"""
__device__ bool fuse_source_arm(const Word &arm_word,const Word &n,ResidentState &output,arm *paired){
    if(!dialect_transport_active)return terminal_source_arm(arm_word,n,output,paired);
    if(!dialect_pending_arms)throw std::runtime_error("transport witness bank missing");
    dialect_pending_arms->push_back(arm_word);
    return false;
}
"""
    body=body.replace('struct IdempotentArrow {',wrapper+'struct IdempotentArrow {',1)
    final='        if(!fuse_source_arm(e,n,output,paired)||status!=8)'
    assert final in body
    body=body.replace(final, '        if(dialect_transport_active){fuse_source_arm(e,n,output,paired);output.complement=complement;return powerset_unit(coordinate); }\n'+final)
    # Full prior witness banks, if present, resume alongside the other banks.
    pending=[];offsets=[0]
    for m in members if resume_return else [{} for _ in range(3*len(data['tables']))]:
        words=m.get('deferred_source_arms',[])
        if any(len(word)!=width or any(cell not in (8,9) for cell in word) for word in words):
            raise ValueError('invalid deferred source witness bank')
        pending.extend(cell for word in words for cell in word);offsets.append(len(pending)//width)
    deferred_payload=project/'deferred_source_arms.cells'
    deferred_payload.write_bytes(bytes(pending) if pending else bytes([8]))
    table.append('extern "C" const arm RESUMED_DEFERRED_ARMS[];')
    table.append('static const unsigned long long RESUMED_DEFERRED_OFFSETS[]={'+','.join(map(str,offsets))+'};')
    table.append('constexpr bool DIALECT_TERMINAL_READOUT=true;')
    # SIC reconstruction and consistency instruments run at the return boundary.
    body=body.replace('if(!sic_zeta_resident(', 'if(!dialect_transport_active&&!sic_zeta_resident(')
    body=body.replace('++sic_roundtrips;', 'if(!dialect_transport_active)++sic_roundtrips;')
    body=body.replace('if(!member.status){\n            Word square,checked;', 'if(!member.status&&!dialect_transport_active){\n            Word square,checked;')
    marker = '// One Y payload operation'
    first, rest = body.split(marker, 1)
    call = 'IdempotentArrow{n,paired}(0,member,successor);'
    if rest.count(call) != 1:
        raise ValueError('resident Y payload dispatch changed')
    rest = rest.replace(call, 'DialectArrow{n,paired}(0,member,successor);')
    program = base + '\n'.join(table) + first + arrow_source + marker + rest + runner_source
    (project / 'membrane.cpp').write_text(program)
    payload_assembly=project/'retained_payload.S'
    payload_lines=['.section .rodata']
    for symbol,payload in [('PARENT_RETURN_GZIP_HEX',parent_payload),('RESUMED_DEFERRED_ARMS',deferred_payload)]:
        payload_lines.extend([f'.global {symbol}',f'.type {symbol}, @object',
                              f'{symbol}:',f'.incbin {json.dumps(str(payload))}',
                              f'.size {symbol}, .-{symbol}'])
    payload_lines.append('.section .note.GNU-stack,"",@progbits')
    payload_assembly.write_text('\n'.join(payload_lines)+'\n')
    payload_object=project/'retained_payload.o'
    run(['g++','-c',payload_assembly,'-o',payload_object],project/'payload_build.log')
    run(flags + ['-DDIALECT_ONE_SWEEP', project / 'membrane.cpp', payload_object, '-o', project / 'instrument'], project / 'instrument_build.log')
    instrument = subprocess.run([str(project / 'instrument')], cwd=ROOT, capture_output=True, check=False)
    (project / 'instrument.json').write_bytes(instrument.stdout)
    (project / 'instrument.stderr').write_bytes(instrument.stderr)
    if instrument.returncode not in (0, 2):
        raise ValueError(f'dialect instrument failed with status {instrument.returncode}')
    report = json.loads(instrument.stdout)
    if not report['monad_laws'] or report['source_word'] != source_word:
        raise ValueError('resident dialect instrument failed source/law checks')
    run(flags + [project / 'membrane.cpp', payload_object, '-o', output], project / 'build.log')
    run(['readelf', '-l', '-d', output], project / 'linkage.txt')
    linkage = (project / 'linkage.txt').read_text()
    if 'INTERP' in linkage or '(NEEDED)' in linkage:
        raise ValueError('emitted image is not static')
    bound = [source, kernel, saved, output, project / 'membrane.cpp', tables_path,
             ROOT / 'src/idempotent_gpu.cu', ROOT / 'src/idempotent_cpu_word.hpp',
             ROOT / 'src/dialect_factor_export.rs', ROOT / 'src/dialect_factor_arrow.hpp',
             ROOT / 'src/dialect_factor_runner.hpp', Path(__file__)]
    bound.extend([parent_payload,deferred_payload,payload_assembly,payload_object])
    if resume_return:bound.extend([resume_return,resume_verification,project/'parent_return.json.gz'])
    if any(p.read_bytes()!=raw for p,raw in source_snapshots.items()):
        raise ValueError('compiler sources changed during compilation; preserve this image and rebuild from a stable snapshot')
    receipt = {'format': 'dialect-factor-compilation-v1', 'source_bits': data['source_bits'],
               'compilation_seconds': time.monotonic()-started, 'static': True,
               'preparation_replay_equal': True, 'input_boundary': 'N-only',
               'instrument_closed': report['closed'], 'members': len(report['members']),
               'keep_fibres': keep_fibres, 'compose_idempotent': compose_idempotent,
               'successive_dialect_hops': hop,
               'product_frame': product_frame,
               'cofactor_feedback':cofactor_feedback,
               'support_newton':support_newton,
               'support_secant':support_secant,
               'support_parity':support_parity,
               'support_frame_point':support_frame_point,
               'environment_carry':environment_carry,
               'mixed_environment_operations':mixed_environment_operations,
               'promotion_return':promotion_return,
               'source_readout_boundary':'after transport; retained witnesses read without altering family',
               'resumed_from_sha256':parent_sha,
               'retained_payload_storage':'static read-only embedded bytes; IMASM cells unchanged',
               'bindings': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in bound}}
    (project / 'compilation.json').write_text(json.dumps(receipt, indent=2)+'\n')
    print(json.dumps({k:v for k,v in receipt.items() if k != 'bindings'}, indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--kernel', type=Path, required=True,
                        help='the rebuilt kernel executable containing dialect-factor-export')
    parser.add_argument('--keep-fibres', action='store_true', help='tensor exact SIXTEEN labels beside each type coordinate')
    parser.add_argument('--compose-idempotent', action='store_true', help='continue the transformed register through the retained idempotent arrow')
    parser.add_argument('--hop', action='store_true', help='carry each state into the next dialect on every retained Y invocation')
    parser.add_argument('--product-frame', action='store_true', help='apply and return a source-bound two-arm operator and its product residual')
    parser.add_argument('--resume-return',type=Path,help='restore the complete family from its N-only retained return')
    parser.add_argument('--resume-verification',type=Path,help='independent readback binding for that retained return')
    parser.add_argument('--cofactor-feedback',action='store_true',help='update the paired product point from chart parity and the exact N quotient/residual')
    parser.add_argument('--support-newton',action='store_true',help='advance the retained anchor by the N-bit support polynomial and derivative; fuse nonunits and split returns')
    parser.add_argument('--promotion-return',action='store_true',help='apply one kernel promotion axis to the carried exact fibre and retain its return certificate')
    parser.add_argument('--support-secant',action='store_true',help='advance the retained point using the source polynomial divided difference across its promotion displacement')
    parser.add_argument('--support-parity',action='store_true',help='carry the corrected support word through correlated parity arms with exact retained interlace return before quotient feedback')
    parser.add_argument('--support-frame-point',action='store_true',help='apply support correction to the selected transformed joint product-frame coordinate before parity and quotient feedback')
    parser.add_argument('--environment-carry',action='store_true',help='carry full type coordinates between dialect environments and apply each environment algebra to the preceding transformed coordinates')
    parser.add_argument('--mixed-environment-operations',action='store_true',help='cycle meet, join and tensor on the same retained environment tuples using the carried Y invocation index')
    args = parser.parse_args()
    bake(args.source, args.output, args.kernel, args.keep_fibres, args.compose_idempotent, args.hop, args.product_frame,args.resume_return,args.resume_verification,args.cofactor_feedback,args.support_newton,args.promotion_return,args.support_secant,args.support_parity,args.support_frame_point,args.environment_carry,args.mixed_environment_operations)
