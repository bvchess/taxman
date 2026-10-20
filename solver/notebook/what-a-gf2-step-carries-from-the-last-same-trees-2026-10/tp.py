"""Sums the samples of an Instruments Time Profiler trace by function: the trace exported with
`xcrun xctrace export --input T.trace --xpath '/trace-toc/run[@number="1"]/data/table[@schema="time-profile"]'`,
every frame named with its inlined frames (innermost first), the crate's names shortened by
solver/tools/profile/attribute.py.  Standard library only.

usage: tp.py EXPORT.xml self [N]          the samples by the innermost function of the crate
       tp.py EXPORT.xml incl [N]          the samples under each function of the crate
       tp.py EXPORT.xml children FUNCTION the samples under FUNCTION, by the frame below it
"""
import collections
import importlib.util
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

ATTRIBUTE = Path(__file__).resolve().parents[2] / 'tools' / 'profile' / 'attribute.py'
spec = importlib.util.spec_from_file_location('attribute', ATTRIBUTE)
attribute = importlib.util.module_from_spec(spec)
spec.loader.exec_module(attribute)


def samples(path):
    """(weight in ms, [frame names, leaf first]) for each sample; frames and stacks are written once and referred to."""
    root = ET.parse(path).getroot()
    frames, stacks, weights = {}, {}, {}
    out = []
    for row in root.iter('row'):
        w = row.find('weight')
        if w is None:
            weight = 1.0
        else:
            if 'id' in w.attrib:
                weights[w.attrib['id']] = int(w.text) / 1e6
            weight = weights[w.attrib.get('id') or w.attrib['ref']]
        bt = row.find('tagged-backtrace')
        if bt is None:
            continue
        if 'ref' in bt.attrib:
            stack = stacks[bt.attrib['ref']]
        else:
            stack = []
            for f in bt.iter('frame'):
                if 'ref' in f.attrib:
                    stack.append(frames[f.attrib['ref']])
                else:
                    name = f.attrib.get('name', '?')
                    if name.startswith('_R'):
                        name = attribute.short(name + ' (in x)')
                    frames[f.attrib['id']] = name
                    stack.append(name)
            stacks[bt.attrib['id']] = stack
        out.append((weight, stack))
    return out


def main():
    path, mode = sys.argv[1], sys.argv[2]
    ss = samples(path)
    total = sum(w for w, _ in ss)
    c = collections.Counter()
    if mode == 'self':
        for w, st in ss:
            ours = [f for f in st if f.startswith('taxman:')]
            c[ours[0] if ours else (st[0] if st else '?')] += w
    elif mode == 'incl':
        for w, st in ss:
            for k in set(st):
                if k.startswith('taxman:'):
                    c[k] += w
    elif mode == 'children':
        fn = sys.argv[3]
        under = 0
        for w, st in ss:
            if fn in st:
                i = st.index(fn)
                under += w
                c[st[i - 1] if i > 0 else '(self)'] += w
        print(f'{fn}: {100 * under / total:.2f}% of all')
    n = int(sys.argv[3]) if mode != 'children' and len(sys.argv) > 3 else 40
    print(f'total {total:.0f} ms')
    for k, v in c.most_common(n):
        print(f'{100 * v / total:6.2f}%  {k}')


main()
