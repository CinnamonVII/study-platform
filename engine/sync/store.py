"""Atomic snapshots + a cross-process lock shared by API, CLI and runner."""
import fcntl
import json
import os
from contextlib import contextmanager
from pathlib import Path
from engine.models import Snapshot

class BusyError(RuntimeError): pass

class Store:
    def __init__(self, root):
        self.root = Path(root)
        self.root.mkdir(parents=True, exist_ok=True, mode=0o700)
        self.path = self.root/'snapshot.json'
    @contextmanager
    def lock(self):
        with (self.root/'sync.lock').open('a') as f:
            try:
                fcntl.flock(f, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise BusyError('Another synchronization or edit is running')
            try: yield
            finally: fcntl.flock(f, fcntl.LOCK_UN)
    def load(self):
        return Snapshot.model_validate_json(self.path.read_text()) if self.path.exists() else Snapshot()
    def save(self, state):
        data = state.model_dump_json(indent=2)
        tmp = self.root/'snapshot.tmp'
        fd = os.open(tmp, os.O_WRONLY|os.O_CREAT|os.O_TRUNC, 0o600)
        with os.fdopen(fd,'w') as f:
            f.write(data); f.flush(); os.fsync(f.fileno())
        os.replace(tmp, self.path)
        fd = os.open(self.root, os.O_RDONLY)
        try: os.fsync(fd)
        finally: os.close(fd)
    def mutate(self, fn):
        with self.lock():
            state=self.load(); result=fn(state); self.save(state); return result
