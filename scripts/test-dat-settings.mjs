// Real PCM checks for cassette leading silence and DAT-oriented WAV formats.
import {spawn} from 'node:child_process';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const work=path.join(root,'work',`dat-settings-${Date.now()}`);
await mkdir(work,{recursive:true});
const source=Buffer.alloc(44+2*44100*4);
source.write('RIFF');source.writeUInt32LE(source.length-8,4);source.write('WAVEfmt ',8);
source.writeUInt32LE(16,16);source.writeUInt16LE(1,20);source.writeUInt16LE(2,22);
source.writeUInt32LE(44100,24);source.writeUInt32LE(44100*4,28);source.writeUInt16LE(4,32);source.writeUInt16LE(16,34);
source.write('data',36);source.writeUInt32LE(source.length-44,40);
for(let i=0;i<2*44100;i++){const v=Math.round(5000*Math.sin(2*Math.PI*400*i/44100));source.writeInt16LE(v,44+i*4);source.writeInt16LE(-v,46+i*4);}
const file=path.join(work,'tone.wav');await writeFile(file,source);
const server=spawn(path.join(root,'target/debug/cassette-studio'+(process.platform==='win32'?'.exe':'')),[],{
 cwd:root,windowsHide:true,env:{...process.env,CASSETTE_DATA:path.join(work,'data'),CASSETTE_PORT:'0'}});
let logs='';server.stdout.on('data',d=>logs+=d);server.stderr.on('data',d=>logs+=d);
let base;
async function api(route,body){const r=await fetch(base+route,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});const data=await r.json();assert.ok(r.ok,JSON.stringify(data));return data;}
function rms(wav,start,end){const rate=wav.readUInt32LE(24);let sum=0,count=0;for(let i=Math.round(start*rate);i<Math.round(end*rate);i++){const v=wav.readInt16LE(44+i*4);sum+=v*v;count++;}return Math.sqrt(sum/count);}
function exactSilence(wav,start,end){const rate=wav.readUInt32LE(24);assert.ok(wav.subarray(44+Math.round(start*rate)*4,44+Math.round(end*rate)*4).every(v=>v===0));}
try{
 for(let i=0;i<100;i++){base=logs.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];if(base)break;await new Promise(r=>setTimeout(r,50));}
 assert.ok(base,logs);
 const form=new FormData();form.append('files',new Blob([source],{type:'audio/wav'}),'tone.wav');
 const imported=await(await fetch(base+'/api/import',{method:'POST',body:form})).json();assert.equal(imported.errors.length,0);
 const track=imported.tracks[0];
 const entry={track_id:track.id,offset:0,duration:1,fade_in:0,fade_out:0,continuation:false};
 const settings={side_seconds:4,lead_in:.5,gap:.5,fade:.5};
 const sides=[[entry,{...entry,offset:1}],[{...entry,offset:1,fade_in:.5,fade_out:.5,continuation:true}]];
 for(const selection of [undefined,48000,44100]){
  const configured={...settings,...(selection===undefined?{}:{wav_sample_rate:selection})};
  const result=await api('/api/render',{sides,settings:configured});
  for(const [side,url] of result.urls.entries()){
   const wav=Buffer.from(await(await fetch(base+url)).arrayBuffer());const rate=selection??44100;
   assert.equal(wav.readUInt32LE(24),rate);assert.equal(wav.readUInt32LE(28),rate*4);
   assert.equal(wav.readUInt16LE(22),2);assert.equal(wav.readUInt16LE(34),16);
   assert.equal(wav.length,44+4*rate*4);exactSilence(wav,0,.5);
   assert.ok(rms(wav,.7,.9)>100);
   if(side===0){exactSilence(wav,1.5,2);assert.ok(rms(wav,2.2,2.5)>1000);exactSilence(wav,3,4);}
   else{assert.ok(rms(wav,.5,.6)<rms(wav,.9,1));exactSilence(wav,1.5,4);}
  }
 }
 // Trial playback remains a whole-track 44.1 kHz preview, without cassette lead-in.
 const preview=await api('/api/preview/'+track.id,{...settings,wav_sample_rate:48000});
 const previewWav=Buffer.from(await(await fetch(base+preview.url)).arrayBuffer());
 assert.equal(previewWav.readUInt32LE(24),44100);assert.ok(rms(previewWav,0,.1)>1000);
 const capacity={side_seconds:2,lead_in:.5,gap:0,fade:.5,fill_threshold:0,continuation:'resume',wav_sample_rate:48000};
 for(const optimize of [false,true]){
  const planned=await api('/api/plan',{tracks:[track.id],settings:capacity,optimize});
  assert.equal(planned[0][0].duration,1.5);assert.equal(planned[1][0].offset,1.5);
  assert.equal(planned[1][0].duration,.5);
  for(const entries of planned)assert.ok(entries.reduce((n,e)=>n+e.duration,capacity.lead_in)<=2);
  await api('/api/render',{sides:planned,settings:capacity});
 }
 const saved=await api('/api/collections',{id:'',name:'DAT settings',kind:'cassette',tracks:[],sides,settings:{...settings,wav_sample_rate:48000}});
 const persisted=JSON.parse(await readFile(path.join(work,'data/library.json'),'utf8')).collections.find(c=>c.id===saved.id);
 assert.equal(persisted.settings.lead_in,.5);assert.equal(persisted.settings.wav_sample_rate,48000);
 for(const bad of [{lead_in:.25},{lead_in:5},{lead_in:-.5},{wav_sample_rate:96000}]){
  const r=await fetch(base+'/api/render',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({sides,settings:{...settings,...bad}})});
  assert.equal(r.status,400);
 }
 const overflow=await fetch(base+'/api/render',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({sides,settings:{...settings,side_seconds:2.5}})});
 assert.equal(overflow.status,400,'Lead-in counts toward capacity');
 assert.deepEqual(await readFile(file),source);
 console.log('PASS: A/B lead-in, exact PCM gaps/fades/length, 16-bit 44.1/48 kHz and default, preview isolation, capacity, continuation, persistence, validation');
}finally{server.kill();}
