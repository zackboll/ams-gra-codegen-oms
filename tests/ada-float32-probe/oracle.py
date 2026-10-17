from fractions import Fraction
from pathlib import Path
import hashlib
import json
import random
import sys

sys.set_int_max_str_digits(20000)
ROOT = Path(__file__).resolve().parent

def exact(bits):
    exponent, mantissa = bits >> 23, bits & 0x7fffff
    return Fraction(mantissa if exponent == 0 else mantissa + 0x800000) * Fraction(2) ** (-149 if exponent == 0 else exponent - 150)

def decimal(value):
    sign = '-' if value < 0 else ''
    value = abs(value)
    n, d = value.numerator, value.denominator
    places = d.bit_length() - 1
    assert d == 1 << places
    digits = str(n * 5 ** places).rjust(places + 1, '0')
    return sign + (digits[:-places] + '.' + digits[-places:] if places else digits)

def oracle(text):
    negative = text.startswith('-')
    # Bound enormous exponents without imposing this limitation on the parser.
    if 'e' in text.lower():
        mantissa, exponent = text.lower().split('e')
        e = int(exponent)
        if e > 10000:
            return 'OVERFLOW' if Fraction(mantissa) else ('80000000' if negative else '00000000')
        if e < -10000:
            return '80000000' if negative else '00000000'
    value = abs(Fraction(text))
    # Overflow rounds at max + half an ulp (ties to the even next exponent).
    if value >= exact(0x7f7fffff) + Fraction(2) ** 103:
        return 'OVERFLOW'
    lo, hi = 0, 0x7f7fffff
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if exact(mid) <= value:
            lo = mid
        else:
            hi = mid - 1
    if lo < 0x7f7fffff:
        down, up = value - exact(lo), exact(lo + 1) - value
        if up < down or (up == down and lo & 1):
            lo += 1
    return f'{lo | (0x80000000 if negative else 0):08X}'

def main():
    original = (ROOT/'stage-zero-vectors.tsv').read_text().splitlines()
    adversarial = [line.split('\t', 1)[1] for line in original if line.split('\t')[0] not in ['REJECT','SPECIAL']]
    adversarial += ['18446744073709551615', '9223372036854775808', '340282346638528859811704183484516925440',
                    '1E+000001', '-0e999999999999999999999999999999',
                    '1e999999999999999999999999999999', '-1e-999999999999999999999999999999',
                    '1.' + '0' * 2000 + '1', '0.' + '0' * 2000 + '1']
    for bits in [0,1,2,0x007ffffe,0x007fffff,0x00800000,0x3f7fffff,0x3f800000,0x3f800001,0x7f7ffffe]:
        midpoint = (exact(bits) + exact(bits+1))/2
        epsilon = Fraction(1,10 ** 200)
        for x in [midpoint-epsilon, midpoint, midpoint+epsilon]:
            if x >= 0:
                s = decimal(x) if (x.denominator & (x.denominator-1)) == 0 else None
                # Denominators are powers of 2 and 5; render exactly by long division.
                if s is None:
                    n, d = x.numerator, x.denominator
                    whole, rem = divmod(n,d); digits=[]
                    while rem:
                        digit, rem = divmod(rem*10,d); digits.append(str(digit))
                    s = str(whole)+'.'+''.join(digits)
                adversarial += [s, '-' + s]
    overflow_midpoint = exact(0x7f7fffff)+Fraction(2)**103
    adversarial += [str(int(overflow_midpoint)-1), str(int(overflow_midpoint)), str(int(overflow_midpoint)+1),
                    '3.5e38', '1e1000', '-1e1000', '-1e-1000']
    rows=[('adversarial',oracle(s),s) for s in adversarial]
    rng=random.Random(9501)
    # Generated midpoint stress is separate from the hand-authored adversaries.
    # Perturb on either side by an exact power-of-ten much smaller than an ulp.
    for bits in [rng.randrange(0x7f7fffff) for _ in range(750)]:
        midpoint=(exact(bits)+exact(bits+1))/2
        epsilon=Fraction(1,10**200)
        for value in [midpoint-epsilon,midpoint,midpoint+epsilon]:
            n,d=value.numerator,value.denominator
            whole,rem=divmod(n,d); digits=[]
            while rem:
                digit,rem=divmod(rem*10,d); digits.append(str(digit))
            text=str(whole)+('.'+''.join(digits) if digits else '')
            for token in [text,'-'+text]:
                rows.append(('generated-midpoint',oracle(token),token))
    bits_set={0,1,2,0x007fffff,0x00800000,0x7f7fffff}
    for exponent in range(255):
        for mantissa in [0,1,0x3fffff,0x7ffffe,0x7fffff]:
            bits_set.add((exponent<<23)|mantissa)
    bits_set.update(rng.randrange(0x7f800000) for _ in range(1500))
    encoding=[]
    for bits in sorted(bits_set):
        s=decimal(exact(bits))
        for text in [s, '-'+s, s+'e+0000000000000000000000000']:
            rows.append(('generated-exact',oracle(text),text))
        # A bounded 9-significant-digit spelling, not used as the oracle.
        text=format(float(exact(bits)),'.9g')
        rows.append(('generated-short',oracle(text),text))
        encoding += [f'{bits:08X}',f'{bits|0x80000000:08X}']
    (ROOT/'conversions.tsv').write_text(''.join(f'{group}\t{want}\t{text}\n' for group,want,text in rows))
    (ROOT/'encoding.tsv').write_text('\n'.join(encoding)+'\n')
    # A whole document with unconverted raw numeric values and independently known spans.
    tokens=[s for _,_,s in rows[:len(adversarial)]]
    doc=' {"numbers" : [ '+',\n  '.join(tokens)+' ], "integer": 42} '
    (ROOT/'tokens.json').write_text(doc)
    spans=[]; pos=doc.index('[ ')+2
    for s in tokens:
        start=doc.index(s,pos); spans.append({'text':s,'first':start+1,'last':start+len(s)})
        pos=start+len(s)
    start=doc.rindex('42'); spans.append({'text':'42','first':start+1,'last':start+2})
    (ROOT/'token-expectations.json').write_text(json.dumps(spans,indent=2)+'\n')
    (ROOT/'corpus-summary.json').write_text(json.dumps({'conversion_rows':len(rows),'adversarial_rows':len(adversarial),'encoding_rows':len(encoding),'numeric_tokens':len(spans),'seed':9501,'oracle':'exact Fraction binary search; nearest, ties even; signed zero; overflow midpoint'},indent=2)+'\n')

if __name__=='__main__':
    main()