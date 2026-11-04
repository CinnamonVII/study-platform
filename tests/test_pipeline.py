import json
from dataclasses import dataclass
from pathlib import Path
import pytest
from engine.models import *
from engine.demo import DemoGenerator, documents
from engine.sync.store import Store, BusyError
from engine.sync.pipeline import Pipeline
from engine.ingestion.files import ingest
from engine.validation.rules import publishable

@pytest.fixture
def store(tmp_path): return Store(tmp_path/'private')
@pytest.fixture
def doc(): return documents()[0]
class Counting(DemoGenerator):
    def __init__(self):self.calls=0;self.reads=0
    def generate(self,docs,existing=None):self.calls+=1;return super().generate(docs,existing)
    def transcribe(self,doc):
        self.reads+=1
        return Transcription(text='Suite arithmétique : ajout d’une constante.\nu₀ = 3\nu₁ = 5',uncertainties=[],observations=[])

def test_handwritten_level_one_preserves_original(store,tmp_path):
    from PIL import Image
    path=tmp_path/'cahier.png';Image.new('RGB',(200,100),'white').save(path)
    d=ingest(store,path,'Mathématiques','Suites',level=1,reason='Photo de mon cahier')
    original=Path(d.original).read_bytes();g=Counting();r=Pipeline(store,g).run([d])
    s=store.load();assert g.reads==1 and len(s.fiches)==1 and s.questions
    assert Path(s.documents[d.id].original).read_bytes()==original
    assert s.documents[d.id].status==Status.pending
    assert all(q.status==Status.pending for q in s.questions.values())

def test_level_one_requires_justification():
    with pytest.raises(ValueError):Document(id='d',title='x',kind='cours',level=1,subject='SES',chapter='x',content_hash='x')

def test_unknown_provenance_not_inferred(store,doc):
    doc.level=None;Pipeline(store,DemoGenerator()).run([doc]);assert not store.load().fiches

def test_generated_document_never_becomes_original(doc):
    with pytest.raises(ValueError): Document.model_validate({**doc.model_dump(),'derived':True})

def test_modify_source_regenerates_stable_fiche(store,doc):
    g=Counting();p=Pipeline(store,g);p.run([doc]);old=store.load();ids=set(old.fiches)
    d=doc.model_copy(update={'text':doc.text+'\nVérifier la raison.','content_hash':'new'})
    p.run([d]);assert g.calls==2 and set(store.load().fiches)==ids
    assert len(store.load().questions)==3

def test_homework_generates_without_unjustified_level_one(store):
    d=documents()[1];d.level=None;p=Pipeline(store,DemoGenerator());p.run([d]);assert store.load().questions

def test_no_change_no_duplicate_no_regeneration(store,doc):
    g=Counting();p=Pipeline(store,g);p.run([doc]);r=p.run([doc]);assert g.calls==1
    assert r['changed']==r['fiches']==r['questions']==[]

def test_semantic_duplicate_filter(store,doc):
    class D(DemoGenerator):
        def generate(self,*a,**kw):
            r=super().generate(*a,**kw);q=r.questions[0].model_copy(deep=True);q.concept_key='copy';q.prompt=q.prompt.upper()+'!';r.questions.append(q);return r
    Pipeline(store,D()).run([doc]);assert len(store.load().questions)==3

def test_resume_after_interruption(store,doc):
    class Broken(Counting):
        def generate(self,*a,**kw):raise RuntimeError('interruption')
    p=Pipeline(store,Broken());r=p.run([doc]);assert r['errors'] and store.load().last_success is None
    g=Counting();r=Pipeline(store,g).run([doc]);assert not r['errors'] and len(store.load().fiches)==1
    Pipeline(store,g).run([doc]);assert g.calls==1

def test_formula_uncertainty_blocks_questions(store,tmp_path):
    from PIL import Image
    path=tmp_path/'bad.png';Image.new('RGB',(10,10),'white').save(path)
    d=ingest(store,path,'Mathématiques','Cercle',level=1,reason='Cahier personnel')
    class Bad(Counting):
        def transcribe(self,doc):return Transcription(text='x² ? y² = 4',uncertainties=[Uncertainty(message='Signe + ou − illisible',page=1)],observations=[])
    r=Pipeline(store,Bad()).run([d]);s=store.load();assert not s.questions and not s.fiches and s.documents[d.id].uncertainties

