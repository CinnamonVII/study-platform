"""Fail closed for public fixture export and static artifacts."""
import json
import sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from engine.models import Snapshot

def check_fixture(path):
    s=Snapshot.model_validate_json(Path(path).read_text())
    assert s.demo, 'Only the explicit demo fixture may enter the public build'
    assert all(d.id.startswith('demo-') and not d.private and not d.original and not d.origin_url for d in s.documents.values())
    assert all(not x.private and all(r.document_id.startswith('demo-') for r in x.sources) for x in [*s.fiches.values(),*s.questions.values()])
    assert not s.proposals and not s.reviews, 'Never publish user annotations or progress'
    return s

if __name__=='__main__':
    check_fixture('fixtures/demo/snapshot-v2.json')
    dist=Path('apps/web/dist')
    for p in dist.rglob('*'):
        if p.is_file():
            assert not any(x in p.parts for x in ('originals','private','transcriptions'))
            assert p.suffix not in ('.pdf','.env','.sqlite','.docx')
            if p.suffix in ('.js','.json','.html'):
                text=p.read_text()
                for canary in ['STUDY_API_TOKEN=','NOTION_TOKEN=','OPENAI_API_KEY=','PRIVATE_TEST_CANARY']:
                    assert canary not in text, 'Secret or private canary found in static build'
    print('Public boundary verified: fictitious content only, no private asset paths')
