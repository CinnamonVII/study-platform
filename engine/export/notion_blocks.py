"""Native colored Notion blocks, limited to one atomic page-create request."""
def rich(text,color='default'):
    return [{'type':'text','text':{'content':text[i:i+1800]},'annotations':{'color':color}} for i in range(0,len(text),1800)]
def block(kind,text,color='default'):
    return {'object':'block','type':kind,kind:{'rich_text':rich(text,color)}}
def blocks_for(f,state):
    out=[block('heading_1',f.title,'red'),block('paragraph',f.subject+' · '+f.chapter+' · source de niveau 2','blue')]
    for b in f.blocks:
        out.append(block('heading_2',b.title,'red'))
        color='green' if b.kind=='definition' else 'red' if b.kind=='warning' else 'blue'
        if b.text:out.append(block('paragraph',b.text,color))
        if b.kind in ('mechanism','timeline'):
            out.append(block('paragraph',' → '.join(b.items),'purple'))
        else:
            for v in b.items:out.append(block('numbered_list_item' if b.kind=='steps' else 'bulleted_list_item',v,color))
        if b.columns:
            out.append(block('paragraph',' | '.join(b.columns),'green'))
            for row in b.rows:out.append(block('paragraph',' | '.join(row),'blue'))
    for title,items in [('À retenir',f.takeaways),('Rappel actif',f.recall)]:
        out.append(block('heading_2',title,'red'))
        for item in items:out.append(block('bulleted_list_item',item,'yellow_background' if title=='À retenir' else 'blue'))
    out.append(block('heading_2','Sources','red'))
    for r in f.sources:
        d=state.documents[r.document_id]
        content=rich(d.title+(' · p. '+str(r.page) if r.page else ''),'blue')
        if d.origin_url:
            for part in content:part['text']['link']={'url':d.origin_url}
        out.append({'object':'block','type':'paragraph','paragraph':{'rich_text':content}})
    if len(out)>100:raise ValueError('Split this fiche: too many Notion blocks for atomic page creation')
    return out
