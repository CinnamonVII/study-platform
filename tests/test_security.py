import importlib.util
from pathlib import Path
import pytest
from engine.models import Snapshot

def test_private_fixture_cannot_enter_public_build(tmp_path):
    spec=importlib.util.spec_from_file_location('check_public','scripts/check-public.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
    s=Snapshot.model_validate_json(Path('fixtures/demo/snapshot-v2.json').read_text());s.documents['demo-0'].private=True
    path=tmp_path/'snapshot.json';path.write_text(s.model_dump_json())
    with pytest.raises(AssertionError):m.check_fixture(path)

def test_private_content_not_cached_in_service_worker():
    source=Path('scripts/pwa.mjs').read_text()
    assert "r.headers.has('Authorization')" in source and 'u.origin!==base.origin' in source
    assert 'caches.delete' in source and 'cache.put' not in source
