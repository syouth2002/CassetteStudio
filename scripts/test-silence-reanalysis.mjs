// End-to-end silence reanalysis with existing mixes and intentional internal pauses.
import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';
import {mkdir,readFile,writeFile,rename} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const work=path.join(root,'work',`silence-reanalysis-${Date.now()}`);await mkdir(work,{recursive:true});
function wav(seconds,sample){
 const rate=44100,n=Math.round(seconds*rate),b=Buffer.alloc(44+n*4);
 b.write('RIFF');b.writeUInt32LE(b.length-8,4);b.write('WAVEfmt ',8);b.writeUInt32LE(16,16);
 b.writeUInt16LE(1,20);b.writeUInt16LE(2,22);b.writeUInt32LE(rate,24);b.writeUInt32LE(rate*4,28);
 b.writeUInt16LE(4,32);b.writeUInt16LE(16,34);b.write('data',36);b.writeUInt32LE(n*4,40);
 for(let i=0;i<n;i++){const v=Math.round(6000*sample(i/rate));b.writeInt16LE(v,44+i*4);b.writeInt16LE(v,46+i*4);}return b;
}
const tone=t=>Math.sin(2*Math.PI*440*t);
const paused=wav(6,t=>((t>=.5&&t<1.5)||(t>=4.5&&t<5.5))?tone(t):0);
const tinyReturn=wav(3,t=>(t<.5||t>=2.99)?tone(t):0);
const recoveredId=randomUUID();
await mkdir(path.join(work,'data/sources'),{recursive:true});
const recoveredBytes=Buffer.concat([paused,Buffer.from([74,85,78,75,0,0,0,0])]);
recoveredBytes.writeUInt32LE(recoveredBytes.length-8,4);
await writeFile(path.join(work,'data/sources',recoveredId),recoveredBytes);
await writeFile(path.join(work,'data/library.json'),JSON.stringify({tracks:[{
 id:recoveredId,title:'Previously clipped after a pause',artist:'Synthetic',filename:'recovered.wav',source:'local',
 duration:6,trim_start:.5,trim_end:1.5
}],collections:[]}));
const server=spawn(path.join(root,'target/debug/cassette-studio'+(process.platform==='win32'?'.exe':'')),[],{
 cwd:root,windowsHide:true,env:{...process.env,CASSETTE_DATA:path.join(work,'data'),CASSETTE_PORT:'0'}});
