"""Independent finer sweep around the initial 8-KiB candidate."""
import json,random
from run import OUT,WORKLOADS,measure


def plan():
    rng=random.Random(17380024)
    cases=[]
    for cycle in range(3):
        base=rng.sample([4,6,8,12],4)
        for step,shift in enumerate(rng.sample(range(4),4)):
            block=cycle*4+step
            order=base[shift:]+base[:shift]
            workloads=list(WORKLOADS)
            workloads=workloads[block%3:]+workloads[:block%3]
            for workload in workloads:
                cases.extend((workload,size,block,pos) for pos,size in enumerate(order))
    return cases


def run():
    cases=plan()
    (OUT/'confirmation-plan.json').write_text(json.dumps(dict(seed=17380024,blocks=12,plan=cases),indent=2)+'\n')
    with (OUT/'confirmation-warmups.jsonl').open('x') as output:
        for workload in WORKLOADS:
            for size in [4,6,8,12]:
                output.write(json.dumps(measure(workload,size,'confirmation_warmup',-1,-1))+'\n');output.flush()
    print('confirmation warmups complete: 12/12',flush=True)
    with (OUT/'confirmation.jsonl').open('x') as output:
        for number,(workload,size,block,position) in enumerate(cases,1):
            output.write(json.dumps(measure(workload,size,'confirmation',block,position))+'\n');output.flush()
            if number%12==0:
                print(f'confirmation measurements complete: {number}/{len(cases)}',flush=True)

if __name__=='__main__':
    run()
