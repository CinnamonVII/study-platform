from __future__ import annotations
from pathlib import Path
from html import escape
import re
from engine.models import Fiche

COLORS={'red':'#BD4D50','blue':'#304B72','green':'#28786B','purple':'#78608D','yellow':'#FFF2BF'}

def markdown(f,state=None):
    lines=['# '+f.title,'',f.subject+' · '+f.chapter,'']
    for b in f.blocks:
        lines+=['## '+b.title,'',b.text,'']
        if b.kind=='mechanism': lines+=[' → '.join(b.items),'']
        elif b.kind in ('steps','timeline'): lines += [f'{i+1}. {v}' for i,v in enumerate(b.items)]+['']
        else: lines += ['- '+v for v in b.items]+['']
        if b.columns:
            lines+=['| '+' | '.join(b.columns)+' |','| '+' | '.join('---' for _ in b.columns)+' |']
            lines+=['| '+' | '.join(row)+' |' for row in b.rows]+['']
    lines+=['## À retenir','']+['- '+v for v in f.takeaways]+['','## Rappel actif','']+['- '+v for v in f.recall]+['','## Sources','']
    for r in f.sources:
        d=state.documents.get(r.document_id) if state else None
        url=d.origin_url if d else ''
        title=d.title if d else r.document_id
        lines.append(('- ['+title+']('+url+')') if url else '- '+title)
        if r.page: lines[-1]+=' · p. '+str(r.page)
    return '\n'.join(lines)+'\n'

def html(f,state=None):
    s='<article class="fiche"><h1>'+escape(f.title)+'</h1><p>'+escape(f.subject+' · '+f.chapter)+'</p>'
    for b in f.blocks:
        color=COLORS['green' if b.kind=='definition' else 'red' if b.kind=='warning' else 'purple' if b.kind in ('mechanism','timeline') else 'blue']
        s+='<section><h2 style="color:'+COLORS['red']+'">'+escape(b.title)+'</h2><p style="color:'+color+'">'+escape(b.text).replace('\n','<br>')+'</p>'
        if b.kind=='mechanism': s+='<p style="color:'+COLORS['purple']+'">'+' → '.join(escape(v) for v in b.items)+'</p>'
        else: s+='<ol>' + ''.join('<li>'+escape(v)+'</li>' for v in b.items)+'</ol>'
        if b.columns: s+='<table border="1"><tr>'+''.join('<th>'+escape(c)+'</th>' for c in b.columns)+'</tr>'+''.join('<tr>'+''.join('<td>'+escape(v)+'</td>' for v in row)+'</tr>' for row in b.rows)+'</table>'
        s+='</section>'
    s+='<h2>À retenir</h2><ul>'+''.join('<li>'+escape(v)+'</li>' for v in f.takeaways)+'</ul><h2>Rappel actif</h2><ul>'+''.join('<li>'+escape(v)+'</li>' for v in f.recall)+'</ul>'
    s+='<h3>Sources</h3>'
    for ref in f.sources:
        d=state.documents.get(ref.document_id) if state else None
        s+='<p>'+escape(d.title if d else ref.document_id)+(' · p. '+str(ref.page) if ref.page else '')+'</p>'
    if f.notion_url.startswith('https://'):s+='<p><a href="'+escape(f.notion_url,quote=True)+'">Ouvrir dans Notion</a></p>'
    return s+'</article>'

def latex_escape(value):
    return ''.join({'\\':r'\textbackslash{}','&':r'\&','%':r'\%','$':r'\$','#':r'\#','_':r'\_','{':r'\{','}':r'\}'}.get(c,c) for c in value)

def latex(f):
    t=[r'\documentclass[11pt,a4paper]{article}',r'\usepackage{fontspec,amsmath,amssymb,xcolor,geometry,enumitem}',
       r'\geometry{margin=17mm}',r'\setmainfont{DejaVu Sans}',r'\definecolor{ink}{HTML}{304B72}',r'\definecolor{heading}{HTML}{BD4D50}',
       r'\pagecolor[HTML]{FFFDF6}\color{ink}',r'\begin{document}',r'{\color{heading}\LARGE '+latex_escape(f.title)+r'}\par']
    for b in f.blocks:
        t+=[r'\section*{'+latex_escape(b.title)+'}',latex_escape(b.text)+r'\par']
        if b.items: t+=[r'\begin{enumerate}']+[r'\item '+latex_escape(v) for v in b.items]+[r'\end{enumerate}']
        if b.columns:
            t+=[r'\begin{tabular}{'+'p{'+str(round(16/len(b.columns)-.3,2))+'cm}'*len(b.columns)+'}', ' & '.join(latex_escape(v) for v in b.columns)+r'\\\hline']
            t+=[' & '.join(latex_escape(v) for v in row)+r'\\' for row in b.rows]+[r'\end{tabular}']
    for title,items in [('À retenir',f.takeaways),('Rappel actif',f.recall),('Sources',[r.document_id+(' p. '+str(r.page) if r.page else '') for r in f.sources])]:
        t+=[r'\section*{'+title+'}',r'\begin{itemize}']+[r'\item '+latex_escape(v) for v in items]+[r'\end{itemize}']
    return '\n'.join(t+[r'\end{document}'])+'\n'

