from __future__ import annotations
import argparse
import json
import sys
from pathlib import Path
from urllib.parse import urlparse
from datetime import datetime,timezone
import httpx
from PySide6.QtCore import Qt,QObject,Signal,QRunnable,QThreadPool,QUrl
from PySide6.QtGui import QDesktopServices
from PySide6.QtWidgets import (QApplication,QMainWindow,QWidget,QHBoxLayout,QVBoxLayout,QLabel,QListWidget,
    QPushButton,QComboBox,QLineEdit,QTextBrowser,QTextEdit,QStackedWidget,QTabWidget,QMessageBox,QDialog,QDialogButtonBox,
    QFileDialog,QCheckBox,QScrollArea,QSplitter)
from engine.models import Snapshot,Review,Status
from engine.sync.store import Store
from engine.repetition import schedule
from engine.validation.rules import publishable
from engine.export.render import html

class Signals(QObject):
    done=Signal(object); error=Signal(str)
class Task(QRunnable):
    def __init__(self,fn): super().__init__(); self.fn=fn; self.signals=Signals()
    def run(self):
        try: self.signals.done.emit(self.fn())
        except Exception as e: self.signals.error.emit(str(e))

def training(state,subject='Toutes'):
    return sorted([q for q in state.questions.values() if publishable(q,state) and (subject=='Toutes' or q.subject==subject)],key=lambda q:state.reviews.get(q.id,Review(question_id=q.id,due='')).due)

def bundle_root(): return Path(getattr(sys,'_MEIPASS',Path(__file__).resolve().parents[2]))

