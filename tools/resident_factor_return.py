"""Verify emitted factor arithmetic and the terminal paired return only."""
from verify_dialect_factor import numeral


def factor_return(report,source):
    if report['source_word']!=source:raise ValueError('returned source changed')
    closure=report.get('closure')
    result={'status':report['status'],'source_bits':numeral(source).bit_length(),
            'proper_factor':False,'full_word_execution':False,
            'scope':'emitted factor arithmetic and terminal paired boundary'}
    if closure is None:return result
    n=numeral(source);d=numeral(closure['divisor_word']);q=numeral(closure['quotient_word'])
    if not 1<d<n or divmod(n,d)!=(q,0) or d*q!=n:
        raise ValueError('emitted factors do not close the exact source')
    tail=report['verification_word_control'];holding=report['holding_frame']
    if tail['pc']!=45 or tail['executed_positions']!=list(range(36,45)):
        raise ValueError('factor return lost its terminal operations')
    if tail['divisor_word']!=closure['divisor_word'] or tail['quotient_word']!=closure['quotient_word']:
        raise ValueError('factor readout differs from its terminal word')
    if numeral(tail['remainder_word'])!=0 or tail['product_word']!=source or numeral(tail['paired_word'])!=q:
        raise ValueError('factor return lost remainder, IMASM product or paired boundary')
    if not holding['fixed'] or holding['returned_bank']!=tail['returned_bank']:
        raise ValueError('factor return lost its enclosing fixed boundary')
    clock=closure['trigger_to_verified_factor_seconds']
    if clock!=tail['trigger_to_verified_factor_seconds'] or not 0<=clock<=report['trigger_to_readout_seconds']:
        raise ValueError('factor return lost its execution clocks')
    result.update(proper_factor=True,divisor=str(d),quotient=str(q),zero_remainder=True,
                  exact_imasm_product=True,paired_boundary_closed=True,
                  full_word_execution=bool((report.get('control_word') or {}).get('full_word_execution')),
                  trigger_to_verified_factor_seconds=clock,
                  trigger_to_readout_seconds=report['trigger_to_readout_seconds'])
    return result
