"""Independent source inverse-product deduction checks after production."""
import json
from verify_dialect_factor import numeral


def least(domains,lower):
    """Least allowed numeral >= lower, by ordered prefix backtracking."""
    width=len(domains)
    if lower<0 or lower.bit_length()>width:return None
    if any(d==0 for d in domains):return None
    ones=sum(1<<i for i,d in enumerate(domains) if d==1)
    prefix=0;free=[]
    for bit in range(width-1,-1,-1):
        domain=domains[bit];value=(lower>>bit)&1
        if value==0:
            if domain&2:
                if domain&1:free.append(bit)
            else:
                return prefix|(1<<bit)|(ones&((1<<bit)-1))
        elif domain&1:prefix|=1<<bit
        else:
            if not free:return None
            higher=free[-1]
            return (prefix&~((1<<(higher+1))-1))|(1<<higher)|(ones&((1<<higher)-1))
    return prefix


def audit(directory,report):
    saved=report.get('source_product_return')
    if saved is None:return {}
    source=numeral(report['source_word']);width=source.bit_length();deductions=0;returns=0
    prepared=json.loads((directory/'product.prepared.json').read_text())
    if saved['banks'][:len(prepared['banks'])]!=prepared['banks']:
        raise ValueError('source inverse-product preparation banks changed')
    if saved['visits']!=len(saved['banks'])-len(prepared['banks']):
        raise ValueError('source inverse-product visit ancestry changed')
    for item in saved['banks']:
        bank=item['bank']
        if bank['source_word']!=report['source_word'] or item['phase']>report['bridge_phase']:
            raise ValueError('source inverse-product bank changed source/phase')
        p,q=item['p_before'],item['q_before']
        if len(p)!=width or len(q)!=width or any(v not in (1,2,3) for v in p+q):
            raise ValueError('source inverse-product operands lost their lane supports')
        bounds=[]
        for domains in (p,q):
            bounds.append((max(3,sum(1<<i for i,d in enumerate(domains) if d==1)),
                           sum(1<<i for i,d in enumerate(domains) if d&1)))
        (pmin,pmax),(qmin,qmax)=bounds
        if pmin>pmax or qmin>qmax:
            if 'contradiction' not in bank:raise ValueError('invalid proper-lane contradiction bank')
            continue
        expected=[pmin,pmax,qmin,qmax]
        if [numeral(bank[k]) for k in ['p_min','p_max','q_min','q_max']]!=expected:
            raise ValueError('source inverse-product operands differ from retained domains')
        for entry,denominator in zip(bank['quotient_returns'],[qmax,qmin,pmax,pmin]):
            d,f,r=[numeral(entry[k]) for k in ['denominator','floor','remainder']]
            if d!=denominator or d*f+r!=source or not 0<=r<d:
                raise ValueError('source inverse-product quotient/remainder changed')
            returns+=1
        intervals=[(max(pmin,(source+qmax-1)//qmax),min(pmax,source//qmin)),
                   (max(qmin,(source+pmax-1)//pmax),min(qmax,source//pmin))]
        for lane,domains,(lower,upper) in zip(('p','q'),(p,q),intervals):
            if (numeral(bank[lane+'_lower']),numeral(bank[lane+'_upper']))!=(lower,upper):
                raise ValueError('source inverse-product interval differs')
            returned=item[lane+'_after']
            if len(returned)!=width:raise ValueError('source inverse-product return width changed')
            for bit,(before,after) in enumerate(zip(domains,returned)):
                if after&~before:raise ValueError('source inverse-product return expanded a lane')
                for removed in (1,2):
                    if before&removed and not after&removed:
                        test=domains[:];test[bit]=removed
                        witness=least(test,lower)
                        if witness is not None and witness<=upper:
                            raise ValueError('source inverse-product deduction removed a possible lane value')
                        deductions+=1
    return {'source_product_banks_checked':len(saved['banks']),
            'native_source_quotient_returns_checked':returns,
            'source_product_removed_alternatives_checked':deductions}