def test_manual_question_preserved_proposal_created(store,doc):
    p=Pipeline(store,DemoGenerator());p.run([doc]);s=store.load();q=next(iter(s.questions.values()));qid=q.id;q.answer='Correction de Merlin';q.manual=True;store.save(s)
    d=doc.model_copy(update={'content_hash':'changed','text':doc.text+'\nNouvelle information'})
    p.run([d]);s=store.load();assert s.questions[qid].answer=='Correction de Merlin' and 'question:'+qid in s.proposals

def test_manual_transcription_preserved(store,doc):
    p=Pipeline(store,DemoGenerator());p.run([doc]);s=store.load();s.documents[doc.id].manual=True;s.documents[doc.id].text='Texte corrigé manuellement';store.save(s)
    p.run([doc.model_copy(update={'content_hash':'new','text':'autre transcription'})]);d=store.load().documents[doc.id]
    assert d.text=='Texte corrigé manuellement' and any(not u.resolved for u in d.uncertainties)

def test_validated_level_two_reused_with_originals(store,doc):
    p=Pipeline(store,DemoGenerator());p.run([doc]);s=store.load()
    derivative=Document(id='derived',title='Fiche validée',kind='fiche',level=2,subject=doc.subject,chapter=doc.chapter,content_hash='derivative',text='Méthode validée complémentaire',sources=[Reference(document_id=doc.id)],status=Status.validated,derived=True)
    class Capture(Counting):
        def generate(self,docs,existing=None):self.seen=docs;return super().generate(docs,existing)
    g=Capture();Pipeline(store,g).run([derivative]);assert {x.id for x in g.seen}=={doc.id,'derived'}
    Pipeline(store,g).run([derivative]);assert g.calls==1

def test_invalid_source_citation_rejected(store,doc):
    class Invalid(DemoGenerator):
        def generate(self,*a,**kw):
            r=super().generate(*a,**kw);r.questions[0].sources=[Reference(document_id='invented')];return r
    r=Pipeline(store,Invalid()).run([doc]);assert r['errors'] and not store.load().fiches

def test_collision_no_partial_mutation(store,doc):
    with store.lock():
        with pytest.raises(BusyError):Pipeline(store,DemoGenerator()).run([doc])
    assert not store.load().documents

def test_failed_publication_retried_without_regeneration(store,doc):
    class Pub:
        def __init__(self):self.fail=True;self.calls=0
        def publish(self,s,store):
            self.calls+=1
            if self.fail:raise RuntimeError('GitHub/Notion unavailable')
    pub=Pub();g=Counting();p=Pipeline(store,g,[pub]);assert p.run([doc])['errors']
    pub.fail=False;assert not p.run([doc])['errors'];assert g.calls==1 and pub.calls==2

def test_deleted_source_invalidates_outputs(store,doc):
    d=doc.model_copy(update={'id':'notion-abc'});p=Pipeline(store,DemoGenerator());p.run([d])
    p.run([],complete_inventory=True);s=store.load();assert s.documents[d.id].deleted
    assert all(f.stale for f in s.fiches.values()) and all(not publishable(q,s) for q in s.questions.values())

def test_archived_source_invalidates(store,doc):
    p=Pipeline(store,DemoGenerator());p.run([doc]);p.run([doc.model_copy(update={'deleted':True})]);assert store.load().documents[doc.id].deleted

def test_classification_change_invalidates(store,doc):
    p=Pipeline(store,DemoGenerator());p.run([doc]);d=doc.model_copy(update={'level':None});p.run([d]);assert all(q.stale for q in store.load().questions.values())

def test_end_to_end_four_subjects_schema_exports(store,tmp_path):
    from engine.export.render import export_fiche
    from jsonschema import Draft202012Validator
    r=Pipeline(store,DemoGenerator()).run(documents());s=store.load()
    assert not r['errors'] and len(s.fiches)==4 and len(s.questions)==12
    schema=Snapshot.model_json_schema(mode='serialization');Draft202012Validator(schema).validate(s.model_dump(mode='json'))
    import fitz
    for f in s.fiches.values():
        path=export_fiche(f,s,tmp_path/'exports');pdf=fitz.open(path/'fiche.pdf')
        assert 'retenir' in ''.join(p.get_text() for p in pdf)
        assert (path/'fiche.tex').exists() and (path/'fiche.md').exists()

def test_accepting_generated_fiche_reinjects_once_without_loop(store,doc):
    g=Counting();p=Pipeline(store,g);p.run([doc]);s=store.load()
    for d in s.documents.values():d.status=Status.validated
    f=next(iter(s.fiches.values()));f.status=Status.validated;f.manual=True;store.save(s)
    p.run([]);count=g.calls
    assert any(d.derived and d.level==2 for d in store.load().documents.values())
    p.run([]);assert g.calls==count
