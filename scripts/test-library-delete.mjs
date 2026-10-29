// Library removal checks use only synthetic files and an isolated library.
import {spawn} from 'node:child_process';
import {mkdir,readFile,writeFile,readdir} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const work=path.join(root,'work',`library-delete-${Date.now()}`),data=path.join(work,'data');await mkdir(work,{recursive:true});
function wav(freq){const n=44100,b=Buffer.alloc(44+n*4);b.write('RIFF');b.writeUInt32LE(b.length-8,4);b.write('WAVEfmt ',8);b.writeUInt32LE(16,16);b.writeUInt16LE(1,20);b.writeUInt16LE(2,22);b.writeUInt32LE(44100,24);b.writeUInt32LE(176400,28);b.writeUInt16LE(4,32);b.writeUInt16LE(16,34);b.write('data',36);b.writeUInt32LE(n*4,40);for(let i=0;i<n;i++){const v=Math.round(5000*Math.sin(2*Math.PI*freq*i/44100));b.writeInt16LE(v,44+i*4);b.writeInt16LE(v,46+i*4);}return b;}
const sources=[wav(440),wav(660),wav(880)];await writeFile(path.join(work,'original.wav'),sources[0]);
const server=spawn(path.join(root,'target/debug/cassette-studio'+(process.platform==='win32'?'.exe':'')),[],{cwd:root,windowsHide:true,env:{...process.env,CASSETTE_DATA:data,CASSETTE_PORT:'0'}});
let logs='',base;server.stdout.on('data',x=>logs+=x);server.stderr.on('data',x=>logs+=x);
async function api(route,body,method='POST',expected=200){const r=await fetch(base+route,body===undefined?{}:{method,headers:{'content-type':'application/json'},body:JSON.stringify(body)});const j=await r.json();assert.equal(r.status,expected,JSON.stringify(j));return j;}
const settings={side_seconds:10,gap:0,fade:.5};
const entry=(id,offset=0,duration=1)=>({track_id:id,offset,duration,fade_in:0,fade_out:0,continuation:offset>0});
try{
 for(let i=0;i<100;i++){base=logs.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];if(base)break;await new Promise(r=>setTimeout(r,50));}assert.ok(base,logs);
 const form=new FormData();for(let i=0;i<sources.length;i++)form.append('files',new Blob([sources[i]]),`source-${i}.wav`);
 const imported=await(await fetch(base+'/api/import',{method:'POST',body:form})).json();assert.deepEqual(imported.errors,[]);assert.equal(imported.tracks.length,3);
 const [a,b,c]=imported.tracks;
 const playlist=await api('/api/collections',{id:'',name:'Playlist',kind:'playlist',tracks:[a.id,a.id,b.id,c.id],sides:[[],[]],settings});
 const tape=await api('/api/collections',{id:'',name:'Tape',kind:'cassette',tracks:[a.id,b.id],sides:[[entry(a.id),entry(b.id)],[entry(a.id,.5,.5),entry(c.id)]],settings});
 const before=await api('/api/library');
 await api('/api/tracks',{tracks:[]},'DELETE',400);
 await api('/api/tracks',{tracks:Array(5001).fill(a.id)},'DELETE',400);
 assert.deepEqual(await api('/api/library'),before);
 let lib=await api('/api/tracks',{tracks:[a.id,a.id]},'DELETE');
 assert.deepEqual(lib.tracks.map(t=>t.id),[b.id,c.id]);
 assert.deepEqual(lib.collections.find(x=>x.id===playlist.id).tracks,[b.id,c.id]);
 const updated=lib.collections.find(x=>x.id===tape.id);assert.deepEqual(updated.tracks,[b.id]);assert.deepEqual(updated.sides,[[entry(b.id)],[entry(c.id)]]);
 assert.deepEqual(lib.tracks,before.tracks.filter(t=>t.id!==a.id));
 assert.deepEqual(await readFile(path.join(work,'original.wav')),sources[0]);
 for(let i=0;i<3;i++)assert.deepEqual(await readFile(path.join(data,'sources',imported.tracks[i].id)),sources[i]);
 assert.deepEqual(await api('/api/tracks',{tracks:[a.id]},'DELETE'),lib);
 lib=await api('/api/tracks',{tracks:[b.id,c.id]},'DELETE');assert.deepEqual(lib.tracks,[]);
 assert.ok(lib.collections.every(x=>x.tracks.length===0&&x.sides.every(es=>es.length===0)));
 assert.equal(lib.collections.length,2);assert.equal((await readdir(path.join(data,'sources'))).length,3);
 assert.deepEqual(JSON.parse(await readFile(path.join(data,'library.json'),'utf8')),lib);
 const again=new FormData();again.append('files',new Blob([sources[0]]),'reimport.wav');
 const added=await(await fetch(base+'/api/import',{method:'POST',body:again})).json();assert.deepEqual(added.errors,[]);assert.equal(added.tracks.length,1);assert.notEqual(added.tracks[0].id,a.id);
 assert.deepEqual((await api('/api/library')).collections,lib.collections);
 console.log('PASS: library deletion, all playlist/cassette references, continuation, idempotent retry, empty/oversized requests, persistence, retained originals/imported copies, reimport without resurrecting references');console.log(work);
}finally{server.kill();}
