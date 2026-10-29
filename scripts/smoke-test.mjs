// End-to-end test using synthetic media only. Keeps test data under work/.
import {spawn,spawnSync} from 'node:child_process';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {fileURLToPath} from 'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const work=path.join(root,'work',`smoke-${Date.now()}`);await mkdir(work,{recursive:true});
const suffix=process.platform==='win32'?'.exe':'';
const ffmpeg=process.env.CASSETTE_FFMPEG||path.join(root,'tools',`ffmpeg${suffix}`);
function ff(args){const r=spawnSync(ffmpeg,['-v','error','-nostdin','-y',...args],{encoding:'utf8',windowsHide:true});assert.equal(r.status,0,r.stderr||r.error?.message);}
const wav=path.join(work,'Tone with edge silence.wav'),video=path.join(work,'Video with audio.mkv');
ff(['-f','lavfi','-i','sine=frequency=440:duration=4','-af','adelay=1000,apad=pad_dur=2','-ar','44100','-ac','2',wav]);
ff(['-f','lavfi','-i','color=c=0x315e51:s=160x90:r=5:d=10','-f','lavfi','-i','sine=frequency=880:duration=8','-filter:a','adelay=500,apad=pad_dur=1.5','-c:v','ffv1','-c:a','pcm_s16le','-ar','44100','-ac','2',video]);
const port=process.env.SMOKE_PORT||'3211';const base=`http://127.0.0.1:${port}`;
const server=spawn(path.join(root,'target','debug',`cassette-studio${suffix}`),[],{cwd:root,env:{...process.env,CASSETTE_PORT:port,CASSETTE_DATA:path.join(work,'data')},windowsHide:true,stdio:['ignore','pipe','pipe']});
let logs='';server.stdout.on('data',d=>logs+=d);server.stderr.on('data',d=>logs+=d);
async function api(route,body,method='POST'){const r=await fetch(base+route,body===undefined?{}:{method,headers:{'content-type':'application/json'},body:JSON.stringify(body)});const data=await r.json();assert.ok(r.ok,JSON.stringify(data));return data;}
const s={side_seconds:10,gap:1,fill_threshold:2,fade:1,continuation:'resume',rewind:1,silence_db:-50,silence_min:.1};
const close=(a,b,epsilon=.04)=>assert.ok(Math.abs(a-b)<epsilon,`${a} != ${b}`);
function rms(buffer,start,length){let sum=0,n=0;for(let i=Math.round(start*44100)*4;i<Math.min(buffer.length-44,Math.round((start+length)*44100)*4);i+=4){const v=buffer.readInt16LE(44+i);sum+=v*v;n++;}return Math.sqrt(sum/n);}
try{
 for(let i=0;i<80;i++){try{await api('/api/library');break;}catch{if(server.exitCode!==null)throw Error(logs);await new Promise(r=>setTimeout(r,150));if(i===79)throw Error('Server start timeout');}}
 const status=await api('/api/status');assert.ok(status.ffmpeg&&status.ffprobe,'FFmpeg is required');
 const form=new FormData();form.append('settings',JSON.stringify(s));form.append('files',new Blob([await readFile(wav)]),'Tone with edge silence.wav');form.append('files',new Blob([await readFile(video)]),'Video with audio.mkv');
 const imported=await (await fetch(base+'/api/import',{method:'POST',body:form})).json();assert.equal(imported.errors?.length,0,JSON.stringify(imported));assert.equal(imported.tracks.length,2);
 const [a,b]=imported.tracks;close(a.duration,7);close(a.trim_start,1);close(a.trim_end,5);close(b.duration,10);close(b.trim_start,.5);close(b.trim_end,8.5);
 const ids=imported.tracks.map(t=>t.id);
 const sides=await api('/api/plan',{tracks:ids,settings:{...s,rewind:3},optimize:false});assert.equal(sides[0].length,2);close(sides[0][1].duration,5);close(sides[0][1].fade_out,1);close(sides[1][0].offset,5);close(sides[1][0].duration,3);close(sides[1][0].fade_in,1);
 const restart=await api('/api/plan',{tracks:ids,settings:{...s,continuation:'restart'},optimize:false});close(restart[1][0].offset,0);close(restart[1][0].duration,8);close(restart[1][0].fade_in,0);
 const auto=await api('/api/plan',{tracks:ids,settings:s,optimize:true});assert.ok(auto.every(es=>es.reduce((n,e)=>n+e.duration,0)+Math.max(0,es.length-1)*s.gap<=10.001));
 const c=await api('/api/collections',{id:'',name:'Synthetic test cassette',kind:'cassette',tracks:ids,sides,settings:s});assert.ok(c.id);assert.ok(!Object.hasOwn(c.settings,'rewind'));
 await api('/api/collections',{id:'',name:'Synthetic test playlist',kind:'playlist',tracks:[...ids].reverse(),sides:[[],[]],settings:s});
 const cache=await api('/api/render',{sides,settings:s});assert.equal(cache.urls.length,2);
 const [outA,outB]=await Promise.all(cache.urls.map(async(u)=>Buffer.from(await (await fetch(base+u)).arrayBuffer())));
 for(const out of [outA,outB]){assert.equal(out.toString('ascii',0,4),'RIFF');assert.equal(out.readUInt32LE(40),44100*4*10);assert.equal(out.length,44+44100*4*10);}
 assert.ok(rms(outA,4.2,.3)<1,'Track gap must be silent');assert.ok(rms(outA,5.3,.2)>100,'Filler must have audio');assert.ok(rms(outA,9.85,.1)<rms(outA,9.1,.1)*.3,'A fade-out');
 assert.ok(rms(outB,.05,.1)<rms(outB,.8,.1)*.3,'B fade-in');assert.ok(rms(outB,4.2,.3)<1,'B tail padding must be silent');
 const preview=await api(`/api/preview/${a.id}`,{});const range=await fetch(base+preview.url,{headers:{Range:'bytes=0-99'}});assert.equal(range.status,206);assert.equal((await range.arrayBuffer()).byteLength,100);
 const fractional={...s,side_seconds:3*1.000015+2*.000015,gap:.000015};
 const short={track_id:a.id,offset:0,duration:1.000015,fade_in:0,fade_out:0,continuation:false};
 const rounded=await api('/api/render',{sides:[[short,short,short],[]],settings:fractional});
 const roundedWav=Buffer.from(await(await fetch(base+rounded.urls[0])).arrayBuffer());
 assert.equal(roundedWav.length,44+Math.round(fractional.side_seconds*44100)*4,'Exact sample length after segment rounding');
 const bad=await fetch(base+'/api/render',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({sides:[[...sides[0],sides[0][0]],[]],settings:s})});assert.equal(bad.status,400);
 const csrf=await fetch(base+'/api/collections',{method:'POST',headers:{origin:'https://unrelated.example','content-type':'application/json'},body:'{}'});assert.equal(csrf.status,403);
 const saved=JSON.parse(await readFile(path.join(work,'data','library.json'),'utf8'));assert.equal(saved.collections.length,2);assert.equal(saved.tracks.length,2);
 await writeFile(path.join(work,'result.json'),JSON.stringify({passed:true,checks:['audio/video import','edge silence removal','manual/auto placement','restart/resume','fade RMS','exact side WAV duration','inter-track/tail silence','HTTP range','overrun rejection','cross-origin rejection','persistent library'],work},null,2));
 console.log(`PASS: audio/video, silence, both continuation modes, PCM fades, durations, persistence, HTTP range, validation.\nTest data: ${work}`);
 if(process.env.SMOKE_KEEP==='1'){console.log(`UI test server: ${base}/`);await new Promise(()=>{});}
}finally{server.kill();}
