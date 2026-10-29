"""Reviewed R2 limits and source scope; shared only by trusted infrastructure."""
SOURCE_COMMIT = '06f33bdda655b37f578a81bbca99ef7567407926'
SOURCE_TREE = '4b2612df3cfcf5c5bfc0498b8ea75f8d3e453fce'
SCOPE = 'PERF-009 PortablePilot traced=False; full provider hardening included'
WORKERS = 2
ITERATION_SECONDS = 2250
IMAGE_SECONDS = 1800
JOB_MINUTES = 360
MEMORY_BYTES = 6 * 1024**3
WORK_BYTES = 4 * 1024**3
MAX_THROTTLED_FRACTION = .01


def budget():
    # No reuse speedup credited. These are enforcement envelopes, not a claim
    # that an arbitrary reviewed candidate finishes inside them.
    parts = dict(campaigns=8 * ITERATION_SECONDS, image=IMAGE_SECONDS,
                 preparation=300, packaging_uploads=600, operational_margin=900)
    assert sum(parts.values()) == JOB_MINUTES * 60
    return parts