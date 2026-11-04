from __future__ import annotations
from datetime import datetime, timezone
from enum import StrEnum
from hashlib import sha256
import json
from typing import Literal
from pydantic import BaseModel, ConfigDict, Field, model_validator

GENERATOR_VERSION = 'study-0.2.0'

def now() -> str:
    return datetime.now(timezone.utc).isoformat()

def digest(value) -> str:
    return sha256(json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(',', ':')).encode()).hexdigest()

def stable(prefix: str, *parts: str) -> str:
    return prefix + '-' + digest(parts)[:24]

class Model(BaseModel):
    model_config = ConfigDict(extra='forbid', validate_assignment=True, json_schema_serialization_defaults_required=True)

class Status(StrEnum):
    pending = 'pending'
    validated = 'validated'
    rejected = 'rejected'

class Reference(Model):
    document_id: str
    page: int | None = Field(default=None, ge=1)
    region: list[float] | None = None
    quote: str = ''

class Uncertainty(Model):
    message: str
    page: int | None = None
    region: list[float] | None = None
    resolved: bool = False

class Document(Model):
    id: str
    title: str
    kind: Literal['cours','devoir','exercice','correction','fiche','methode','annale']
    level: Literal[1,2] | None = None
    provenance_reason: str = ''
    subject: str
    chapter: str
    modified_at: str = Field(default_factory=now)
    content_hash: str
    status: Status = Status.pending
    sources: list[Reference] = Field(default_factory=list)
    text: str = ''
    original: str = ''
    origin_url: str = ''
    private: bool = True
    deleted: bool = False
    uncertainties: list[Uncertainty] = Field(default_factory=list)
    manual: bool = False
    derived: bool = False
    @model_validator(mode='after')
    def provenance(self):
        if self.level == 1 and not self.provenance_reason.strip():
            raise ValueError('Level 1 requires an explicit provenance justification')
        if self.derived and self.level != 2:
            raise ValueError('Generated content must remain level 2')
        if self.level == 2 and not self.sources and self.derived:
            raise ValueError('Generated content requires sources')
        return self

class Block(Model):
    kind: Literal['definition','explanation','steps','mechanism','comparison','timeline','warning','formula']
    title: str
    text: str = ''
    items: list[str] = Field(default_factory=list)
    columns: list[str] = Field(default_factory=list)
    rows: list[list[str]] = Field(default_factory=list)
    sources: list[Reference] = Field(default_factory=list)

class Question(Model):
    id: str
    concept_key: str
    subject: str
    chapter: str
    type: Literal['short','mcq','open','math','method','flashcard']
    cognitive: Literal['memory','understanding','application','reasoning']
    prompt: str
    answer: str
    explanation: str
    choices: list[str] = Field(default_factory=list)
    correct_indices: list[int] = Field(default_factory=list)
    traps: list[str] = Field(default_factory=list)
    steps: list[str] = Field(default_factory=list)
    rubric: list[str] = Field(default_factory=list)
    difficulty: int = Field(ge=1, le=5)
    sources: list[Reference] = Field(min_length=1)
    fiche_ids: list[str] = Field(default_factory=list)
    status: Status = Status.pending
    created_at: str = Field(default_factory=now)
    updated_at: str = Field(default_factory=now)
    generator_version: str = GENERATOR_VERSION
    dependency_hash: str = ''
    manual: bool = False
    stale: bool = False
    private: bool = True
    @model_validator(mode='after')
    def mcq(self):
        if self.type == 'mcq' and (len(self.choices)<2 or not self.correct_indices or any(i<0 or i>=len(self.choices) for i in self.correct_indices)):
            raise ValueError('MCQ requires choices and valid correct indices')
        if self.type == 'math' and not self.steps:
            raise ValueError('Math questions require intermediate steps')
        if self.type in ('open','method') and not self.rubric:
            raise ValueError('Open and method questions require a rubric')
        return self

class Fiche(Model):
    id: str
    title: str
    subject: str
    chapter: str
    level: Literal[2] = 2
    blocks: list[Block] = Field(min_length=1)
    takeaways: list[str] = Field(min_length=1)
    recall: list[str] = Field(min_length=1)
    sources: list[Reference] = Field(min_length=1)
    status: Status = Status.pending
    created_at: str = Field(default_factory=now)
    updated_at: str = Field(default_factory=now)
    generator_version: str = GENERATOR_VERSION
    dependency_hash: str = ''
    notion_url: str = ''
    manual: bool = False
    stale: bool = False
    private: bool = True

class Transcription(Model):
    text: str
    uncertainties: list[Uncertainty]
    observations: list[str]

class Generation(Model):
    fiche: Fiche
    questions: list[Question] = Field(min_length=1)

class Review(Model):
    question_id: str
    due: str = Field(default_factory=now)
    interval: int = 0
    ease: float = 2.5
    repetitions: int = 0
    last_grade: int | None = None

class Snapshot(Model):
    schema_version: Literal['2.0'] = '2.0'
    documents: dict[str,Document] = Field(default_factory=dict)
    fiches: dict[str,Fiche] = Field(default_factory=dict)
    questions: dict[str,Question] = Field(default_factory=dict)
    reviews: dict[str,Review] = Field(default_factory=dict)
    proposals: dict[str,dict] = Field(default_factory=dict)
    jobs: dict[str,dict] = Field(default_factory=dict)
    reports: list[dict] = Field(default_factory=list)
    last_success: str | None = None
    demo: bool = False

class SourceConfig(Model):
    page_id: str
    subject: str
    chapter: str
    kind: Literal['cours','devoir','exercice','correction','fiche','methode','annale'] = 'cours'
    level: Literal[1,2] | None = None
    provenance_reason: str = ''
    status: Status = Status.pending

class GitHubSourceConfig(Model):
    repository: str
    path: str
    ref: str = 'main'
    subject: str
    chapter: str
    kind: Literal['cours','devoir','exercice','correction','fiche','methode','annale'] = 'cours'
    level: Literal[1,2] | None = None
    provenance_reason: str = ''

class Config(Model):
    github_sources: list[GitHubSourceConfig] = Field(default_factory=list)
    notion_sources: list[SourceConfig] = Field(default_factory=list)
    notion_destinations: dict[str,str] = Field(default_factory=dict)
    excluded_pages: list[str] = Field(default_factory=list)
    github_repository: str = ''
    allowed_origins: list[str] = Field(default_factory=list)
    attachment_hosts: list[str] = Field(default_factory=lambda:['prod-files-secure.s3.us-west-2.amazonaws.com','s3.us-west-2.amazonaws.com'])
    max_document_bytes: int = 20_000_000
