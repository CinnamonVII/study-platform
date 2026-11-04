import re
import unicodedata
from difflib import SequenceMatcher
from engine.models import Status

def normalized(text):
    return re.sub(r'\W+',' ',unicodedata.normalize('NFKC',text).casefold()).strip()

def duplicate(a,b):
    return a.subject==b.subject and a.chapter==b.chapter and a.type==b.type and SequenceMatcher(None,normalized(a.prompt),normalized(b.prompt)).ratio()>.91

def check_generation(result,docs):
    known={d.id:d for d in docs}
    originals={d.id for d in docs if d.level==1 or d.kind in ('devoir','exercice','correction','methode','annale') and not d.derived}
    for item in [result.fiche,*result.questions]:
        for ref in item.sources:
            if ref.document_id not in known: raise ValueError('Invented source reference: '+ref.document_id)
            if ref.quote and ref.quote not in known[ref.document_id].text:
                raise ValueError('Source quote is not present in the transcription')
        if hasattr(item,'concept_key') and not any(r.document_id in originals for r in item.sources):
            raise ValueError('Questions must be checked against original sources')
    for block in result.fiche.blocks:
        for r in block.sources:
            if r.document_id not in known: raise ValueError('Invented block source')

def publishable(item,state):
    if item.status != Status.validated or item.stale: return False
    for r in item.sources:
        d=state.documents.get(r.document_id)
        if d is None or d.deleted or d.status != Status.validated or any(not u.resolved for u in d.uncertainties): return False
    return True
