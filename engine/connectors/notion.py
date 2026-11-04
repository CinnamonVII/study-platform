"""Notion REST: explicit page allowlist, complete block pagination, immutable originals."""
from pathlib import Path
from hashlib import sha256
from urllib.parse import urlparse
import json
import httpx
from engine.models import Document, digest, Status
from engine.validation.rules import publishable

class NotionConnector:
    def __init__(self,token,config,client=None):
        self.config=config
        self.http=client or httpx.Client(base_url='https://api.notion.com/v1/',headers={
            'Authorization':'Bearer '+token,'Notion-Version':'2026-03-11','Content-Type':'application/json'},timeout=60)
    def request(self,method,path,**kw):
        # Retry read requests only. Mutations use reconciliation on the next run.
        import time
        for attempt in range(4):
            r=self.http.request(method,path,**kw)
            if method=='GET' and r.status_code in (429,502,503) and attempt<3:
                time.sleep(min(float(r.headers.get('Retry-After','1')),10)); continue
            r.raise_for_status(); return r.json()
    def children(self,page_id):
        result=[]; cursor=None
        while True:
            params={'page_size':100}
            if cursor: params['start_cursor']=cursor
            data=self.request('GET',f'blocks/{page_id}/children',params=params)
            result.extend(data['results'])
            if not data.get('has_more'): return result
            cursor=data['next_cursor']
    def blocks(self,page_id):
        out=[]
        for b in self.children(page_id):
            out.append(b)
            # Child pages are separate sources and require their own explicit allowlist entry.
            if b.get('has_children') and b['type'] not in ('child_page','child_database'):
                out.extend(self.blocks(b['id']))
        return out
    def inventory(self,store):
        docs=[]
        for src in self.config.notion_sources:
            if src.page_id in self.config.excluded_pages: continue
            page=self.request('GET',f'pages/{src.page_id}') # 403/404 fails entire scan: never infer deletion from access loss.
            docid='notion-'+src.page_id.replace('-','')
            if page.get('archived') or page.get('in_trash'):
                old=store.load().documents.get(docid)
                if old: docs.append(old.model_copy(update={'deleted':True,'status':Status.pending}))
                continue
            blocks=self.blocks(src.page_id)
            # Strip expiring URLs from the fingerprint, retain them only in private archive metadata.
            def canonical(v):
                if isinstance(v,dict): return {k:canonical(x) for k,x in v.items() if k not in ('url','expiry_time','last_edited_time','created_time','last_edited_by','created_by')}
                if isinstance(v,list): return [canonical(x) for x in v]
                return v
            h=digest(canonical(blocks))
            archive=store.root/'originals'/h; archive.mkdir(parents=True,exist_ok=True,mode=0o700)
            original=archive/'notion.json'
            if not original.exists(): original.write_text(json.dumps(blocks,ensure_ascii=False)); original.chmod(0o400)
            title=next((''.join(t.get('plain_text',t.get('text',{}).get('content','')) for t in p.get('title',[])) for p in page.get('properties',{}).values() if p.get('type')=='title'),src.chapter)
            text=[]
            attachments=[]
            for b in blocks:
                kind=b['type']; body=b.get(kind,{})
                rich=body.get('rich_text',[])
                text.append(''.join(t.get('plain_text',t.get('text',{}).get('content',t.get('equation',{}).get('expression',''))) for t in rich))
                if kind=='equation': text.append(body.get('expression',''))
                if kind in ('image','pdf','file'):
                    url=body.get(body.get('type','file'),{}).get('url','')
                    if url: attachments.append((b['id'],url,kind))
            root=Document(id=docid,title=title or src.chapter,kind=src.kind,level=src.level,provenance_reason=src.provenance_reason,
                subject=src.subject,chapter=src.chapter,modified_at=page['last_edited_time'],content_hash=h,text='\n'.join(text).strip(),
                original=str(original),origin_url=page.get('url','https://notion.so/'+src.page_id),status=src.status)
            # Do not feed the JSON archive to a vision model if the page has no text.
            if not root.text: root.text='[Page Notion sans texte ; consulter les pièces jointes.]'
            docs.append(root)
            for block_id,url,kind in attachments:
                data,mime=self.download(url)
                suffix={ 'image/png':'.png','image/jpeg':'.jpg','application/pdf':'.pdf',
                    'application/vnd.openxmlformats-officedocument.wordprocessingml.document':'.docx','text/plain':'.txt','text/markdown':'.md'}.get(mime)
                if not suffix: raise ValueError('Unsupported Notion attachment MIME type')
                bh=sha256(data).hexdigest(); folder=store.root/'originals'/bh
                folder.mkdir(parents=True,exist_ok=True,mode=0o700)
                target=folder/('source'+suffix)
                if not target.exists(): target.write_bytes(data); target.chmod(0o400)
                from engine.ingestion.files import ingest
                d=ingest(store,target,src.subject,src.chapter,src.kind,src.level,src.provenance_reason,
                    'notion-'+block_id.replace('-',''))
                d.origin_url=root.origin_url; d.status=src.status
                docs.append(d)
        return docs
    def download(self,url):
        parsed=urlparse(url)
        if parsed.scheme!='https' or parsed.hostname not in self.config.attachment_hosts or parsed.username:
            raise ValueError('Attachment host is not explicitly authorized')
        with httpx.Client(timeout=60,follow_redirects=False) as client:
            with client.stream('GET',url) as r:
                r.raise_for_status()
                if r.is_redirect: raise ValueError('Attachment redirects are not allowed')
                data=bytearray()
                for chunk in r.iter_bytes():
                    data.extend(chunk)
                    if len(data)>self.config.max_document_bytes: raise ValueError('Attachment is too large')
                return bytes(data),r.headers.get('content-type','').split(';')[0]
    def publish(self,state,store):
        from engine.export.notion_blocks import blocks_for
        for f in state.fiches.values():
            if not publishable(f,state): continue
            destination=self.config.notion_destinations.get(f.subject)
            if not destination: continue
            version=digest(f.model_dump(mode='json',exclude={'notion_url','updated_at'}))
            key='notion:'+f.id+':'+version
            if state.jobs.get(key,{}).get('status')=='complete': continue
            mapping='notion-page:'+f.id
            title='Fiche ['+f.id+']'
            existing=state.jobs.get(mapping,{})
            page_id=existing.get('page_id')
            desired=blocks_for(f,state)
            if not page_id:
                page=next((b for b in self.children(destination) if b['type']=='child_page' and b['child_page']['title']==title),None)
                if page: page_id=page['id']
                else:
                    result=self.request('POST','pages',json={'parent':{'page_id':destination},'properties':{'title':{'title':[{'text':{'content':title}}]}},'children':desired})
                    page_id=result['id']
                # Persist destination immediately; retry finds an already-created page by deterministic title.
                existing={'page_id':page_id}
                state.jobs[mapping]=existing;store.save(state)
            current=self.children(page_id)
            def clean(block):
                kind=block['type'];body=block[kind]
                return {'object':'block','type':kind,kind:{k:v for k,v in body.items() if k in ('rich_text','color','icon')}}
            def canonical(block):
                b=clean(block);kind=b['type']
                parts=[]
                for r in b[kind].get('rich_text',[]):
                    annotation=r.get('annotations',{})
                    text=r.get('text',{})
                    parts.append((r['type'],text.get('content',r.get('equation',{}).get('expression','')),
                        (text.get('link') or {}).get('url'),
                        tuple(annotation.get(k,False) for k in ('bold','italic','strikethrough','underline','code')),
                        annotation.get('color','default')))
                return (kind,parts)
            current_signature=[canonical(b) for b in current]
            desired_signature=[canonical(b) for b in desired]
            previous=existing.get('signature')
            # JSON restores tuples as lists.
            normalized=lambda x:json.dumps(x,ensure_ascii=False)
            current_json=normalized(current_signature);desired_json=normalized(desired_signature)
            if current_json!=desired_json:
                if previous is None or len(current)>len(desired) and not previous:
                    state.proposals['notion:'+f.id]={'message':'Existing Notion content differs. Review it before replacing any block.','page_id':page_id}
                    store.save(state);continue
                # Reconcile partial updates after a crash, one block at a time.
                for i,block in enumerate(current):
                    sig=canonical(block)
                    old=previous[i] if i<len(previous) else None
                    new=desired_signature[i] if i<len(desired_signature) else None
                    if normalized(sig) not in (normalized(old),normalized(new)):
                        state.proposals['notion:'+f.id]={'message':'Manual Notion edits preserved; automatic update paused.','page_id':page_id}
                        store.save(state);break
                else:
                    for i,body in enumerate(desired):
                        if i<len(current):
                            if canonical(current[i])!=desired_signature[i]: self.request('PATCH','blocks/'+current[i]['id'],json={body['type']:body[body['type']]})
                        else: self.request('PATCH','blocks/'+page_id+'/children',json={'children':[body]})
                    # Generated trailing blocks are retained if a shorter update is proposed; never delete human content.
                    if len(current)>len(desired):
                        state.proposals['notion:'+f.id]={'message':'Shorter update requires review of trailing generated blocks.','page_id':page_id}
                        store.save(state);continue
                    verify=self.children(page_id)
                    if normalized([canonical(b) for b in verify])!=desired_json: raise ValueError('Notion write verification failed')
                    current_signature=desired_signature
                    existing['signature']=current_signature
                    f.notion_url='https://notion.so/'+page_id.replace('-','')
                    state.jobs[key]={'status':'complete','url':f.notion_url}
                    store.save(state);continue
                continue
            existing['signature']=desired_signature
            f.notion_url='https://notion.so/'+page_id.replace('-','')
            state.jobs[key]={'status':'complete','url':f.notion_url}
            store.save(state)
