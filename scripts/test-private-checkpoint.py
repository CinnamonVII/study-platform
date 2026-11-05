"""Focused integration check using temporary Git repositories and synthetic data."""
import os
from pathlib import Path
import sqlite3
import subprocess
import tempfile


def run(args, **kwargs):
    return subprocess.run(args, check=True, stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, **kwargs)


project = Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory(prefix="study-checkpoint-test-") as temporary:
    root = Path(temporary)
    remote, work = root / "remote.git", root / "private"
    run(["git", "init", "--bare", str(remote)])
    run(["git", "init", "-b", "main", str(work)])
    run(["git", "config", "user.name", "Test"], cwd=work)
    run(["git", "config", "user.email", "test@example.invalid"], cwd=work)
    run(["git", "remote", "add", "origin", str(remote)], cwd=work)
    (work / "state").mkdir()
    (work / ".gitignore").write_text(
        (project / "packaging/private-data.gitignore").read_text())
    with sqlite3.connect(work / "state/study.sqlite") as db:
        db.execute("PRAGMA journal_mode=WAL")
        db.execute("CREATE TABLE publications(id TEXT,status TEXT)")
        db.execute("INSERT INTO publications VALUES('one','attempting')")
    environment = dict(os.environ, STUDY_STATE="state", STUDY_PRIVATE_REPOSITORY="true")
    script = str(project / "scripts/checkpoint-private-state.sh")
    run(["bash", script], cwd=work, env=environment)
    with sqlite3.connect(work / "state/checkpoint.sqlite") as db:
        assert db.execute("SELECT status FROM publications").fetchone() == ("attempting",)
    tracked = run(["git", "ls-files"], cwd=work).stdout.decode().splitlines()
    assert "state/checkpoint.sqlite" in tracked
    assert "state/study.sqlite" not in tracked
    assert run(["git", "rev-parse", "HEAD"], cwd=work).stdout == run(
        ["git", "--git-dir", str(remote), "rev-parse", "main"]).stdout
    run(["git", "remote", "set-url", "origin", str(root / "missing.git")], cwd=work)
    failure = subprocess.run(["bash", script], cwd=work, env=environment,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert failure.returncode != 0
    refused = subprocess.run(["bash", script], cwd=work,
                             env=dict(environment, STUDY_PRIVATE_REPOSITORY="false"),
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert refused.returncode != 0
print("Private checkpoint: consistent backup, remote persistence and failure gates verified")
