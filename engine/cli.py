import argparse
import json
import os
from pathlib import Path
from engine import service
from engine.models import Snapshot, Status
from engine.sync.pipeline import Pipeline
from engine.demo import DemoGenerator,documents

def main():
    p=argparse.ArgumentParser(description='Study Platform private pipeline')
    sub=p.add_subparsers(dest='command',required=True)
    sub.add_parser('demo'); sub.add_parser('daily').add_argument('--preview',action='store_true')
    sub.add_parser('generate'); sub.add_parser('export'); sub.add_parser('schema')
    imp=sub.add_parser('import'); imp.add_argument('path'); imp.add_argument('--subject',required=True); imp.add_argument('--chapter',required=True)
    imp.add_argument('--kind',default='cours'); imp.add_argument('--level',type=int,choices=[1,2]); imp.add_argument('--reason',default=''); imp.add_argument('--id')
    args=p.parse_args(); store=service.store()
    if args.command=='demo':
        if store.load().documents and not store.load().demo: p.error('Use a separate empty STUDY_STATE directory for the demonstration')
        report=Pipeline(store,DemoGenerator()).run(documents())
        def approve(s):
            s.demo=True
            for item in [*s.documents.values(),*s.fiches.values(),*s.questions.values()]: item.status=Status.validated; item.private=False
        store.mutate(approve); print(json.dumps(report,ensure_ascii=False))
    elif args.command=='daily':
        report=service.daily(store,preview=args.preview); print(json.dumps(report,ensure_ascii=False))
        if report.get('errors'): raise SystemExit(1)
    elif args.command=='import':
        from engine.ingestion.files import ingest
        def add(s):
            doc=ingest(store,args.path,args.subject,args.chapter,args.kind,args.level,args.reason,args.id)
            old=s.documents.get(doc.id)
            if old and old.content_hash==doc.content_hash: return
            if old and old.manual:
                from engine.models import Uncertainty
                s.proposals['source:'+doc.id]=doc.model_dump(mode='json')
                old.content_hash=doc.content_hash;old.original=doc.original;old.status=Status.pending
                old.uncertainties.append(Uncertainty(message='Source reimported: reconcile preserved manual transcription'))
            else: s.documents[doc.id]=doc
            for item in [*s.fiches.values(),*s.questions.values()]:
                if any(r.document_id==doc.id for r in item.sources): item.stale=True
            print(doc.id)
        store.mutate(add)
    elif args.command=='generate':
        from engine.generation.provider import AIGenerator
        from engine.transcription.multimodal import ModelProvider
        r=Pipeline(store,AIGenerator(ModelProvider(os.environ.get('OPENAI_MODEL','')))).run([])
        print(json.dumps(r,ensure_ascii=False))
        if r['errors']: raise SystemExit(1)
    elif args.command=='export':
        from engine.export.render import export_fiche
        s=store.load()
        for f in s.fiches.values(): print(export_fiche(f,s,store.root/'exports'))
    elif args.command=='schema':
        out=Path('schemas/snapshot-v2.json'); out.parent.mkdir(exist_ok=True)
        out.write_text(json.dumps(Snapshot.model_json_schema(mode="serialization"),ensure_ascii=False,indent=2)+'\n')
if __name__=='__main__': main()
