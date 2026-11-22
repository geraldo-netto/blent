"""Finite loopback measurements; wait for original children without elapsed kill caps."""
from pathlib import Path
import hashlib,json,platform,random,subprocess,time

ROOT=Path('/home/netto/Desktop/projects/blent')
OUT=Path('/tmp/blent-buffer-sweep-20261122')
BINARY=ROOT/'target/release/examples/websocket_buffers'
SIZES=[4,8,16,32,64,128]
WORKLOADS={
    'interactive': ['--payload','200','--clients','8','--frames','10000'],
    'small_burst': ['--payload','200','--clients','8','--frames','10000','--burst'],
    'large_burst': ['--payload','65536','--clients','4','--frames','1024','--burst'],
}

def command(workload,size):
    return ['taskset','-c','2,4',str(BINARY),'--buffer-kib',str(size),*WORKLOADS[workload]]

def measure(workload,size,phase,block,position):
    started=time.time()
    result=subprocess.run(command(workload,size),text=True,capture_output=True,check=True)
    return dict(json.loads(result.stdout),workload=workload,phase=phase,block=block,
                position=position,utc_started=started,utc_finished=time.time(),command=command(workload,size))

def plan():
    rng=random.Random(17380022)
    cases=[]
    for cycle in range(3):
        base=rng.sample(SIZES,len(SIZES))
        shifts=rng.sample(range(6),6)
        for step,shift in enumerate(shifts):
            block=cycle*6+step
            order=base[shift:]+base[:shift]
            work_order=list(WORKLOADS)
            work_order=work_order[block%3:]+work_order[:block%3]
            for workload in work_order:
                cases.extend((workload,size,block,pos) for pos,size in enumerate(order))
    return cases

def run():
    cases=plan()
    metadata=dict(seed=17380022,blocks=18,planned_runs=len(cases),warmups=18,affinity=[2,4],
        source_sha256=hashlib.sha256((ROOT/'host/examples/websocket_buffers.rs').read_bytes()).hexdigest(),
        binary_sha256=hashlib.sha256(BINARY.read_bytes()).hexdigest(),platform=platform.platform(),
        toolchain=subprocess.check_output(['rustc','--version'],text=True).strip(),
        cpu=subprocess.check_output(['lscpu'],text=True),plan=cases)
    (OUT/'metadata.json').write_text(json.dumps(metadata,indent=2)+'\n')
    with (OUT/'warmups.jsonl').open('x') as output:
        for workload in WORKLOADS:
            for size in SIZES:
                output.write(json.dumps(measure(workload,size,'warmup',-1,-1))+'\n');output.flush()
    print('warmups complete: 18/18',flush=True)
    with (OUT/'results.jsonl').open('x') as output:
        for number,(workload,size,block,position) in enumerate(cases,1):
            output.write(json.dumps(measure(workload,size,'measurement',block,position))+'\n');output.flush()
            if number%18==0:
                print(f'measurements complete: {number}/{len(cases)}',flush=True)

if __name__=='__main__':
    run()
