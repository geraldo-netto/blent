import json
from pathlib import Path
base = Path(__file__).resolve().parent
actors = json.loads((base / 'measure/results.json').read_text())
allocations = json.loads((base / 'allocations/results.json').read_text())
rows = []
for trial in actors['trials']:
    for role in trial['roles']:
        latency = sorted(role['hinted_latency_ns'])
        rows.append(dict(fallback_ms=trial['fallback_ms'], known=trial['known'], role=role['role'],
                         cpu_ms=role['idle_actor_cpu_ns'] / 1e6, advances=role['idle_advances'],
                         allocated_bytes=role.get('idle_actor_allocated_bytes'),
                         hinted_p95_ms=latency[30] / 1e6, hinted_max_ms=latency[-1] / 1e6,
                         missed_ms=[value / 1e6 for value in role['missed_hint_latency_ns']]))
summary = dict(scope=actors['scope'], java=actors['java'], actor_rows=rows, allocation_probes=allocations)
(base / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
