#!/usr/bin/env python3
"""Measure hosted job intervals; queued started_at values are not execution.

Input is one or more complete REST jobs responses. Each completed job requires
real setup/completion step timestamps. Endpoints are half-open [start,end).
"""
import argparse
from datetime import datetime
import json
from pathlib import Path


def report(jobs, limit=5, lanes=None):
    if len({j['id'] for j in jobs}) != len(jobs):
        raise ValueError('duplicate job identities')
    events=[]
    rows=[]
    minutes=0
    if lanes is not None and set(lanes) != {j['id'] for j in jobs}:
        raise ValueError('incomplete lane identity inventory')
    if lanes is not None and any(type(lane) is not int or not 1 <= lane <= 5 for lane in lanes.values()):
        raise ValueError('invalid shared main lane')
    for job in jobs:
        if job['status']!='completed' or job['conclusion']!='success':
            raise ValueError('incomplete/unsuccessful job: '+job['name'])
        steps=job.get('steps',[])
        if not steps or any(s['conclusion']!='success' for s in steps):
            raise ValueError('missing/failed execution evidence: '+job['name'])
        start=datetime.fromisoformat(steps[0]['started_at'].replace('Z','+00:00'))
        end=datetime.fromisoformat(job['completed_at'].replace('Z','+00:00'))
        if end<=start:
            raise ValueError('invalid execution interval')
        events.extend([(start,1,job['id']),(end,-1,job['id'])])
        duration=(end-start).total_seconds()
        minutes+=duration/60
        rows.append(dict(id=job['id'],name=job['name'],started_at=start.isoformat(),completed_at=end.isoformat(),seconds=duration,
                         lane=lanes[job['id']] if lanes is not None else None))
    current=maximum=0
    lane_running = {}
    lane_maximum = {}
    timeline=[]
    for when,delta,job_id in sorted(events):
        current+=delta
        if lanes is not None:
            lane = lanes[job_id]
            lane_running[lane] = lane_running.get(lane, 0) + delta
            lane_maximum[lane] = max(lane_maximum.get(lane, 0), lane_running[lane])
            if lane_running[lane] > 1:
                raise ValueError('multiple running jobs in lane: ' + str(lane))
        maximum=max(maximum,current)
        timeline.append(dict(at=when.isoformat(),delta=delta,running=current,job_id=job_id))
    if maximum>limit:
        raise ValueError(f'observed concurrency {maximum} exceeds {limit}')
    return dict(jobs=len(rows),maximum_running=maximum,maximum_per_lane=lane_maximum,runner_minutes=round(minutes,3),
                wall_seconds=(max(e[0] for e in events)-min(e[0] for e in events)).total_seconds(),
                intervals=rows,timeline=timeline,
                queue_wait='not inferred from queued job started_at; requires run/job eligibility timestamps')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('jobs_json',nargs='+',type=Path)
    parser.add_argument('--limit',type=int,default=5)
    args=parser.parse_args()
    jobs=[j for p in args.jobs_json for j in json.loads(p.read_text())['jobs']]
    print(json.dumps(report(jobs,args.limit),indent=2))