class Window(QMainWindow):
    def __init__(self,state=None):
        super().__init__(); self.setWindowTitle('Révisions · première'); self.resize(1180,820)
        self.pool=QThreadPool(); self.api=''; self.token=''; self.revision=''; self.connected=False
        self.demo_store=Store(Path.home()/'.local/share/study-platform/demo-desktop')
        self.state=state or Snapshot.model_validate_json((bundle_root()/'fixtures/demo/snapshot-v2.json').read_text())
        if self.demo_store.path.exists(): self.state.reviews=self.demo_store.load().reviews
        self.current_q=None; self.current_f=None; self.current_d=None; self.index=0; self.revealed=False
        self.setStyleSheet('QPushButton {padding:8px 12px} QLineEdit,QComboBox {padding:6px} QListWidget::item {padding:10px} QTextBrowser {font-size:14px}');root=QWidget(); self.setCentralWidget(root); layout=QHBoxLayout(root)
        self.nav=QListWidget(); self.nav.addItems(['Tableau de bord','Bibliothèque','Fiches','Entraînement','Synchronisation']); self.nav.setFixedWidth(205)
        layout.addWidget(self.nav); main=QVBoxLayout();layout.addLayout(main,1)
        title=QLabel('R.  révisions'); title.setStyleSheet('font-size:26px;font-weight:bold;color:#304B72;padding:12px')
        main.addWidget(title); filters=QHBoxLayout();self.subject=QComboBox();self.subject.addItem('Toutes')
        filters.addWidget(self.subject);self.search=QLineEdit();self.search.setPlaceholderText('Rechercher une notion, un chapitre…');filters.addWidget(self.search)
        main.addLayout(filters);self.pages=QStackedWidget(); main.addWidget(self.pages,1)
        self.notice=QLabel('Démonstration · documents fictifs · tâche de 7 h à configurer');self.notice.setTextFormat(Qt.PlainText);self.notice.setWordWrap(True);main.addWidget(self.notice)
        self.build_dashboard();self.build_library();self.build_fiches();self.build_practice();self.build_sync()
        self.nav.currentRowChanged.connect(self.change_view);self.subject.currentTextChanged.connect(self.refresh);self.search.textChanged.connect(self.refresh)
        self.nav.setCurrentRow(0);self.refresh()
    def page(self):
        w=QWidget();l=QVBoxLayout(w);self.pages.addWidget(w);return l
    def button(self,text,fn,layout):
        b=QPushButton(text);b.clicked.connect(fn);layout.addWidget(b);return b
    def task(self,fn,done=None):
        self.notice.setText('Traitement en cours…');t=Task(fn)
        t.signals.done.connect(done or (lambda _:self.reload()))
        t.signals.error.connect(lambda msg:self.notice.setText('Erreur : '+msg))
        self.pool.start(t)
    def request(self,path,method='GET',body=None):
        if not self.connected and path!='/v1/snapshot': raise ValueError('Connecte ton serveur privé')
        p=urlparse(self.api)
        if p.scheme!='https' and p.hostname not in ('127.0.0.1','localhost'): raise ValueError('HTTPS requis')
        r=httpx.request(method,self.api.rstrip('/')+path,headers={'Authorization':'Bearer '+self.token},json=body,timeout=300)
        if not r.is_success: raise ValueError(f'API {r.status_code}: {r.text}')
        return r
    def reload(self):
        def done(data):
            self.state=Snapshot.model_validate(data['snapshot']);self.revision=data['revision'];self.connected=True
            self.notice.setText('Espace privé chargé.');self.refresh()
        self.task(lambda:self.request('/v1/snapshot').json(),done)
    def change_view(self,i): self.pages.setCurrentIndex(i);self.refresh()
    def build_dashboard(self):
        l=self.page();self.dash=QTextBrowser();self.dash.setOpenExternalLinks(False);l.addWidget(self.dash)
        self.button('Commencer une révision',lambda:self.nav.setCurrentRow(3),l)
    def build_library(self):
        l=self.page();f=QHBoxLayout();self.kind=QComboBox();self.kind.addItems(['Tous','cours','devoir','exercice','correction','fiche','methode','annale'])
        self.level=QComboBox();self.level.addItems(['Tous','1','2','non attribué']);f.addWidget(self.kind);f.addWidget(self.level);l.addLayout(f)
        self.kind.currentTextChanged.connect(self.refresh);self.level.currentTextChanged.connect(self.refresh)
        splitter=QSplitter();self.docs=QListWidget();self.doc_text=QTextBrowser();self.doc_text.setOpenExternalLinks(True)
        splitter.addWidget(self.docs);splitter.addWidget(self.doc_text);splitter.setSizes([300,650]);l.addWidget(splitter)
        self.docs.currentRowChanged.connect(self.show_document)
        row=QHBoxLayout();l.addLayout(row)
        self.button('Corriger / classifier / valider',lambda:self.edit('documents',self.current_d),row)
        self.button('Ouvrir la source',self.open_source,row);self.button('Télécharger l’original',self.download_original,row)
    def build_fiches(self):
        l=self.page();splitter=QSplitter();self.fiches=QListWidget();self.fiche_view=QTextBrowser();self.fiche_view.setOpenExternalLinks(True)
        splitter.addWidget(self.fiches);splitter.addWidget(self.fiche_view);splitter.setSizes([280,680]);l.addWidget(splitter)
        self.fiches.currentRowChanged.connect(self.show_fiche)
        row=QHBoxLayout();l.addLayout(row)
        self.button('Corriger / accepter / refuser',lambda:self.edit('fiches',self.current_f),row)
        self.button('Exporter PDF',lambda:self.export('pdf'),row);self.button('Exporter Markdown',lambda:self.export('md'),row);self.button('Exporter LaTeX',lambda:self.export('tex'),row)
    def build_practice(self):
        l=self.page();self.q_title=QLabel();self.q_title.setTextFormat(Qt.PlainText);self.q_title.setWordWrap(True);self.q_title.setStyleSheet('font-size:22px;padding:15px');l.addWidget(self.q_title)
        self.choices=QWidget();self.choice_layout=QVBoxLayout(self.choices);l.addWidget(self.choices)
        self.response=QTextEdit();self.response.setPlaceholderText('Écris ton raisonnement avant de regarder la correction…');l.addWidget(self.response)
        self.check=self.button('Valider ma réponse',self.reveal,l)
        self.correction=QTextBrowser();l.addWidget(self.correction)
        self.grades=QWidget();r=QHBoxLayout(self.grades)
        for n,label in [(1,'À reprendre'),(3,'Difficile'),(4,'Compris'),(5,'Facile')]: self.button(label,lambda checked=False,n=n:self.grade(n),r)
        l.addWidget(self.grades)
        r=QHBoxLayout();l.addLayout(r)
        self.button('Voir la fiche',self.question_fiche,r);self.button('Voir le document source',self.question_source,r)
        self.button('Signaler une erreur',self.report_question,r);self.button('Corriger la question',lambda:self.edit('questions',self.current_q.id if self.current_q else None),r)
    def build_sync(self):
        l=self.page();l.addWidget(QLabel('Connecter mon espace privé'))
        self.api_input=QLineEdit();self.api_input.setPlaceholderText('https://revision.example.fr');l.addWidget(self.api_input)
        self.token_input=QLineEdit();self.token_input.setEchoMode(QLineEdit.Password);self.token_input.setPlaceholderText('Jeton personnel du serveur (jamais une clé Notion)');l.addWidget(self.token_input)
        r=QHBoxLayout();l.addLayout(r)
        self.button('Charger mes données',self.connect_api,r);self.button('Déconnexion',self.disconnect,r)
        self.button('Prévisualiser les changements',lambda:self.sync(True),r);self.button('Synchroniser / relancer',lambda:self.sync(False),r)
        self.sync_log=QTextBrowser();l.addWidget(self.sync_log)
        self.proposals=QListWidget();l.addWidget(self.proposals)
        r=QHBoxLayout();l.addLayout(r)
        self.button('Comparer la proposition',self.preview_proposal,r)
        self.button('Accepter la proposition',lambda:self.proposal('accept'),r);self.button('Refuser la proposition',lambda:self.proposal('reject'),r)
    def connect_api(self):
        self.api=self.api_input.text().strip();self.token=self.token_input.text();self.reload()
    def disconnect(self):
        self.connected=False;self.token='';self.token_input.clear();self.revision=''
        self.state=Snapshot.model_validate_json((bundle_root()/'fixtures/demo/snapshot-v2.json').read_text());self.refresh();self.notice.setText('Déconnecté · démonstration')
    def matches(self,obj): return (self.subject.currentText() in ('Toutes',obj.subject)) and self.search.text().casefold() in (obj.subject+' '+obj.chapter+' '+getattr(obj,'title',getattr(obj,'prompt',''))).casefold()
    def refresh(self,*args):
        selected=self.subject.currentText();subjects=['Toutes']+sorted({d.subject for d in self.state.documents.values()})
        if [self.subject.itemText(i) for i in range(self.subject.count())]!=subjects:
            self.subject.blockSignals(True);self.subject.clear();self.subject.addItems(subjects);self.subject.setCurrentText(selected if selected in subjects else 'Toutes');self.subject.blockSignals(False)
        qs=training(self.state,self.subject.currentText());due=sum(1 for q in qs if q.id not in self.state.reviews or self.state.reviews[q.id].due<=datetime.now(timezone.utc).isoformat())
        from html import escape
        self.dash.setHtml('<h1>Ton programme, une notion à la fois.</h1><h2>'+str(due)+' questions à revoir · '+str(len(self.state.fiches))+' fiches</h2><p>Mathématiques, SES, HGGSP et français. Comprendre, s’entraîner, retenir.</p><h2>Les dernières fiches</h2>'+''.join('<p>'+escape(f.subject)+' · '+escape(f.title)+' · '+f.status+'</p>' for f in list(self.state.fiches.values())[-5:])+'<p>Dernier succès : '+str(self.state.last_success or 'aucun')+'</p>')
        self.docs.blockSignals(True);self.docs.clear();self.document_ids=[]
        for d in self.state.documents.values():
            if self.matches(d) and self.kind.currentText() in ('Tous',d.kind) and self.level.currentText() in ('Tous',str(d.level) if d.level else 'non attribué'):
                self.docs.addItem(d.title+'\n'+d.kind+' · niveau '+str(d.level or '?')+' · '+d.status);self.document_ids.append(d.id)
        self.docs.blockSignals(False)
        self.fiches.blockSignals(True);self.fiches.clear();self.fiche_ids=[]
        for f in self.state.fiches.values():
            if self.matches(f): self.fiches.addItem(f.subject+'\n'+f.title+' · '+('à actualiser' if f.stale else f.status));self.fiche_ids.append(f.id)
        self.fiches.blockSignals(False)
        if self.current_d in self.document_ids: self.docs.setCurrentRow(self.document_ids.index(self.current_d));self.show_document(self.document_ids.index(self.current_d))
        elif self.document_ids: self.docs.setCurrentRow(0)
        else: self.current_d=None;self.doc_text.clear()
        if self.current_f in self.fiche_ids: self.fiches.setCurrentRow(self.fiche_ids.index(self.current_f));self.show_fiche(self.fiche_ids.index(self.current_f))
        elif self.fiche_ids:self.fiches.setCurrentRow(0)
        else:self.current_f=None;self.fiche_view.clear()
        self.show_question()
        self.sync_log.setPlainText('Dernier succès : '+str(self.state.last_success)+'\nTâche ChatGPT de 7 h : vérifier dans ChatGPT.\n\n'+json.dumps(self.state.reports[-7:],ensure_ascii=False,indent=2))
        self.proposal_keys=list(self.state.proposals);self.proposals.clear();self.proposals.addItems(self.proposal_keys)
    def show_document(self,i):
        if not 0<=i<len(self.document_ids): return
        from html import escape
        d=self.state.documents[self.document_ids[i]];self.current_d=d.id
        self.doc_text.setHtml('<h2>'+escape(d.title)+'</h2><p>'+escape(d.provenance_reason)+'</p><pre>'+escape(d.text)+'</pre>'+''.join('<p style="color:#BD4D50">'+escape(u.message)+'</p>' for u in d.uncertainties if not u.resolved))
    def show_fiche(self,i):
        if not 0<=i<len(self.fiche_ids):return
        self.current_f=self.fiche_ids[i];f=self.state.fiches[self.current_f]
        self.fiche_view.setStyleSheet('QTextBrowser {background:#FFFDF6; color:#304B72;border:1px solid #EED17B;padding:14px}')
        self.fiche_view.setHtml(html(f,self.state))
    def show_question(self):
        qs=training(self.state,self.subject.currentText());self.correction.clear();self.grades.hide();self.revealed=False;self.response.clear();self.response.setReadOnly(False);self.check.setEnabled(bool(qs))
        while self.choice_layout.count():
            w=self.choice_layout.takeAt(0).widget()
            if w:w.deleteLater()
        self.choice_boxes=[]
        if not qs:self.current_q=None;self.q_title.setText('Aucune question validée. Valide les sources et les questions avant l’entraînement.');return
        q=qs[self.index%len(qs)];self.current_q=q;self.q_title.setText(q.subject+' · '+q.type+' · difficulté '+str(q.difficulty)+'/5\n\n'+q.prompt)
        self.response.setVisible(q.type!='mcq');self.choices.setVisible(q.type=='mcq')
        for text in q.choices:
            c=QCheckBox(text);self.choice_boxes.append(c);self.choice_layout.addWidget(c)
    def reveal(self):
        if not self.current_q:return
        q=self.current_q
        if q.type!='mcq' and not self.response.toPlainText().strip():self.notice.setText('Essaie une réponse avant de lire la correction.');return
        if q.type=='mcq' and not any(c.isChecked() for c in self.choice_boxes):self.notice.setText('Choisis au moins une réponse.');return
        self.revealed=True;self.check.setEnabled(False);self.response.setReadOnly(True)
        for c in self.choice_boxes:c.setEnabled(False)
        result=''
        if q.type=='mcq':result='Bonne réponse !\n' if [i for i,c in enumerate(self.choice_boxes) if c.isChecked()]==q.correct_indices else 'Compare tes choix à la correction.\n'
        self.correction.setPlainText(result+q.answer+'\n\n'+'\n'.join(q.steps)+'\n\n'+q.explanation+'\n'+'\n'.join(q.traps)+'\n\nGrille : '+' · '.join(q.rubric));self.grades.show()
    def grade(self,n):
        if not self.current_q or not self.revealed:return
        qid=self.current_q.id
        if self.connected:
            self.task(lambda:self.request('/v1/questions/'+qid+'/review','POST',{'grade':n}),lambda _:self.next_question());return
        self.state.reviews[qid]=schedule(self.state.reviews.get(qid,Review(question_id=qid)),n)
        self.demo_store.mutate(lambda s:setattr(s,'reviews',self.state.reviews));self.next_question()
    def next_question(self):
        self.index+=1
        if self.connected:self.reload()
        else:self.show_question()
    def question_fiche(self):
        if self.current_q and self.current_q.fiche_ids:self.current_f=self.current_q.fiche_ids[0];self.nav.setCurrentRow(2)
    def question_source(self):
        if self.current_q:self.current_d=self.current_q.sources[0].document_id;self.nav.setCurrentRow(1)
    def open_source(self):
        if self.current_d:
            url=self.state.documents[self.current_d].origin_url
            if url:QDesktopServices.openUrl(QUrl(url))
    def edit(self,collection,id):
        if not self.connected:self.notice.setText('Connecte le serveur privé pour corriger et valider les contenus.');return
        if not id:return
        obj=getattr(self.state,collection)[id]
        keys={'documents':['text','level','provenance_reason','kind','subject','chapter','status','uncertainties'],
              'fiches':['title','blocks','takeaways','recall','status'],
              'questions':['prompt','answer','explanation','choices','correct_indices','traps','steps','rubric','difficulty','status']}[collection]
        d=QDialog(self);d.setWindowTitle('Corriger / classifier / valider');d.resize(800,650);l=QVBoxLayout(d)
        l.addWidget(QLabel('Statut : pending, validated ou rejected. Valide les sources en premier.'))
        editor=QTextEdit();editor.setPlainText(json.dumps({k:getattr(obj,k) for k in keys},ensure_ascii=False,indent=2,default=lambda o:o.model_dump(mode='json')));l.addWidget(editor)
        buttons=QDialogButtonBox(QDialogButtonBox.Save|QDialogButtonBox.Cancel);l.addWidget(buttons);buttons.accepted.connect(d.accept);buttons.rejected.connect(d.reject)
        if d.exec()==QDialog.Accepted:
            try:changes=json.loads(editor.toPlainText())
            except ValueError as e:self.notice.setText(str(e));return
            self.task(lambda:self.request('/v1/'+collection+'/'+id,'PATCH',{'revision':self.revision,'changes':changes}))
    def export(self,fmt):
        if not self.current_f:return
        path,_=QFileDialog.getSaveFileName(self,'Exporter la fiche',self.current_f+'.'+fmt)
        if not path:return
        if self.connected:self.task(lambda:Path(path).write_bytes(self.request('/v1/fiches/'+self.current_f+'/export/'+fmt).content),lambda _:self.notice.setText('Export enregistré.'))
        else:
            from engine.export.render import export_fiche
            import shutil
            f=export_fiche(self.state.fiches[self.current_f],self.state,self.demo_store.root/'exports')
            shutil.copyfile(f/('fiche.'+fmt),path);self.notice.setText('Export fictif enregistré.')
    def download_original(self):
        if not self.connected or not self.current_d:return
        path,_=QFileDialog.getSaveFileName(self,'Enregistrer l’original',self.state.documents[self.current_d].title)
        if path:self.task(lambda:Path(path).write_bytes(self.request('/v1/documents/'+self.current_d+'/original').content),lambda _:self.notice.setText('Original enregistré.'))
    def sync(self,preview):
        if not self.connected:self.notice.setText('Connecte ton serveur privé.');return
        self.task(lambda:self.request('/v1/sync'+('?preview=true' if preview else ''),'POST').json(),
            (lambda data:self.notice.setText(json.dumps(data,ensure_ascii=False))) if preview else None)
    def report_question(self):
        if not self.connected or not self.current_q:return
        from PySide6.QtWidgets import QInputDialog
        msg,ok=QInputDialog.getMultiLineText(self,'Signaler une erreur','Quel problème as-tu repéré ?')
        if ok and msg:self.task(lambda:self.request('/v1/questions/'+self.current_q.id+'/report','POST',{'message':msg}))
    def preview_proposal(self):
        i=self.proposals.currentRow()
        if i<0:return
        d=QDialog(self);d.resize(800,600);l=QVBoxLayout(d);t=QTextBrowser();t.setPlainText(json.dumps(self.state.proposals[self.proposal_keys[i]],ensure_ascii=False,indent=2));l.addWidget(t);d.exec()
    def proposal(self,action):
        i=self.proposals.currentRow()
        if not self.connected or i<0:return
        key=self.proposal_keys[i]
        self.task(lambda:self.request('/v1/proposals/'+key+'/'+action,'POST',{'revision':self.revision,'changes':{}}))

def main():
    app=QApplication(sys.argv);app.setApplicationName('Révisions');app.setOrganizationName('StudyPlatform')
    w=Window();w.show();sys.exit(app.exec())
if __name__=='__main__':main()