def pdf(f,state,path):
    from reportlab.platypus import SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, KeepTogether
    from reportlab.lib.styles import ParagraphStyle
    from reportlab.lib.colors import HexColor
    from reportlab.lib.pagesizes import A4
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    # Embed full Unicode fonts; no dependence on the reader's system fonts.
    fonts=Path(__file__).parent/'fonts'
    fallback=Path('/usr/share/fonts/truetype/dejavu')
    for face,file in [('Study','DejaVuSans.ttf'),('StudyBold','DejaVuSans-Bold.ttf')]:
        source=(fonts/file) if (fonts/file).exists() else fallback/file
        pdfmetrics.registerFont(TTFont(face,str(source)))
    pdfmetrics.registerFontFamily('Study',normal='Study',bold='StudyBold')
    def style(name,color,size=11): return ParagraphStyle(name,fontName='Study',fontSize=size,leading=size*1.4,textColor=HexColor(color),spaceAfter=6)
    body=style('body',COLORS['blue']); small=style('small',COLORS['blue'],8)
    title=style('title',COLORS['red'],24); heading=style('heading',COLORS['red'],15)
    def p(t,sty=body): return Paragraph(escape(t).replace('\n','<br/>'),sty)
    story=[p(f.title,title),p(f.subject+' · '+f.chapter,small),Spacer(1,12)]
    width=A4[0]-96
    for b in f.blocks:
        group=[p(b.title,heading)]
        color=COLORS['green' if b.kind=='definition' else 'red' if b.kind=='warning' else 'blue']
        if b.text: group.append(p(b.text,style('block',color)))
        if b.kind in ('mechanism','timeline') and b.items:
            # A vertical vector flow keeps labels readable regardless of count.
            for i,item in enumerate(b.items):
                box=Table([[p(item,style('flow',COLORS['purple']))]],colWidths=[width-12])
                box.setStyle(TableStyle([('BOX',(0,0),(-1,-1),.6,HexColor('#D5C6DE')),('BACKGROUND',(0,0),(-1,-1),HexColor('#F5EFF8')),('LEFTPADDING',(0,0),(-1,-1),10),('TOPPADDING',(0,0),(-1,-1),6)]))
                group.append(box)
                if i<len(b.items)-1: group.append(p('↓',style('arrow',COLORS['purple'],13)))
        else:
            group += [p((str(i+1)+'. ' if b.kind=='steps' else '• ')+v) for i,v in enumerate(b.items)]
        if b.columns:
            data=[[p(v,style('th',COLORS['green'],10)) for v in b.columns]]+[[p(v,small) for v in row] for row in b.rows]
            table=Table(data,colWidths=[width/len(b.columns)]*len(b.columns),repeatRows=1,hAlign='LEFT')
            table.setStyle(TableStyle([('GRID',(0,0),(-1,-1),.4,HexColor('#D5DCD5')),('BACKGROUND',(0,0),(-1,0),HexColor('#EAF4EC')),('VALIGN',(0,0),(-1,-1),'TOP'),('LEFTPADDING',(0,0),(-1,-1),7),('TOPPADDING',(0,0),(-1,-1),7),('BOTTOMPADDING',(0,0),(-1,-1),7)]))
            group.append(table)
        group.append(Spacer(1,12))
        # Allow large blocks to split; keep only heading and first paragraph together.
        story.append(KeepTogether(group[:2])); story.extend(group[2:])
    for name,items in [('À retenir',f.takeaways),('Rappel actif',f.recall)]:
        story.append(p(name,heading))
        for item in items:
            box=Table([[p('• '+item)]],colWidths=[width])
            box.setStyle(TableStyle([('BACKGROUND',(0,0),(-1,-1),HexColor('#FFF2BF' if name=='À retenir' else '#F4F1FA')),('LEFTPADDING',(0,0),(-1,-1),9),('TOPPADDING',(0,0),(-1,-1),6)]))
            story.extend([box,Spacer(1,5)])
    story.extend([Spacer(1,12),p('Sources',heading)])
    for ref in f.sources:
        d=state.documents.get(ref.document_id)
        story.append(p((d.title if d else ref.document_id)+(f' · page {ref.page}' if ref.page else '')+(' · '+d.origin_url if d and d.origin_url else ''),small))
    def background(canvas,doc):
        w,h=A4
        canvas.saveState(); canvas.setFillColor(HexColor('#FFFDF6')); canvas.rect(0,0,w,h,fill=1,stroke=0)
        canvas.setStrokeColor(HexColor('#E6E9E6')); canvas.setLineWidth(.2)
        for x in range(32,int(w)-30,14): canvas.line(x,32,x,h-32)
        for y in range(32,int(h)-30,14): canvas.line(32,y,w-32,y)
        canvas.setStrokeColor(HexColor('#EED17B')); canvas.setLineWidth(1); canvas.rect(30,30,w-60,h-60)
        canvas.setFont('Study',8); canvas.setFillColor(HexColor(COLORS['blue'])); canvas.drawRightString(w-48,40,str(doc.page)); canvas.restoreState()
    SimpleDocTemplate(str(path),pagesize=A4,leftMargin=48,rightMargin=48,topMargin=48,bottomMargin=56,title=f.title,author='Study Platform').build(story,onFirstPage=background,onLaterPages=background)

def export_fiche(f,state,root):
    folder=Path(root)/f.id; folder.mkdir(parents=True,exist_ok=True,mode=0o700)
    (folder/'fiche.md').write_text(markdown(f,state))
    (folder/'fiche.tex').write_text(latex(f))
    pdf(f,state,folder/'fiche.pdf')
    return folder
