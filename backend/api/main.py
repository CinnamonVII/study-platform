from __future__ import annotations
from pathlib import Path
from hmac import compare_digest
import os
from fastapi import FastAPI, Depends, Header, HTTPException, Response
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from pydantic import BaseModel, Field
from engine.models import Status, Review, Uncertainty, Fiche, Question, Document, digest, now
from engine.repetition import schedule
from engine.sync.store import BusyError
from engine.validation.rules import publishable
from engine import service

class Edit(BaseModel):
    revision: str
    changes: dict
class Grade(BaseModel):
    grade: int = Field(ge=0,le=5)
class ReportError(BaseModel):
    message: str = Field(min_length=1,max_length=2000)

def create_app(store=None,config=None,token=None,sync_fn=None):
    store=store or service.store(); config=config or service.config()
    token=token if token is not None else os.environ.get('STUDY_API_TOKEN','')
    app=FastAPI(title='Study Platform private API',version='0.2.0',docs_url=None,redoc_url=None)
    app.add_middleware(CORSMiddleware,allow_origins=config.allowed_origins,allow_credentials=False,
        allow_methods=['GET','POST','PATCH','OPTIONS'],allow_headers=['Authorization','Content-Type'],expose_headers=['ETag'])
    def auth(authorization:str=Header(default='')):
        if len(token)<32: raise HTTPException(503,'Server authentication is not configured')
        if not compare_digest(authorization,'Bearer '+token): raise HTTPException(401,'Authentication required')
    @app.middleware('http')
    async def headers(request,call_next):
        result=await call_next(request)
        result.headers['Cache-Control']='no-store'
        result.headers['X-Content-Type-Options']='nosniff'
        result.headers['Referrer-Policy']='no-referrer'
        return result
    @app.exception_handler(BusyError)
    async def busy(req,exc):
        from fastapi.responses import JSONResponse
        return JSONResponse(status_code=409,content={'detail':str(exc)})
    @app.exception_handler(ValueError)
    async def invalid(req,exc):
        from fastapi.responses import JSONResponse
        return JSONResponse(status_code=422,content={'detail':str(exc)})
    def revision(s): return digest(s.model_dump(mode='json'))
    @app.get('/health')
    def health(): return {'status':'ok','schema_version':'2.0'}
    @app.get('/v1/snapshot',dependencies=[Depends(auth)])
    def snapshot(response:Response):
        state=store.load(); response.headers['ETag']=revision(state)
        return {'revision':revision(state),'snapshot':state.model_dump(mode='json')}
    @app.patch('/v1/{collection}/{item_id}',dependencies=[Depends(auth)])
    def edit(collection:str,item_id:str,body:Edit):
        if collection not in ('documents','fiches','questions'): raise HTTPException(404)
        allowed={'documents':{'text','level','provenance_reason','kind','subject','chapter','status','uncertainties'},
            'fiches':{'title','blocks','takeaways','recall','status'},
            'questions':{'prompt','answer','explanation','choices','correct_indices','traps','steps','rubric','difficulty','status'}}
        if set(body.changes)-allowed[collection]: raise HTTPException(422,'Protected fields cannot be changed')
        def mutate(state):
            if body.revision!=revision(state): raise HTTPException(409,'Data changed. Reload before editing.')
            entries=getattr(state,collection)
            if item_id not in entries: raise HTTPException(404)
            before=entries[item_id]
            data=before.model_dump(mode='json'); data.update(body.changes); data['manual']=True
            if collection=='documents':
                obj=Document.model_validate(data)
                if obj.status==Status.validated and any(not u.resolved for u in obj.uncertainties): raise HTTPException(422,'Resolve transcription uncertainties first')
            else:
                obj=(Fiche if collection=='fiches' else Question).model_validate(data)
                if obj.status==Status.validated:
                    obj.stale=False
                    if not publishable(obj,state): raise HTTPException(422,'Validate all source documents first')
            entries[item_id]=obj
            if collection=='documents':
                for item in [*state.fiches.values(),*state.questions.values()]:
                    if any(r.document_id==item_id for r in item.sources): item.stale=True
            return {'saved':item_id}
        return store.mutate(mutate)
    @app.post('/v1/questions/{qid}/review',dependencies=[Depends(auth)])
    def review(qid:str,body:Grade):
        def mutate(state):
            q=state.questions.get(qid)
            if q is None: raise HTTPException(404)
            if not state.demo and not publishable(q,state): raise HTTPException(422,'Question is awaiting validation')
            r=schedule(state.reviews.get(qid,Review(question_id=qid)),body.grade)
            state.reviews[qid]=r; return r
        return store.mutate(mutate)
    @app.post('/v1/questions/{qid}/report',dependencies=[Depends(auth)])
    def report(qid:str,body:ReportError):
        def mutate(state):
            if qid not in state.questions: raise HTTPException(404)
            state.questions[qid].status=Status.pending
            state.proposals['report:'+qid]={'message':body.message,'at':now()}
            return {'reported':qid}
        return store.mutate(mutate)
    @app.post('/v1/sync',dependencies=[Depends(auth)])
    def sync(preview:bool=False): return (sync_fn or service.daily)(store,config,preview=preview)
    @app.post('/v1/proposals/{key:path}/accept',dependencies=[Depends(auth)])
    def accept(key:str,body:Edit):
        def mutate(state):
            if body.revision!=revision(state): raise HTTPException(409,'Reload first')
            if key not in state.proposals: raise HTTPException(404)
            prefix,_,id=key.partition(':'); data=state.proposals[key]
            if prefix not in ('fiche','question'): raise HTTPException(422,'Reconcile source transcription in the document editor')
            obj=(Fiche if prefix=='fiche' else Question).model_validate(data); obj.manual=True; obj.status=Status.pending
            getattr(state,'fiches' if prefix=='fiche' else 'questions')[id]=obj
            del state.proposals[key]; return {'accepted':id}
        return store.mutate(mutate)
    @app.post('/v1/proposals/{key:path}/reject',dependencies=[Depends(auth)])
    def reject(key:str,body:Edit):
        def mutate(state):
            if body.revision!=revision(state): raise HTTPException(409,'Reload first')
            state.proposals.pop(key,None); return {'rejected':key}
        return store.mutate(mutate)
    @app.get('/v1/documents/{item_id}/original',dependencies=[Depends(auth)])
    def original(item_id:str):
        doc=store.load().documents.get(item_id)
        if doc is None or not doc.original: raise HTTPException(404)
        p=Path(doc.original).resolve()
        if not p.is_relative_to((store.root/'originals').resolve()): raise HTTPException(403)
        return FileResponse(p,filename=doc.id+p.suffix)
    @app.get('/v1/fiches/{item_id}/export/{fmt}',dependencies=[Depends(auth)])
    def export(item_id:str,fmt:str):
        if fmt not in ('pdf','md','tex'): raise HTTPException(404)
        state=store.load(); f=state.fiches.get(item_id)
        if f is None: raise HTTPException(404)
        from engine.export.render import export_fiche
        folder=export_fiche(f,state,store.root/'exports')
        return FileResponse(folder/('fiche.'+fmt),filename=f.id+'.'+fmt)
    return app

app=create_app()
