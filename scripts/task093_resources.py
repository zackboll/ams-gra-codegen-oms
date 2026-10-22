"""Linux process-tree measurements for the Task 093 qualification campaign.

RSS sums intentionally count shared pages per process: a conservative resource
budget, not unique physical memory. Sampling can miss very short-lived peaks.
"""
import os
from pathlib import Path
import resource
import threading
import time


class Measurement:
    def __init__(self):
        self.started = time.monotonic()
        self.cpu = self.cpu_seconds()
        self.peak_rss_kib = 0
        self.stop = threading.Event()
        self.thread = threading.Thread(target=self.sample, daemon=True)
        self.thread.start()

    @staticmethod
    def cpu_seconds():
        usages = [resource.getrusage(kind) for kind in
                  (resource.RUSAGE_SELF, resource.RUSAGE_CHILDREN)]
        return sum(usage.ru_utime + usage.ru_stime for usage in usages)

    def sample(self):
        while not self.stop.is_set():
            pending = [os.getpid()]
            descendants = set()
            rss = 0
            while pending:
                pid = pending.pop()
                if pid in descendants:
                    continue
                descendants.add(pid)
                try:
                    process = Path('/proc') / str(pid)
                    fields = (process / 'stat').read_text().rsplit(')', 1)[1].split()
                    rss += int(fields[21])
                    # Include children launched by executor threads, not only
                    # those attributed to the process's main Linux task.
                    for children in (process / 'task').glob('*/children'):
                        pending.extend(int(child) for child in children.read_text().split())
                except (OSError, ValueError, IndexError):
                    continue  # Process exited between listing and reading.
            self.peak_rss_kib = max(self.peak_rss_kib, rss * os.sysconf('SC_PAGE_SIZE') // 1024)
            self.stop.wait(0.2)

    def finish(self):
        self.stop.set()
        self.thread.join()
        return dict(wall_seconds=time.monotonic() - self.started,
                    cpu_seconds=self.cpu_seconds() - self.cpu,
                    sampled_peak_tree_rss_kib=self.peak_rss_kib,
                    sampling_interval_seconds=0.2)