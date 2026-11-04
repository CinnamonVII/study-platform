"""Atomic multi-file commits into a private review branch; never push private content to a public repo."""
import base64
import json
import httpx
from engine.models import digest
from engine.validation.rules import publishable

class GitHubPublisher:
    def __init__(self,token,repository,client=None):
        self.repo=repository
        self.http=client or httpx.Client(base_url='https://api.github.com/',headers={'Authorization':'Bearer '+token,
            'Accept':'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28'},timeout=60)
    def req(self,method,path,**kw):
        r=self.http.request(method,'repos/'+self.repo+'/'+path,**kw); r.raise_for_status(); return r.json()
    def publish(self,state,store):
        meta=self.req('GET','')
        if not meta.get('private'): raise ValueError('Private school content requires a PRIVATE data repository')
        # Fingerprint only approved contents, not report timestamps or publication markers.
        fiches={k:v for k,v in state.fiches.items() if publishable(v,state)}
        questions={k:v for k,v in state.questions.items() if publishable(v,state)}
        if not fiches and not questions: return
        data={'schema_version':'2.0','fiches':{k:v.model_dump(mode='json') for k,v in fiches.items()},'questions':{k:v.model_dump(mode='json') for k,v in questions.items()}}
        version=digest(data); key='github:'+version
        if state.jobs.get(key,{}).get('status')=='complete': return
        branch='study/generated-'+version[:16]
        refs=self.req('GET','git/ref/heads/'+meta['default_branch'])
        base=refs['object']['sha']
        try:
            head=self.req('GET','git/ref/heads/'+branch)['object']['sha']
        except httpx.HTTPStatusError as e:
            if e.response.status_code!=404: raise
            head=base
        commit=self.req('GET','git/commits/'+head)
        # Resume after a commit/ref write but before the local checkpoint.
        if commit['message']!='study: '+version:
            from engine.export.render import export_fiche
            elements=[{'path':'content/snapshot.json','mode':'100644','type':'blob','content':json.dumps(data,ensure_ascii=False,indent=2)}]
            for f in fiches.values():
                folder=export_fiche(f,state,store.root/'exports')
                for p in folder.iterdir():
                    blob=self.req('POST','git/blobs',json={'content':base64.b64encode(p.read_bytes()).decode(),'encoding':'base64'})
                    elements.append({'path':'content/fiches/'+f.id+'/'+p.name,'mode':'100644','type':'blob','sha':blob['sha']})
            tree=self.req('POST','git/trees',json={'base_tree':commit['tree']['sha'],'tree':elements})
            new=self.req('POST','git/commits',json={'message':'study: '+version,'tree':tree['sha'],'parents':[head]})
            if head==base:
                try: self.req('POST','git/refs',json={'ref':'refs/heads/'+branch,'sha':new['sha']})
                except httpx.HTTPStatusError as e:
                    if e.response.status_code!=422: raise
                    current=self.req('GET','git/ref/heads/'+branch)['object']['sha']
                    if current!=new['sha']: raise RuntimeError('Branch changed concurrently; review before retry')
            else:
                # Never overwrite manual changes to an existing generated branch.
                raise RuntimeError('Generated branch has manual changes; preserve and review')
        prs=self.req('GET','pulls',params={'head':self.repo.split('/')[0]+':'+branch,'state':'all'})
        if prs: url=prs[0]['html_url']
        else:
            pr=self.req('POST','pulls',json={'title':'Révisions : contenus validés '+version[:8], 'head':branch,'base':meta['default_branch'],
                'draft':True,'body':'Sorties validées, sources conservées dans Notion. Vérifier les corrections manuelles avant fusion.'})
            url=pr['html_url']
        state.jobs[key]={'status':'complete','url':url,'branch':branch}; store.save(state)


class GitHubSource:
    def __init__(self, token, client=None):
        self.http=client or httpx.Client(base_url='https://api.github.com/',headers={'Authorization':'Bearer '+token,'Accept':'application/vnd.github+json'},timeout=60)
    def inventory(self,store,sources):
        from engine.models import stable,Status
        from engine.ingestion.files import ingest,SUPPORTED
        from pathlib import Path
        from urllib.parse import quote
        docs=[]
        for src in sources:
            if Path(src.path).suffix.lower() not in SUPPORTED or '..' in Path(src.path).parts: raise ValueError('Unsupported GitHub source path')
            prefix='repos/'+src.repository+'/'
            def get(path,**kw):
                r=self.http.get(prefix+path,**kw);r.raise_for_status();return r.json()
            # A successful, complete tree proves deletion. A 404/access error never does.
            tree=get('git/trees/'+quote(src.ref,safe=''),params={'recursive':'1'})
            if tree.get('truncated'): raise ValueError('GitHub tree truncated: cannot establish complete inventory')
            entry=next((x for x in tree['tree'] if x['path']==src.path and x['type']=='blob'),None)
            did=stable('github',src.repository,src.path)
            if entry is None:
                old=store.load().documents.get(did)
                if old: docs.append(old.model_copy(update={'deleted':True,'status':Status.pending}))
                continue
            if entry.get('size',0)>20_000_000: raise ValueError('GitHub source too large')
            blob=get('git/blobs/'+entry['sha'])
            data=base64.b64decode(blob['content'])
            if len(data)>20_000_000: raise ValueError('GitHub source too large')
            import hashlib
            folder=store.root/'originals'/hashlib.sha256(data).hexdigest()
            folder.mkdir(parents=True,exist_ok=True,mode=0o700);path=folder/('source'+Path(src.path).suffix.lower())
            if not path.exists():path.write_bytes(data);path.chmod(0o400)
            d=ingest(store,path,src.subject,src.chapter,src.kind,src.level,src.provenance_reason,did)
            d.title=Path(src.path).name;d.origin_url='https://github.com/'+src.repository+'/blob/'+quote(src.ref,safe='')+'/'+quote(src.path,safe='/')
            docs.append(d)
        return docs
