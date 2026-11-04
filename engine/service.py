import os
from pathlib import Path
from engine.models import Config
from engine.sync.store import Store
from engine.sync.pipeline import Pipeline
from engine.generation.provider import AIGenerator
from engine.transcription.multimodal import ModelProvider
from engine.connectors.notion import NotionConnector
from engine.connectors.github import GitHubPublisher,GitHubSource

def config():
    path=Path(os.environ.get('STUDY_CONFIG','config.json'))
    return Config.model_validate_json(path.read_text()) if path.exists() else Config()

def store():
    return Store(os.environ.get('STUDY_STATE',str(Path.home()/'.local/share/study-platform')))

def daily(s=None,c=None,preview=False):
    s=s or store(); c=c or config()
    if not c.notion_sources and not c.github_sources: raise ValueError('Configure an explicit source allowlist first')
    notion=NotionConnector(os.environ.get('NOTION_TOKEN',''),c) if c.notion_sources or c.notion_destinations else None
    # Inventory outside the mutation lock, but before checkpointing. A failed scan does not delete anything.
    docs=notion.inventory(s) if c.notion_sources else []
    if c.github_sources: docs+=GitHubSource(os.environ['STUDY_CONTENT_TOKEN']).inventory(s,c.github_sources)
    if preview:
        old=s.load()
        return {'changed':[d.id for d in docs if d.id not in old.documents or d.content_hash!=old.documents[d.id].content_hash],
            'missing':[d.id for d in old.documents.values() if d.id.startswith('notion-') and d.id not in {x.id for x in docs}]}
    generator=AIGenerator(ModelProvider(os.environ.get('OPENAI_MODEL','')))
    publishers=[notion] if notion and c.notion_destinations else []
    if c.github_repository: publishers.append(GitHubPublisher(os.environ['STUDY_CONTENT_TOKEN'],c.github_repository))
    return Pipeline(s,generator,publishers).run(docs,complete_inventory=True)
