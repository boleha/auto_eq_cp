import urllib.request
import json
import numpy as np

def parse_rew_file(filepath):
    frequency = []
    raw = []
    with open(filepath, 'r', encoding='gbk', errors='ignore') as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith('*') or line.startswith('Freq'):
                continue
            parts = line.split()
            if len(parts) >= 2:
                try:
                    frequency.append(float(parts[0]))
                    raw.append(float(parts[1]))
                except ValueError:
                    continue
    return frequency, raw

select_freq, select_raw = parse_rew_file('test_file/OlA  II.txt')
target_freq, target_raw = parse_rew_file('test_file/harman2016.txt')

print(f"select: {len(select_freq)} points, {select_freq[0]:.1f}Hz ~ {select_freq[-1]:.1f}Hz")
print(f"target: {len(target_freq)} points, {target_freq[0]:.1f}Hz ~ {target_freq[-1]:.1f}Hz")

body = json.dumps({
    'select': {'frequency': select_freq, 'raw': select_raw},
    'target': {'frequency': target_freq, 'raw': target_raw},
    'eq_range': {'low': 30, 'high': 20000},
    'fs': 44100,
    'config': '10_PEAKING',
    # 'config': '4_PEAKING_WITH_SHELVES',
    'max_filters': 8,
    'gain_range': {'low': -12, 'high': 12},
    'q_range': {'low': 0.1, 'high': 10}
}).encode()

req = urllib.request.Request('http://localhost:8000/eq-by-range', data=body, headers={'Content-Type': 'application/json'})
resp = urllib.request.urlopen(req)
data = json.loads(resp.read())

print(f'\npreamp: {data["preamp"]:.2f} dB')
print(f'filters count: {len(data["filters"])}')
print()
for f in data['filters']:
    print(f'  {f["type"]:12s} fc={f["fc"]:8.1f}Hz  gain={f["gain"]:+7.2f}dB  q={f["q"]:.2f}')
