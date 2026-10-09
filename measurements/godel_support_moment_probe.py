"""Check positional support identities against the native contribution merge."""
from godel_markdown_records import read_record, write_record
import json
from pathlib import Path


def support(x):
    return [i for i in range(x.bit_length()) if x >> i & 1]


def check(p, q):
    n = p*q
    a, b, c = support(p), support(q), support(n)
    raw = [0]*n.bit_length()
    for i in a:
        for j in b:
            raw[i+j] += 1
    transfers = []
    incoming = 0
    for i, count in enumerate(raw):
        assert (count+incoming) % 2 == (n >> i & 1)
        outgoing = (count+incoming)//2
        assert count-(n >> i & 1) == 2*outgoing-incoming
        transfers.append(outgoing)
        incoming = outgoing
    assert incoming == 0
    units = sum(transfers)
    positional = sum(i*t for i, t in enumerate(transfers))
    weighted = sum(t << i for i, t in enumerate(transfers))
    moment = lambda x: sum(i*(1 << i) for i in support(x))
    assert len(a)*len(b)-len(c) == units
    assert len(b)*sum(a)+len(a)*sum(b)-sum(c) == positional-units
    assert moment(n)-q*moment(p)-p*moment(q) == 2*weighted
    return dict(p=str(p), q=str(q), source=str(n), transferUnits=str(units),
                transferPositionMoment=str(positional), transferWeightedValue=str(weighted),
                sourceSupportMoment=str(moment(n)), rawSupportMoment=str(q*moment(p)+p*moment(q)),
                weightedMomentDeficit=str(2*weighted))


if __name__ == '__main__':
    pairs = [(3,7), (5,7), (7,13), (11,13), (13,17), (83,97),
             (37975227936943673922808872755445627854565536638199,
              40094690950920881030683735292761468389214899724061)]
    rows = [check(p, q) for p, q in pairs]
    # Fixed supplied operands cover sparse, dense, overlapping, and even products.
    checks = sum(bool(check(p,q)) for p in range(2,81) for q in range(2,81))
    report = dict(polynomialIdentity='A(x)*B(x)-N(x)=(2-x)*T(x); T coefficients are outgoing transfers',
                  countIdentity='popcount(p)*popcount(q)-popcount(n)=T(1)',
                  positionIdentity='popcount(q)*sum(support(p))+popcount(p)*sum(support(q))-sum(support(n))=T_prime(1)-T(1)',
                  weightedIdentity='M(n)-q*M(p)-p*M(q)=2*T(2); M(x)=sum i*2^i over occupied positions',
                  factorShapeConstraint='M(p)/p + M(q)/q <= M(n)/n; equality iff no contribution transfers',
                  rows=rows, additionalKnownOperandChecks=checks,
                  extractionEvidence='None: identities constrain unknown shapes but do not select their supports')
    output = Path(__file__).with_name('godel_support_moment_relationships')
    write_record(output, report)
    print(json.dumps(dict(knownTriples=len(rows), additionalKnownOperandChecks=checks,
                         rsa100TransferUnits=rows[-1]['transferUnits'],
                         rsa100WeightedMomentDeficit=rows[-1]['weightedMomentDeficit'])))
