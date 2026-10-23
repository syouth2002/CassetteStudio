import {spawn} from 'node:child_process';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const dir=path.join(root,'work',`batch-${Date.now()}`);await mkdir(dir,{recursive:true});
const settings={side_seconds:50,gap:0,fill_threshold:0,fade:5,continuation:'resume',rewind:3,silence_db:-50,silence_min:.2};
const tracks=[30,60,80,40,90,120].map((duration,i)=>({id:randomUUID(),title:`Test ${i}`,artist:'Synthetic fixture',filename:'fixture.wav',source:'local',duration,trim_start:0,trim_end:duration}));
const blank=(name,seconds)=>({id:randomUUID(),name,kind:'cassette',tracks:[],sides:[[],[]],settings:{...settings,side_seconds:seconds}});
const used=blank('収録済み',50);used.sides[0]=[{track_id:tracks[0].id,offset:0,duration:30,fade_in:0,fade_out:0,continuation:false}];
const short=blank('短いカセット',50),long=blank('長いカセット',100),spare=blank('予備カセット',180),spare2=blank('予備カセット2',240);
await writeFile(path.join(dir,'library.json'),JSON.stringify({tracks,collections:[used,short,long,spare,spare2]}));
const server=spawn(path.join(root,'target','debug',`cassette-studio${process.platform==='win32'?'.exe':''}`),[],{cwd:root,windowsHide:true,env:{...process.env,CASSETTE_PORT:'3212',CASSETTE_DATA:dir},stdio:'ignore'});
const base='http://127.0.0.1:3212';
const post=(body)=>fetch(base+'/api/fill-cassettes',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});
try{
 for(let i=0;i<80;i++){try{await fetch(base+'/api/library');break;}catch{await new Promise(r=>setTimeout(r,100));if(i===79)throw Error('Startup timeout');}}
 const response=await post({tracks:[...tracks.map(t=>t.id),tracks[1].id],targets:[short.id,long.id]});assert.equal(response.status,200);const result=await response.json();
 assert.equal(result.collections.length,2);assert.equal(result.excluded,1);
 const seen=new Set();for(const c of result.collections){assert.ok([50,100].includes(c.settings.side_seconds));for(const id of c.tracks){assert.notEqual(id,tracks[0].id);assert.ok(!seen.has(id));seen.add(id);}for(const es of c.sides){assert.ok(es.reduce((n,e)=>n+e.duration,0)<=c.settings.side_seconds+.001);}}
 const saved=JSON.parse(await readFile(path.join(dir,'library.json'),'utf8'));assert.deepEqual(saved.collections[0],used);assert.deepEqual(saved.collections[3],spare);assert.equal(saved.collections[1].name,short.name);
 const before=await readFile(path.join(dir,'library.json'),'utf8');const bad=await post({tracks:tracks.map(t=>t.id),targets:[spare.id,short.id]});assert.equal(bad.status,400);assert.equal(await readFile(path.join(dir,'library.json'),'utf8'),before);
 console.log(`PASS: batch API, saved tape lengths, no reused tracks, persistence, filled-tape protection, atomic failure.\n${dir}`);
 if(process.env.BATCH_KEEP==='1'){console.log(`UI fixture: ${base}/`);await new Promise(()=>{});}
}finally{server.kill();}
