import json
from pathlib import Path
import pytest
from fastapi.testclient import TestClient
from backend.api.main import create_app
from engine.models import *
from engine.sync.store import Store
from engine.sync.pipeline import Pipeline
from engine.demo import DemoGenerator, documents
from engine.repetition import schedule
TOKEN='a'*64
@pytest.fixture
def setup(tmp_path):
    s=Store(tmp_path/'state');Pipeline(s,DemoGenerator()).run(documents())
    api=TestClient(create_app(s,Config(allowed_origins=['https://cinnamonvii.github.io']),TOKEN))
    return s,api

def headers():return {'Authorization':'Bearer '+TOKEN}
def snapshot(api):return api.get('/v1/snapshot',headers=headers()).json()

def test_authentication_required(setup):
    s,api=setup;assert api.get('/v1/snapshot').status_code==401
    assert api.get('/v1/snapshot',headers={'Authorization':'Bearer wrong'}).status_code==401
    assert api.post('/v1/sync').status_code==401

def test_no_fictitious_auth_when_server_unconfigured(tmp_path):
    api=TestClient(create_app(Store(tmp_path),Config(),''));assert api.get('/v1/snapshot').status_code==503

def test_validation_and_manual_edits(setup):
    s,api=setup;q=next(iter(s.load().questions.values()));data=snapshot(api)
    r=api.patch('/v1/questions/'+q.id,headers=headers(),json={'revision':data['revision'],'changes':{'status':'validated'}})
    assert r.status_code==422
    docid=q.sources[0].document_id
    r=api.patch('/v1/documents/'+docid,headers=headers(),json={'revision':data['revision'],'changes':{'status':'validated'}});assert r.status_code==200
    data=snapshot(api)
    r=api.patch('/v1/questions/'+q.id,headers=headers(),json={'revision':data['revision'],'changes':{'answer':'Correction manuelle','status':'validated'}})
    assert r.status_code==200 and s.load().questions[q.id].manual
    assert api.post('/v1/questions/'+q.id+'/review',headers=headers(),json={'grade':4}).status_code==200

def test_optimistic_conflict(setup):
    s,api=setup;doc=next(iter(s.load().documents));r=api.patch('/v1/documents/'+doc,headers=headers(),json={'revision':'old','changes':{'text':'x'}});assert r.status_code==409

def test_private_fields_cannot_be_forged(setup):
    s,api=setup;doc=next(iter(s.load().documents));r=api.patch('/v1/documents/'+doc,headers=headers(),json={'revision':snapshot(api)['revision'],'changes':{'private':False,'original':'/etc/passwd'}});assert r.status_code==422

def test_uncertain_source_cannot_be_validated(setup):
    s,api=setup;doc=next(iter(s.load().documents))
    r=api.patch('/v1/documents/'+doc,headers=headers(),json={'revision':snapshot(api)['revision'],'changes':{'status':'validated','uncertainties':[{'message':'Formule illisible'}]}})
    assert r.status_code==422

def test_report_question_removes_from_training(setup):
    s,api=setup;q=next(iter(s.load().questions));assert api.post('/v1/questions/'+q+'/report',headers=headers(),json={'message':'Signe incorrect'}).status_code==200
    assert 'report:'+q in s.load().proposals

def test_private_responses_no_cache_and_cors(setup):
    _,api=setup;r=api.get('/v1/snapshot',headers={**headers(),'Origin':'https://cinnamonvii.github.io'})
    assert r.headers['cache-control']=='no-store' and r.headers['access-control-allow-origin']=='https://cinnamonvii.github.io'
    r=api.get('/v1/snapshot',headers={**headers(),'Origin':'https://evil.example'})
    assert 'access-control-allow-origin' not in r.headers

def test_sm2_known_intervals():
    from datetime import datetime,timezone
    r=Review(question_id='q');at=datetime(2026,1,1,tzinfo=timezone.utc)
    for expected in [1,6,16]:r=schedule(r,5,at);assert r.interval==expected
    r=schedule(r,1,at);assert r.interval==1 and r.repetitions==0
