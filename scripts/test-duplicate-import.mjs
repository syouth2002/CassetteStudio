// Duplicate ingestion checks use synthetic media and an isolated legacy library.
import {spawn} from 'node:child_process';
import {mkdir,readFile,writeFile,readdir,rename} from 'node:fs/promises';
import {randomUUID,createHash} from 'node:crypto';
import {once} from 'node:events';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const work=path.join(root,'work',`duplicate-import-${Date.now()}`),data=path.join(work,'data');
await mkdir(path.join(data,'sources'),{recursive:true});
function wav(frequency){const n=44100,b=Buffer.alloc(44+n*4);b.write('RIFF');b.writeUInt32LE(b.length-8,4);b.write('WAVEfmt ',8);b.writeUInt32LE(16,16);b.writeUInt16LE(1,20);b.writeUInt16LE(2,22);b.writeUInt32LE(44100,24);b.writeUInt32LE(176400,28);b.writeUInt16LE(4,32);b.writeUInt16LE(16,34);b.write('data',36);b.writeUInt32LE(n*4,40);for(let i=0;i<n;i++){const v=Math.round(5000*Math.sin(2*Math.PI*frequency*i/44100));b.writeInt16LE(v,44+i*4);b.writeInt16LE(v,46+i*4);}return b;}
const a=wav(440),b=wav(660),c=wav(880),d=wav(220),e=wav(1000);
const legacy={id:randomUUID(),title:'Existing custom title',artist:'Synthetic',filename:'old.wav',source:'local',duration:1,trim_start:0,trim_end:1,manual_gain_db:-4};
await writeFile(path.join(data,'sources',legacy.id),d);
await writeFile(path.join(data,'library.json'),JSON.stringify({tracks:[legacy],collections:[]}));
let server,base,logs;
async function start(extra={}){logs='';base=null;server=spawn(path.join(root,'target/debug/cassette-studio'+(process.platform==='win32'?'.exe':'')),[],{cwd:root,windowsHide:true,env:{...process.env,CASSETTE_DATA:data,CASSETTE_PORT:'0',...extra}});server.stdout.on('data',x=>logs+=x);server.stderr.on('data',x=>logs+=x);for(let i=0;i<100;i++){base=logs.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];if(base)return;await new Promise(r=>setTimeout(r,50));}throw Error(logs);}
async function stop(){if(server&&server.exitCode===null){server.kill();await once(server,'exit');}}
async function api(route,body,method='POST'){const r=await fetch(base+route,body===undefined?{}:{method,headers:{'content-type':'application/json'},body:JSON.stringify(body)});const j=await r.json();assert.ok(r.ok,JSON.stringify(j));return j;}
async function upload(files,settings){const form=new FormData();if(settings)form.append('settings',JSON.stringify(settings));for(const [name,buffer]of files)form.append('files',new Blob([buffer]),name);const r=await fetch(base+'/api/import',{method:'POST',body:form});const j=await r.json();assert.ok(r.ok,JSON.stringify(j));return j;}
const hash=bytes=>'sha256:'+createHash('sha256').update(bytes).digest('hex');
try{
 await start();
 const old=await upload([['renamed legacy.wav',d]]);assert.equal(old.tracks.length,0);assert.equal(old.duplicates.length,1);
 assert.equal(old.duplicates[0].track.id,legacy.id);assert.equal(old.duplicates[0].track.manual_gain_db,-4);assert.equal(old.duplicates[0].track.title,legacy.title);
 assert.equal(old.duplicates[0].track.content_hash,hash(d));assert.equal(old.duplicates[0].track.dc,null,'Duplicate must not rerun audio analysis');
 const first=await upload([['same.wav',a],['different-name.wav',a],['same.wav',a]]);assert.deepEqual(first.errors,[]);
 assert.equal(first.tracks.length,1);assert.equal(first.duplicates.length,2);const ta=first.tracks[0];
 assert.equal(ta.content_hash,hash(a));assert.deepEqual(first.track_ids,[ta.id,ta.id,ta.id]);
 const different=await upload([['same.wav',b]]);assert.equal(different.tracks.length,1);assert.equal(different.duplicates.length,0);const tb=different.tracks[0];assert.notEqual(ta.id,tb.id);
 const mixed=await upload([['b-copy.wav',b],['c-new.wav',c],['a-copy.wav',a]]);assert.equal(mixed.tracks.length,1);assert.equal(mixed.duplicates.length,2);assert.deepEqual(mixed.track_ids,[tb.id,mixed.tracks[0].id,ta.id]);
 const concurrent=await Promise.all([upload([['e-one.wav',e]]),upload([['e-two.wav',e]])]);assert.equal(concurrent.reduce((n,r)=>n+r.tracks.length,0),1);assert.equal(concurrent.reduce((n,r)=>n+r.duplicates.length,0),1);assert.equal(concurrent[0].track_ids[0],concurrent[1].track_ids[0]);
 await api(`/api/tracks/${ta.id}/gain`,{gain_db:-3},'PUT');
 const changed=await upload([['again.wav',a]],{side_seconds:30,normalize:true,silence_min:.1});assert.equal(changed.tracks.length,0);assert.equal(changed.duplicates[0].track.manual_gain_db,-3);
 const reanalyzed=await api('/api/reanalyze',{tracks:[ta.id],settings:{side_seconds:30,silence_min:.5}},'PUT');assert.equal(reanalyzed.tracks.find(t=>t.id===ta.id).content_hash,hash(a));
 const invalid=await upload([['invalid.wav',Buffer.from('not an audio file')]]);assert.equal(invalid.errors.length,1);assert.equal(invalid.tracks.length,0);
 let lib=await api('/api/library');assert.equal(lib.tracks.length,5);assert.equal(new Set(lib.tracks.map(t=>t.content_hash)).size,5);
 assert.equal((await readdir(path.join(data,'sources'))).length,5);assert.deepEqual(await readdir(path.join(data,'incoming')),[]);
 assert.deepEqual(await readFile(path.join(data,'sources',legacy.id)),d);assert.deepEqual(await readFile(path.join(data,'sources',ta.id)),a);
 await stop();await start({CASSETTE_FFPROBE:path.join(work,'intentionally-unavailable-ffprobe')});
 const cached=await upload([['after restart.wav',a]]);assert.equal(cached.errors.length,0);assert.equal(cached.tracks.length,0);assert.equal(cached.duplicates[0].track.id,ta.id);
 const source=path.join(data,'sources',ta.id);await rename(source,path.join(work,'held-source.wav'));
 const restored=await upload([['restore missing copy.wav',a]]);assert.equal(restored.duplicates[0].track.id,ta.id);assert.deepEqual(await readFile(source),a);
 lib=await api('/api/library');assert.equal(lib.tracks.length,5);assert.equal(lib.tracks.find(t=>t.id===ta.id).manual_gain_db,-3);
 assert.deepEqual(JSON.parse(await readFile(path.join(data,'library.json'),'utf8')),lib);
 assert.deepEqual(await readdir(path.join(data,'incoming')),[]);
 console.log('PASS: same-batch/repeated/renamed files, same-name different content, ordered IDs, concurrent imports, legacy identities, persistence/restart, skipped analysis, preserved settings/gain, source cleanup and recovery');console.log(work);
}finally{await stop();}
