import {readdir,readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
const dir=new URL('../apps/web/dist/',import.meta.url);
async function walk(root,prefix=''){let out=[];for(const e of await readdir(root,{withFileTypes:true})){const p=prefix+e.name;if(e.isDirectory())out.push(...await walk(new URL(e.name+'/',root),p+'/'));else out.push(p);}return out;}
const assets=(await walk(dir)).filter(p=>p!=='sw.js');
if(assets.some(p=>/originals|transcriptions|snapshot|\.pdf$|\.env|config\.json/.test(p)))throw Error('Forbidden private asset path');
const hash=createHash('sha256');for(const p of assets)hash.update(await readFile(new URL(p,dir)));
const code=`const CACHE='study-shell-${hash.digest('hex').slice(0,16)}';const ASSETS=${JSON.stringify(assets)};
self.addEventListener('install',event=>event.waitUntil(caches.open(CACHE).then(cache=>cache.addAll(ASSETS.map(p=>new URL(p,self.registration.scope).href)))));
self.addEventListener('activate',event=>event.waitUntil(caches.keys().then(keys=>Promise.all(keys.filter(k=>k.startsWith('study-shell-')&&k!==CACHE).map(k=>caches.delete(k))))));
self.addEventListener('fetch',event=>{const r=event.request;const u=new URL(r.url);const base=new URL(self.registration.scope);if(r.method!=='GET'||r.headers.has('Authorization')||u.origin!==base.origin||u.search||!u.pathname.startsWith(base.pathname))return;const path=u.pathname.slice(base.pathname.length)||'index.html';if(!ASSETS.includes(path))return;event.respondWith(caches.open(CACHE).then(c=>c.match(new URL(path,self.registration.scope).href)).then(cached=>cached||fetch(r)));});`;
await writeFile(new URL('sw.js',dir),code);
