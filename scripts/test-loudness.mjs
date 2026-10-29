import {spawn,spawnSync} from 'node:child_process';
import {mkdir,writeFile,readFile,copyFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const dir=path.join(root,'work',`loudness-${Date.now()}`),data=path.join(dir,'data');
await mkdir(path.join(data,'sources'),{recursive:true});
const ff=path.join(root,'tools',`ffmpeg${process.platform==='win32'?'.exe':''}`);
const run=args=>{const r=spawnSync(ff,['-hide_banner','-nostats','-nostdin','-y',...args],{encoding:'utf8',windowsHide:true});assert.equal(r.status,0,r.stderr);return r;};
const quiet=path.join(dir,'Quiet.wav'),loud=path.join(dir,'Loud.wav');
for(const [file,volume]of [[quiet,.1],[loud,1]])run(['-f','lavfi','-i','sine=frequency=440:duration=8','-af',`volume=${volume}`,'-ac','2','-ar','44100',file]);
const legacy={id:randomUUID(),title:'旧データの小さな曲',artist:'Test',filename:'legacy.wav',source:'local',duration:8,trim_start:0,trim_end:8};
await copyFile(quiet,path.join(data,'sources',legacy.id));
await writeFile(path.join(data,'library.json'),JSON.stringify({tracks:[legacy],collections:[]}));
const server=spawn(path.join(root,'target','debug',`cassette-studio${process.platform==='win32'?'.exe':''}`),[],{cwd:root,windowsHide:true,env:{...process.env,CASSETTE_PORT:'3213',CASSETTE_DATA:data},stdio:'ignore'});
const base='http://127.0.0.1:3213';
async function api(route,body,method='POST'){const r=await fetch(base+route,body===undefined?{}:{method,headers:{'content-type':'application/json'},body:JSON.stringify(body)});const j=await r.json();assert.ok(r.ok,JSON.stringify(j));return j;}
const s={side_seconds:16,gap:0,fill_threshold:30,fade:1,continuation:'resume',silence_db:-50,silence_min:.2,normalize:true,target_lufs:-18,peak_db:-1};
const entry=(id,offset=0,duration=8)=>({track_id:id,offset,duration,fade_in:0,fade_out:0,continuation:false});
async function download(url,name){const file=path.join(dir,name);await writeFile(file,Buffer.from(await(await fetch(base+url)).arrayBuffer()));return file;}
function measure(file,start=0,length=8){const filter=`atrim=start=${start}:duration=${length},asetpts=PTS-STARTPTS,loudnorm=I=-18:TP=-1:LRA=50:print_format=json`;const out=run(['-i',file,'-af',filter,'-f','null','-']).stderr;const text=out.slice(out.lastIndexOf('{'),out.lastIndexOf('}')+1);const j=JSON.parse(text);return {i:+j.input_i,peak:+j.input_tp};}
const near=(a,b,tolerance=.2)=>assert.ok(Math.abs(a-b)<tolerance,`${a} != ${b}`);
try{
 for(let i=0;i<80;i++){try{await fetch(base+'/api/library');break;}catch{await new Promise(r=>setTimeout(r,100));if(i===79)throw Error('Startup timeout');}}
 const imported=new FormData();imported.append('files',new Blob([await readFile(loud)]),'Loud.wav');
 const response=await(await fetch(base+'/api/import',{method:'POST',body:imported})).json();assert.equal(response.errors.length,0,JSON.stringify(response));const loudTrack=response.tracks[0];assert.ok(loudTrack.loudness.integrated_lufs<0);
 const ids=[legacy.id,loudTrack.id],stats=await api('/api/loudness',{tracks:ids,settings:s});assert.equal(stats.tracks.length,2);near(stats.tracks[1].loudness.integrated_lufs-stats.tracks[0].loudness.integrated_lufs,20,.1);
 const sides=[[entry(legacy.id),entry(loudTrack.id)],[]];
 const result=await api('/api/render',{sides,settings:s});const normalized=await download(result.urls[0],'normalized.wav');
 const first=measure(normalized,0,8),second=measure(normalized,8,8);near(first.i,-18);near(second.i,-18);near(first.i,second.i,.1);
 const preview=await api(`/api/preview/${legacy.id}`,s);const previewFile=await download(preview.url,'preview.wav');near(measure(previewFile).i,first.i,.1);
 const original=await api('/api/render',{sides,settings:{...s,normalize:false}});const off=await download(original.urls[0],'off.wav');near(measure(off,8).i-measure(off).i,20,.2);
 await api(`/api/tracks/${legacy.id}/gain`,{gain_db:-3},'PUT');
 const adjusted=await api(`/api/preview/${legacy.id}`,s);assert.notEqual(adjusted.url,preview.url);const adjustedFile=await download(adjusted.url,'manual.wav');near(measure(adjustedFile).i,-21);
 const continued=await api('/api/render',{sides:[[entry(legacy.id,0,4)],[{...entry(legacy.id,4,4),continuation:true}]],settings:{...s,side_seconds:4}});
 const a=await download(continued.urls[0],'continued-a.wav'),b=await download(continued.urls[1],'continued-b.wav');near(measure(a,0,4).i,measure(b,0,4).i,.1);near(measure(a,0,4).i,-21);
 const peakSettings={...s,target_lufs:-8,peak_db:-12};const peak=await api(`/api/preview/${loudTrack.id}`,peakSettings);const peakFile=await download(peak.url,'peak.wav');assert.ok(measure(peakFile).peak<=-12,'True peak ceiling');
 const report=await api('/api/loudness',{tracks:[loudTrack.id],settings:peakSettings});assert.ok(report.reports[loudTrack.id].limited);
 const bad=await fetch(base+`/api/tracks/${legacy.id}/gain`,{method:'PUT',headers:{'content-type':'application/json'},body:JSON.stringify({gain_db:25})});assert.equal(bad.status,400);
 const saved=JSON.parse(await readFile(path.join(data,'library.json'),'utf8'));assert.equal(saved.tracks[0].manual_gain_db,-3);assert.ok(saved.tracks[0].loudness);
 await writeFile(path.join(dir,'result.json'),JSON.stringify({passed:true,first,second,checks:['legacy migration','20 dB input difference','target LUFS','preview/render agreement','normalization OFF','manual gain','A/B continuation','true peak cap','cache invalidation','persistence','validation']},null,2));
 console.log(`PASS: 20 dB input difference normalized to ${first.i}/${second.i} LUFS; peak ceiling, manual gain, preview, continuation, legacy data, persistence.\n${dir}`);
 if(process.env.LOUDNESS_KEEP==='1'){console.log(`UI: ${base}/`);await new Promise(()=>{});}
}finally{server.kill();}
