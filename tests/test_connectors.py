import json
import httpx
import pytest
from engine.connectors.github import GitHubPublisher
from engine.connectors.notion import NotionConnector
from engine.models import Config, SourceConfig
from engine.sync.store import Store
from engine.sync.pipeline import Pipeline
from engine.demo import DemoGenerator,documents

def test_public_github_repository_rejected_before_any_write(tmp_path):
    calls=[]
    def handler(r):
        calls.append(r.method);return httpx.Response(200,json={'private':False},request=r)
    pub=GitHubPublisher('x','CinnamonVII/study-platform',httpx.Client(base_url='https://api.github.com/',transport=httpx.MockTransport(handler)))
    s=Store(tmp_path)
    with pytest.raises(ValueError,match='PRIVATE'):pub.publish(s.load(),s)
    assert calls==['GET']

def test_notion_pagination_and_access_loss_not_deletion(tmp_path):
    calls=[]
    def handler(r):
        calls.append(str(r.url))
        if 'start_cursor' in str(r.url):return httpx.Response(200,json={'results':[{'id':'b2','type':'paragraph','paragraph':{'rich_text':[{'plain_text':'b'}]}}],'has_more':False},request=r)
        return httpx.Response(200,json={'results':[{'id':'b1','type':'paragraph','paragraph':{'rich_text':[{'plain_text':'a'}]}}],'has_more':True,'next_cursor':'two'},request=r)
    client=httpx.Client(base_url='https://api.notion.com/v1/',transport=httpx.MockTransport(handler))
    notion=NotionConnector('x',Config(),client);assert len(notion.children('a'))==2 and len(calls)==2
    bad=httpx.Client(base_url='https://api.notion.com/v1/',transport=httpx.MockTransport(lambda r:httpx.Response(403,request=r)))
    notion=NotionConnector('x',Config(notion_sources=[SourceConfig(page_id='page',subject='SES',chapter='X')]),bad)
    s=Store(tmp_path);d=documents()[0];d.id='notion-old';Pipeline(s,DemoGenerator()).run([d])
    with pytest.raises(httpx.HTTPStatusError):notion.inventory(s)
    assert not s.load().documents[d.id].deleted

def test_notion_ssrf_rejected():
    n=NotionConnector('x',Config())
    for url in ['http://127.0.0.1/secret','https://evil.example/private','file:///etc/passwd']:
        with pytest.raises(ValueError):n.download(url)

def test_notion_failed_write_not_checkpointed(tmp_path):
    def handler(r):
        if r.method=='GET':return httpx.Response(200,json={'results':[],'has_more':False},request=r)
        return httpx.Response(503,request=r)
    n=NotionConnector('x',Config(notion_destinations={'Mathématiques':'parent'}),httpx.Client(base_url='https://api.notion.com/v1/',transport=httpx.MockTransport(handler)))
    s=Store(tmp_path);Pipeline(s,DemoGenerator()).run([documents()[0]]);state=s.load()
    for x in [*state.documents.values(),*state.fiches.values()]:x.status='validated'
    with pytest.raises(httpx.HTTPStatusError):n.publish(state,s)
    assert not any(k.startswith('notion:') for k in s.load().jobs)

def test_notion_create_retry_update_and_preserve_manual_content(tmp_path):
    remote={'pages':{},'calls':0}
    def handler(r):
        path=r.url.path;body=json.loads(r.content) if r.content else {}
        if path.endswith('/blocks/parent/children'):
            rows=[{'id':key,'type':'child_page','child_page':{'title':v['title']}} for key,v in remote['pages'].items()]
            return httpx.Response(200,json={'results':rows,'has_more':False},request=r)
        if r.method=='POST' and path.endswith('/pages'):
            remote['calls']+=1;key='page-generated';children=body['children']
            for i,b in enumerate(children):b['id']='block-'+str(i)
            remote['pages'][key]={'title':body['properties']['title']['title'][0]['text']['content'],'blocks':children}
            return httpx.Response(200,json={'id':key,'url':'https://notion.so/'+key},request=r)
        if path.endswith('/blocks/page-generated/children') and r.method=='GET':
            return httpx.Response(200,json={'results':remote['pages']['page-generated']['blocks'],'has_more':False},request=r)
        if r.method=='PATCH' and '/blocks/block-' in path:
            i=int(path.split('block-')[-1]);remote['pages']['page-generated']['blocks'][i].update(body)
            return httpx.Response(200,json={'id':'block-'+str(i)},request=r)
        raise AssertionError((r.method,path))
    n=NotionConnector('x',Config(notion_destinations={'Mathématiques':'parent'}),httpx.Client(base_url='https://api.notion.com/v1/',transport=httpx.MockTransport(handler)))
    s=Store(tmp_path);Pipeline(s,DemoGenerator()).run([documents()[0]]);state=s.load()
    for x in [*state.documents.values(),*state.fiches.values()]:x.status='validated'
    n.publish(state,s);assert remote['calls']==1
    n.publish(state,s);assert remote['calls']==1
    f=next(iter(state.fiches.values()));f.title='Titre corrigé';n.publish(state,s)
    assert remote['calls']==1 and remote['pages']['page-generated']['blocks'][0]['heading_1']['rich_text'][0]['text']['content']=='Titre corrigé'
    # Lost local checkpoint after creation: remote stable-title lookup must avoid a second page.
    state.jobs={k:v for k,v in state.jobs.items() if not k.startswith('notion:') and not k.startswith('notion-page:')}
    n.publish(state,s);assert remote['calls']==1
    # A human edits a managed paragraph, then a new generated title arrives.
    remote['pages']['page-generated']['blocks'][0]['heading_1']['rich_text'][0]['text']['content']='Correction humaine Notion'
    f.title='Nouvelle version';n.publish(state,s)
    assert 'notion:'+f.id in state.proposals
    assert remote['pages']['page-generated']['blocks'][0]['heading_1']['rich_text'][0]['text']['content']=='Correction humaine Notion'

def test_github_source_ingestion_and_deletion(tmp_path):
    from engine.connectors.github import GitHubSource
    from engine.models import GitHubSourceConfig
    import base64
    remote={'deleted':False}
    def handler(r):
        if '/git/trees/' in r.url.path:return httpx.Response(200,json={'truncated':False,'tree':[] if remote['deleted'] else [{'path':'notes.md','type':'blob','sha':'abc','size':20}]},request=r)
        return httpx.Response(200,json={'content':base64.b64encode('Suite arithmétique : raison constante.\nu₀ = 3\nu₁ = 5'.encode()).decode()},request=r)
    connector=GitHubSource('x',httpx.Client(base_url='https://api.github.com/',transport=httpx.MockTransport(handler)))
    s=Store(tmp_path);src=GitHubSourceConfig(repository='owner/private',path='notes.md',subject='Mathématiques',chapter='Suites',level=1,provenance_reason='Notes personnelles')
    docs=connector.inventory(s,[src]);assert docs[0].level==1 and docs[0].original
    Pipeline(s,DemoGenerator()).run(docs);remote['deleted']=True
    docs=connector.inventory(s,[src]);assert docs[0].deleted
    Pipeline(s,DemoGenerator()).run(docs);assert all(q.stale for q in s.load().questions.values())
