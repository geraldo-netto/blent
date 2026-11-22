"""Summarize paired randomized-block runs without discarding outliers."""
from pathlib import Path
import collections,json,random,statistics

OUT=Path('/tmp/blent-buffer-sweep-20261122')
METRICS=['payload_mib_per_second','roundtrip_p50_ns','roundtrip_p95_ns','peak_rss_kib']

def quantile(values,p):
    ordered=sorted(values)
    offset=(len(ordered)-1)*p
    low=int(offset)
    high=min(low+1,len(ordered)-1)
    return ordered[low]+(ordered[high]-ordered[low])*(offset-low)

def summary(values):
    return dict(median=statistics.median(values),min=min(values),p10=quantile(values,.1),
                p25=quantile(values,.25),p75=quantile(values,.75),p90=quantile(values,.9),max=max(values))

def confidence(ratios):
    rng=random.Random(17380023)
    medians=[statistics.median(rng.choices(ratios,k=len(ratios))) for _ in range(10000)]
    return [quantile(medians,.025),quantile(medians,.975)]

def paired(groups,workload,size,metric):
    baseline={r['block']:r[metric] for r in groups[workload,4]}
    ratios=[r[metric]/baseline[r['block']] for r in groups[workload,size]]
    return dict(median_ratio=statistics.median(ratios),bootstrap_95ci=confidence(ratios),
                ratios=ratios,ratio_above_one=sum(r>1 for r in ratios),pairs=len(ratios))

def analyze(path,output):
    rows=[json.loads(line) for line in path.read_text().splitlines()]
    groups=collections.defaultdict(list)
    for row in rows:
        groups[row['workload'],row['options']['buffer_kib']].append(row)
    summaries=[]
    for (workload,size),items in sorted(groups.items()):
        metrics={metric:summary([r[metric] for r in items]) for metric in METRICS if items[0][metric] is not None}
        pairs={metric:paired(groups,workload,size,metric) for metric in metrics if size!=4}
        summaries.append(dict(workload=workload,buffer_kib=size,runs=len(items),metrics=metrics,paired_to_4kib=pairs))
    output.write_text(json.dumps(dict(runs=len(rows),summaries=summaries),indent=2)+'\n')
    for row in summaries:
        metrics=row['metrics'];throughput=metrics['payload_mib_per_second']['median']
        p95=metrics.get('roundtrip_p95_ns',{}).get('median')
        print(row['workload'],row['buffer_kib'],row['runs'],f'{throughput:.2f} MiB/s',p95,metrics['peak_rss_kib']['median'])

if __name__=='__main__':
    analyze(OUT/'results.jsonl',OUT/'summary.json')
