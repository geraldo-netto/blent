"""Resolve noisy saturated small-burst comparison with sixty balanced pairs."""
import json,random
from run import OUT,measure


def run():
    order=[False,True]*30
    random.Random(17380025).shuffle(order)
    (OUT/'small-confirmation-plan.json').write_text(json.dumps(dict(seed=17380025,orders=order),indent=2)+'\n')
    with (OUT/'small-confirmation.jsonl').open('x') as output:
        for block,reverse in enumerate(order):
            sizes=[8,4] if reverse else [4,8]
            for position,size in enumerate(sizes):
                output.write(json.dumps(measure('small_burst',size,'small_confirmation',block,position))+'\n');output.flush()
            if (block+1)%10==0:
                print(f'small-burst confirmation pairs complete: {block+1}/60',flush=True)

if __name__=='__main__':
    run()
