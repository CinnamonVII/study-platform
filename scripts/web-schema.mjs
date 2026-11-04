import {createRequire} from 'node:module';
import {readFile,writeFile} from 'node:fs/promises';
const require=createRequire(new URL('../apps/web/package.json',import.meta.url));
const {compile}=require('json-schema-to-typescript');
const schema=JSON.parse(await readFile(new URL('../schemas/snapshot-v2.json',import.meta.url),'utf8'));
const ts=await compile(schema,'Snapshot',{bannerComment:'/* Generated from schemas/snapshot-v2.json. Run python -m engine.cli schema && node scripts/web-schema.mjs */',additionalProperties:false});
await writeFile(new URL('../apps/web/src/schema.d.ts',import.meta.url),ts);
