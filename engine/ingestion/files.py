from pathlib import Path
from hashlib import sha256
import os
from engine.models import Document, stable

SUPPORTED = {'.png','.jpg','.jpeg','.pdf','.md','.txt','.docx'}

def ingest(store, path, subject, chapter, kind='cours', level=None, reason='', document_id=None):
    p=Path(path); ext=p.suffix.lower()
    if ext not in SUPPORTED: raise ValueError('Unsupported document format')
    if p.stat().st_size > 20_000_000: raise ValueError('Document exceeds 20 MB')
    data=p.read_bytes(); h=sha256(data).hexdigest()
    archive=store.root/'originals'/h
    archive.mkdir(parents=True,exist_ok=True,mode=0o700)
    target=archive/('source'+ext)
    if not target.exists():
        target.write_bytes(data); target.chmod(0o400)
    text=''
    if ext in ('.md','.txt'): text=data.decode('utf-8')
    elif ext=='.docx':
        from docx import Document as Docx
        d=Docx(str(target)); text='\n'.join([p.text for p in d.paragraphs]+[' | '.join(c.text for c in row.cells) for t in d.tables for row in t.rows])
    # Images and PDFs intentionally go through the multimodal path, including PDFs with scans.
    return Document(id=document_id or stable('doc',str(p.resolve())),title=p.name,kind=kind,level=level,
        provenance_reason=reason,subject=subject,chapter=chapter,content_hash=h,text=text,original=str(target))