let logs='',base;server.stdout.on('data',d=>logs+=d);server.stderr.on('data',d=>logs+=d);
async function call(route,body,method='POST',ok=true){
 const r=await fetch(base+route,body===undefined?{}:{method,headers:{'content-type':'application/json'},body:JSON.stringify(body)});
 const data=await r.json();if(ok)assert.ok(r.ok,JSON.stringify(data));else assert.equal(r.status,400,JSON.stringify(data));return data;
}
const settings={side_seconds:6,gap:0,fade:.5,silence_db:-50,silence_min:1,continuation:'resume'};
const near=(a,b)=>assert.ok(Math.abs(a-b)<.002,`${a} != ${b}`);
const entry=(id,offset,duration,fade=0)=>({track_id:id,offset,duration,fade_in:fade,fade_out:fade,continuation:offset>0});
async function upload(name,data){const form=new FormData();form.append('settings',JSON.stringify(settings));form.append('files',new Blob([data]),name);const r=await fetch(base+'/api/import',{method:'POST',body:form});const j=await r.json();assert.ok(r.ok,JSON.stringify(j));assert.deepEqual(j.errors,[]);return j.tracks[0];}
const collection=(name,sides,s=settings)=>call('/api/collections',{id:'',name,kind:'cassette',tracks:[],sides,settings:s});
const reanalyze=(ids,min)=>call('/api/reanalyze',{tracks:ids,settings:{...settings,silence_min:min}},'PUT');
function rms(b,start,end){let sum=0,n=0;for(let i=Math.round(start*44100);i<Math.round(end*44100);i++){const v=b.readInt16LE(44+i*4);sum+=v*v;n++;}return Math.sqrt(sum/n);}
try{
 for(let i=0;i<100;i++){base=logs.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];if(base)break;await new Promise(r=>setTimeout(r,50));}assert.ok(base,logs);
 const a=await upload('Intentional pause.wav',paused),b=await upload('Unselected.wav',wav(6,t=>((t>=.5&&t<1.5)||(t>=4.5&&t<5.5))?.8*tone(t):0));
 const reprise=await upload('Short final return.wav',tinyReturn);near(reprise.trim_end,3);near(a.trim_start,0);near(a.trim_end,6);
 await call(`/api/tracks/${a.id}/gain`,{gain_db:-3},'PUT');
 const full=await collection('Full track',[[entry(a.id,0,6)],[]]);
 const split=await collection('Continuation',[[entry(a.id,0,3,.5)],[entry(a.id,3,3,.5)]]);
 const manual=await collection('Manual ranges',[[entry(a.id,1,2,.5),entry(a.id,5.25,.75,.5),entry(a.id,5.7,.3)],[]]);
 const playlist=await call('/api/collections',{id:'',name:'Playlist',kind:'playlist',tracks:[a.id,b.id],sides:[[],[]],settings});
 const recoveredMix=await collection('Recover hidden second section',[[entry(recoveredId,0,1)],[]]);
 const recovered=await reanalyze([recoveredId],.5);
 near(recovered.tracks.find(t=>t.id===recoveredId).trim_end,5.5);
 near(recovered.collections.find(c=>c.id===recoveredMix.id).sides[0][0].duration,5);
 const initial=await call('/api/library');
 let lib=await reanalyze([a.id,a.id],.5),track=lib.tracks.find(t=>t.id===a.id);
 near(track.trim_start,.5);near(track.trim_end,5.5);assert.equal(track.manual_gain_db,-3);
 assert.deepEqual(lib.tracks.find(t=>t.id===b.id),initial.tracks.find(t=>t.id===b.id));
 assert.deepEqual(lib.tracks.find(t=>t.id===reprise.id),initial.tracks.find(t=>t.id===reprise.id));
 assert.deepEqual(lib.collections.find(c=>c.id===playlist.id).tracks,[a.id,b.id]);
 const find=id=>lib.collections.find(c=>c.id===id);
 near(find(full.id).sides[0][0].duration,5);
 near(find(split.id).sides[0][0].duration,2.5);near(find(split.id).sides[1][0].offset,2.5);near(find(split.id).sides[1][0].duration,2.5);
 const ranges=find(manual.id).sides[0];assert.equal(ranges.length,2);near(ranges[0].offset,.5);near(ranges[0].duration,2);
 near(ranges[1].duration,.25);near(ranges[1].fade_in,.25);near(ranges[1].fade_out,.25);
 const output=await call('/api/render',{sides:find(full.id).sides,settings});
 const pcm=Buffer.from(await(await fetch(base+output.urls[0])).arrayBuffer());
 assert.ok(rms(pcm,1.2,3.8)<1,'Internal pause must remain');assert.ok(rms(pcm,4.1,4.7)>500,'Music after pause must remain');
 const overflow=await collection('Capacity warning',[[entry(a.id,0,5)],[]],{...settings,side_seconds:5.5});
 lib=await reanalyze([a.id],1);near(lib.collections.find(c=>c.id===full.id).sides[0][0].duration,6);
 const over=lib.collections.find(c=>c.id===overflow.id);near(over.sides[0][0].duration,6);
 await call('/api/render',{sides:over.sides,settings:over.settings},'POST',false);
 const beforeFailure=await call('/api/library'),sourceB=path.join(work,'data/sources',b.id);
 await rename(sourceB,sourceB+'.held');
 try{await call('/api/reanalyze',{tracks:[a.id,b.id],settings:{...settings,silence_min:.5}},'PUT',false);}finally{await rename(sourceB+'.held',sourceB);}
 assert.deepEqual(await call('/api/library'),beforeFailure,'Failed batch must be atomic');
 await call('/api/reanalyze',{tracks:[],settings},'PUT',false);
 await call('/api/reanalyze',{tracks:['missing'],settings},'PUT',false);
 lib=await call('/api/reanalyze',{tracks:[a.id],settings:{...settings,silence_db:-10,silence_min:.5}},'PUT');
 const silent=lib.tracks.find(t=>t.id===a.id);near(silent.trim_start,0);near(silent.trim_end,0);
 assert.ok(lib.collections.filter(c=>c.kind==='cassette').every(c=>c.sides.every(es=>!es.some(e=>e.track_id===a.id))));
 assert.deepEqual(await readFile(path.join(work,'data/sources',a.id)),paused);
 assert.deepEqual(JSON.parse(await readFile(path.join(work,'data/library.json'),'utf8')),lib);
 console.log('PASS: selected-only reanalysis, existing mixes, whole/partial/continued ranges, clipping/fades, zero-length entries, internal pauses and short return, PCM, capacity warning, gain, atomic failure, persistence, untouched sources');
 console.log(work);
}finally{server.kill();}
