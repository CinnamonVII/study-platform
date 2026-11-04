import Ajv2020 from 'ajv/dist/2020.js';
import schema from '../../../schemas/snapshot-v2.json';
import demo from '../../../fixtures/demo/snapshot-v2.json';
import type {Snapshot,Question,Review} from './schema';
export type {Snapshot,Question,Review,Fiche,Document,Block} from './schema';
const validate=new Ajv2020({strict:false}).compile(schema);
export function parseSnapshot(value:unknown):Snapshot {if(!validate(value))throw new Error('Les données ne respectent pas le schéma 2.0.');return value as unknown as Snapshot;}
export const demoSnapshot=parseSnapshot(demo);
export function training(s:Snapshot,subject='Toutes'):Question[]{return Object.values(s.questions).filter(q=>q.status==='validated'&&!q.stale&&(subject==='Toutes'||q.subject===subject)&&q.sources.every(r=>{const d=s.documents[r.document_id];return d&&!d.deleted&&d.status==='validated'&&!d.uncertainties?.some(u=>!u.resolved);})).sort((a,b)=>(s.reviews[a.id]?.due||'').localeCompare(s.reviews[b.id]?.due||''));}
export function schedule(review:Review,grade:number,at=new Date()):Review {if(grade<0||grade>5)throw Error('Note invalide');const r={...review};if(grade<3){r.repetitions=0;r.interval=1;}else{r.interval=r.repetitions===0?1:r.repetitions===1?6:Math.round((r.interval||0)*(r.ease||2.5));r.repetitions=(r.repetitions||0)+1;}r.ease=Math.max(1.3,(r.ease||2.5)+.1-(5-grade)*(.08+(5-grade)*.02));r.last_grade=grade;r.due=new Date(at.getTime()+(r.interval||0)*86400000).toISOString();return r;}
