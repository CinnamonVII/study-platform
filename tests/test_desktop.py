import os
os.environ['QT_QPA_PLATFORM']='offscreen'
from pathlib import Path
import json
from engine.models import Snapshot
from apps.desktop.main import training

def test_desktop_and_web_share_identical_questions():
    path=Path('fixtures/demo/snapshot-v2.json');state=Snapshot.model_validate_json(path.read_text())
    ids=[q.id for q in training(state)]
    assert set(ids)==set(state.questions)
    assert '../../../fixtures/demo/snapshot-v2.json' in Path('apps/web/src/data.ts').read_text()

def test_native_views_and_practice():
    from PySide6.QtWidgets import QApplication
    from apps.desktop.main import Window
    app=QApplication.instance() or QApplication([]);w=Window()
    for i in range(5):w.nav.setCurrentRow(i);app.processEvents()
    w.nav.setCurrentRow(3);w.response.setPlainText('Une raison constante');w.reveal()
    assert w.revealed and w.correction.toPlainText() and w.pages.count()==5
    w.close()
