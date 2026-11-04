import {describe,it,expect} from 'vitest';
import type {Review} from './data';
import {demoSnapshot,parseSnapshot,training,schedule} from './data';
describe('Shared versioned contract',()=>{
 it('loads the exact questions consumed by the native client',()=>{expect(demoSnapshot.schema_version).toBe('2.0');expect(training(demoSnapshot).map(q=>q.id).sort()).toEqual(Object.keys(demoSnapshot.questions).sort());expect(training(demoSnapshot).length).toBe(12)});
 it('rejects malformed model data',()=>{expect(()=>parseSnapshot({...demoSnapshot,schema_version:'999'})).toThrow()});
 it('excludes invalidated source material',()=>{const s=structuredClone(demoSnapshot);s.documents['demo-0'].deleted=true;expect(training(s,'Mathématiques')).toHaveLength(0)});
 it('uses the same SM-2 intervals as Python',()=>{let r:Review={question_id:'q',ease:2.5,repetitions:0,interval:0,due:'2026-01-01T00:00:00Z',last_grade:null};for(const n of [1,6,16]){r=schedule(r,5,new Date('2026-01-01T00:00:00Z'));expect(r.interval).toBe(n)}expect(schedule(r,1).repetitions).toBe(0)});
});
