from __future__ import annotations
from collections import defaultdict
from pathlib import Path
from engine.models import Snapshot, Status, GENERATOR_VERSION, digest, stable, now
from engine.validation.rules import check_generation, duplicate, publishable

class Pipeline:
    def __init__(self,store,generator,publishers=()):
        self.store=store; self.generator=generator; self.publishers=publishers
    def run(self,incoming,complete_inventory=False):
        # Hold this lock through external calls. No second writer can race a slow job.
        with self.store.lock():
            state=self.store.load()
            report={'at':now(),'changed':[],'deleted':[],'fiches':[],'questions':[],'errors':[]}
            ids={d.id for d in incoming}
            for doc in incoming:
                old=state.documents.get(doc.id)
                if old:
                    # User classification and transcription always win over connector defaults.
                    if old.manual:
                        doc.level=old.level; doc.provenance_reason=old.provenance_reason
                        doc.kind=old.kind; doc.subject=old.subject; doc.chapter=old.chapter; doc.manual=True
                    same=doc.content_hash==old.content_hash and (doc.kind,doc.level,doc.subject,doc.chapter,doc.provenance_reason)==(old.kind,old.level,old.subject,old.chapter,old.provenance_reason) and doc.deleted==old.deleted and not old.deleted
                    if same: continue
                    if old.manual and old.content_hash!=doc.content_hash:
                        state.proposals['source:'+doc.id]=doc.model_dump(mode='json')
                        old.status=Status.pending
                        old.content_hash=doc.content_hash
                        # Keep corrected transcription but force a human reconciliation.
                        from engine.models import Uncertainty
                        old.uncertainties.append(Uncertainty(message='Source changed: reconcile preserved manual transcription with the new original'))
                        old.original=doc.original; old.modified_at=doc.modified_at
                        doc=old
                    elif old.content_hash!=doc.content_hash:
                        doc.status=Status.pending
                state.documents[doc.id]=doc
                report['changed'].append(doc.id)
            if complete_inventory:
                # Only complete, successful connector scans may tombstone their own Notion sources.
                for d in state.documents.values():
                    if d.id.startswith(('notion-','github-')) and d.id not in ids and not d.deleted:
                        d.deleted=True; d.status=Status.pending; report['deleted'].append(d.id)
            changed=set(report['changed']+report['deleted'])
            for item in [*state.fiches.values(),*state.questions.values()]:
                if any(r.document_id in changed for r in item.sources): item.stale=True
            self.store.save(state)
            # Transcriptions are checkpointed before generation; unresolved reads block generation.
            for doc in list(state.documents.values()):
                if doc.deleted or doc.text or not doc.original: continue
                try:
                    tr=self.generator.transcribe(doc)
                    doc.text=tr.text; doc.uncertainties=tr.uncertainties
                    doc.status=Status.pending
                    self.store.save(state)
                except Exception as e:
                    report['errors'].append({'stage':'transcription','id':doc.id,'error':type(e).__name__})
            # Feed accepted fiches back as level-2 evidence. Stable educational hashes exclude publication timestamps.
            from engine.models import Document
            from engine.export.render import markdown
            for f in list(state.fiches.values()):
                if f.status!=Status.validated or f.stale: continue
                text=markdown(f,state)
                did='derived-'+f.id
                state.documents[did]=Document(id=did,title=f.title,kind='fiche',level=2,subject=f.subject,chapter=f.chapter,
                    content_hash=digest(text),text=text,status=Status.validated,sources=f.sources,derived=True)
            groups=defaultdict(list)
            for d in state.documents.values():
                if d.deleted or d.status==Status.rejected or not d.text or any(not u.resolved for u in d.uncertainties): continue
                if d.level==1 or d.kind in ('devoir','exercice','correction','methode','annale') and not d.derived:
                    groups[(d.subject,d.chapter)].append(d)
            for (subject,chapter),docs in groups.items():
                fid=stable('fiche',subject,chapter)
                old=state.fiches.get(fid)
                # Validated level-2 documents add knowledge, but do not replace the originals.
                docs+= [d for d in state.documents.values() if d.level==2 and d.status==Status.validated and not d.deleted and d.subject==subject and d.chapter==chapter and d not in docs]
                dependency=digest({'sources':[(d.id,d.content_hash,d.text,d.kind,d.level,d.status) for d in sorted(docs,key=lambda d:d.id)],
                    'generator':GENERATOR_VERSION})
                jobkey=fid+':'+dependency
                if state.jobs.get(jobkey,{}).get('status')=='complete': continue
                state.jobs[jobkey]={'status':'running','at':now()}; self.store.save(state)
                try:
                    result=self.generator.generate(docs,old if old and old.status==Status.validated else None)
                    check_generation(result,docs)
                    f=result.fiche; f.id=fid; f.subject=subject; f.chapter=chapter
                    f.private=True; f.status=Status.pending; f.dependency_hash=dependency
                    f.stale=False; f.manual=False; f.notion_url=old.notion_url if old else ""
                    if old:
                        f.created_at=old.created_at
                        if old.manual: state.proposals['fiche:'+fid]=f.model_dump(mode='json')
                        else: state.fiches[fid]=f
                    else: state.fiches[fid]=f
                    report['fiches'].append(fid)
                    seen=[]
                    for q in result.questions:
                        q.id=stable('q',subject,chapter,q.concept_key,q.type)
                        q.subject=subject; q.chapter=chapter; q.fiche_ids=[fid]
                        q.status=Status.pending; q.private=True; q.manual=False; q.stale=False; q.dependency_hash=dependency
                        if any(duplicate(q,x) for x in seen): continue
                        prior=state.questions.get(q.id)
                        if prior: q.created_at=prior.created_at
                        if prior and prior.manual: state.proposals['question:'+q.id]=q.model_dump(mode='json')
                        elif not any(duplicate(q,x) and x.id!=q.id and not x.stale for x in state.questions.values()): state.questions[q.id]=q
                        seen.append(q); report['questions'].append(q.id)
                    # Removed concepts remain inspectable, but disappear from training.
                    active={q.id for q in seen}
                    for q in state.questions.values():
                        if fid in q.fiche_ids and q.id not in active: q.stale=True
                    state.jobs[jobkey]={'status':'complete','at':now()}
                    self.store.save(state)
                except Exception as e:
                    state.jobs[jobkey]={'status':'failed','error':type(e).__name__,'at':now()}
                    report['errors'].append({'stage':'generation','id':fid,'error':type(e).__name__})
                    self.store.save(state)
            for publisher in self.publishers:
                try: publisher.publish(state,self.store)
                except Exception as e: report['errors'].append({'stage':'publication','error':type(e).__name__})
            if not report['errors']: state.last_success=now()
            report['uncertainties']=[{'document_id':d.id,'count':sum(not u.resolved for u in d.uncertainties)} for d in state.documents.values() if any(not u.resolved for u in d.uncertainties)]
            state.reports=(state.reports+[report])[-100:]
            self.store.save(state)
            return report
