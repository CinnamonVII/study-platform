"""Public fictitious fixtures. Deterministic generator is for demos and tests only."""
from engine.models import Document, Fiche, Block, Reference, Question, Generation, Transcription, digest

SAMPLES=[
 ('Mathématiques','Suites arithmétiques','cours','Suite arithmétique : on ajoute une raison constante r à chaque terme.','u₀ = 3 ; r = 2.','u₁ = 3 + 2 = 5','u₂ = 5 + 2 = 7','uₙ = u₀ + n × r'),
 ('SES','Marché concurrentiel','devoir',"Prix d’équilibre : prix pour lequel la quantité offerte égale la quantité demandée.",'Hausse de la demande','Déplacement de la demande vers la droite','Nouveau prix d’équilibre plus élevé','Quantité échangée plus élevée'),
 ('HGGSP','Comprendre la démocratie','cours','Démocratie représentative : les citoyens élisent des représentants qui prennent les décisions.','Citoyens','Élections','Représentants','Décisions politiques'),
 ('Français','Analyse de texte','methode','Analyse littéraire : relier une observation précise à une interprétation.','Formuler une idée','Citer brièvement le texte','Identifier le procédé','Expliquer son effet')]

def documents():
    return [Document(id='demo-'+str(i),title=chapter+' · document fictif',kind=kind,level=1,provenance_reason='Fixture explicitement classée : notes personnelles fictives',
        subject=subject,chapter=chapter,content_hash=digest(text),text='\n'.join((text,*items)),private=False)
        for i,(subject,chapter,kind,text,*items) in enumerate(SAMPLES)]

class DemoGenerator:
    def transcribe(self,doc):
        # Never pretends to OCR arbitrary documents.
        raise ValueError('Demo generator cannot transcribe manuscripts; configure a model')
    def generate(self,docs,existing=None):
        d=docs[0]; ref=Reference(document_id=d.id); lines=d.text.splitlines()
        f=Fiche(id='temporary',title=d.chapter,subject=d.subject,chapter=d.chapter,
            blocks=[Block(kind='definition',title='La notion',text=lines[0],sources=[ref]),
                Block(kind='steps' if d.subject in ('Mathématiques','Français') else 'mechanism',title='Comprendre le raisonnement',items=lines[1:],sources=[ref]),
                Block(kind='warning',title='Le piège',text='Relie chaque étape à la précédente. Vérifie les conditions avant de conclure.',sources=[ref])],
            takeaways=[lines[0],lines[-1]],recall=['Peux-tu définir la notion sans regarder ?','Peux-tu expliquer chaque étape ?'],sources=[ref])
        qs=[Question(id='temporary',concept_key='definition',subject=d.subject,chapter=d.chapter,type='short',cognitive='memory',
            prompt='Définis la notion centrale : '+d.chapter+'.',answer=lines[0],explanation='Conserve les conditions indiquées dans le document.',difficulty=1,sources=[ref]),
            Question(id='temporary2',concept_key='mechanism',subject=d.subject,chapter=d.chapter,type='math' if d.subject=='Mathématiques' else 'method',cognitive='application',
                prompt='Retrouve et explique les étapes du document : '+d.chapter+'.',answer='\n'.join(lines[1:]),explanation='Le raisonnement doit conserver les étapes intermédiaires.',
                steps=lines[1:],rubric=['Notion définie','Étapes justifiées','Conclusion reliée à la consigne'],difficulty=3,sources=[ref]),
            Question(id='temporary3',concept_key='recognition',subject=d.subject,chapter=d.chapter,type='mcq',cognitive='understanding',
                prompt='Quel énoncé est fidèle au document ?',answer=lines[0],explanation='Le premier choix reprend la définition source.',
                choices=[lines[0],'Une conclusion est vraie sans vérifier les conditions.','Les étapes intermédiaires sont toujours inutiles.'],correct_indices=[0],
                traps=['Le deuxième choix oublie les conditions.','Le troisième choix supprime le raisonnement.'],difficulty=2,sources=[ref])]
        return Generation(fiche=f,questions=qs)
