// End-to-end DC correction tests; all media and libraries are synthetic and isolated.
import {spawn,spawnSync} from 'node:child_process';
import {mkdir,readFile,writeFile,copyFile} from 'node:fs/promises';
import {randomUUID,createHash} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const dir=path.join(root,'work',`dc-${Date.now()}`),data=path.join(dir,'data');
await mkdir(path.join(data,'sources'),{recursive:true});
const ff=process.env.CASSETTE_FFMPEG||path.join(root,'tools',process.platform==='win32'?'ffmpeg.exe':'ffmpeg');
async function fixture(name,seconds,channels,sample){
 const frames=Math.round(seconds*44100),buffer=Buffer.alloc(44+frames*channels*4);
 buffer.write('RIFF');buffer.writeUInt32LE(buffer.length-8,4);buffer.write('WAVEfmt ',8);buffer.writeUInt32LE(16,16);buffer.writeUInt16LE(3,20);buffer.writeUInt16LE(channels,22);buffer.writeUInt32LE(44100,24);buffer.writeUInt32LE(44100*channels*4,28);buffer.writeUInt16LE(channels*4,32);buffer.writeUInt16LE(32,34);buffer.write('data',36);buffer.writeUInt32LE(frames*channels*4,40);
 for(let i=0;i<frames;i++)for(let c=0;c<channels;c++)buffer.writeFloatLE(sample(i/44100,c),44+(i*channels+c)*4);
 const file=path.join(dir,name);await writeFile(file,buffer);return file;
}
const signal=(t,c)=>(c?.06:.08)*Math.sin(2*Math.PI*(c?880:440)*t);
const biased=await fixture('biased.wav',8,2,(t,c)=>signal(t,c)+(c?-.07:.12));
const clean=await fixture('clean.wav',8,2,signal);
const mono=await fixture('mono.wav',4,1,t=>.09+.05*Math.sin(2*Math.PI*220*t));
const silent=await fixture('silent.wav',2,2,()=>0);
const edge=await fixture('dc-silence.wav',6,2,(t,c)=>(c?-.07:.12)+(t>=1&&t<5?signal(t,c):0));
const zeroEdge=await fixture('zero-silence.wav',6,2,(t,c)=>t>=1&&t<5?signal(t,c)+(c?-.07:.12):0);
const hot=await fixture('hot.wav',8,2,t=>.25+.9*Math.sin(2*Math.PI*440*t));
const legacy={id:randomUUID(),title:'Legacy DC fixture',artist:'Synthetic',filename:'legacy.wav',source:'local',duration:8,trim_start:0,trim_end:8};
const legacyBytes=Buffer.concat([await readFile(biased),Buffer.from([74,85,78,75,0,0,0,0])]);
legacyBytes.writeUInt32LE(legacyBytes.length-8,4);
await writeFile(path.join(data,'sources',legacy.id),legacyBytes);
const legacyHash=createHash('sha256').update(legacyBytes).digest('hex');
await writeFile(path.join(data,'library.json'),JSON.stringify({tracks:[legacy],collections:[]}));
const before=createHash('sha256').update(await readFile(biased)).digest('hex');
const base='http://127.0.0.1:3221';
const server=spawn(path.join(root,'target','debug',process.platform==='win32'?'cassette-studio.exe':'cassette-studio'),[],{cwd:root,windowsHide:true,env:{...process.env,CASSETTE_DATA:data,CASSETTE_PORT:'3221'},stdio:'ignore'});
const off={side_seconds:8,gap:0,fade:1,normalize:false,remove_dc:false,target_lufs:-18,peak_db:-1};
const on={...off,remove_dc:true};
async function api(route,body){const r=await fetch(base+route,body===undefined?{}:{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)});const j=await r.json();assert.ok(r.ok,JSON.stringify(j));return j;}
async function upload(file,s=off){const form=new FormData();form.append('settings',JSON.stringify(s));form.append('files',new Blob([await readFile(file)]),path.basename(file));const r=await fetch(base+'/api/import',{method:'POST',body:form});const j=await r.json();assert.ok(r.ok,JSON.stringify(j));assert.deepEqual(j.errors,[]);return j.tracks[0];}
async function pcm(url){const b=Buffer.from(await(await fetch(base+url)).arrayBuffer());for(let i=12;i+8<=b.length;){const n=b.readUInt32LE(i+4);if(b.toString('ascii',i,i+4)==='data')return b.subarray(i+8,i+8+n);i+=8+n+(n%2);}throw Error('Missing WAV data');}
function mean(b,c){let sum=0;for(let i=c*2;i<b.length;i+=4)sum+=b.readInt16LE(i)/32768;return sum/(b.length/4);}
function equalAudio(a,b,tolerance=2){assert.equal(a.length,b.length);let max=0;for(let i=0;i<a.length;i+=2)max=Math.max(max,Math.abs(a.readInt16LE(i)-b.readInt16LE(i)));assert.ok(max<=tolerance,`PCM difference ${max}`);}
const near=(a,b,e=2e-5)=>assert.ok(Math.abs(a-b)<e,`${a} != ${b}`);
const entry=(id,offset=0,duration=8,fade_in=0,fade_out=0)=>({track_id:id,offset,duration,fade_in,fade_out,continuation:offset>0});
try{
 for(let i=0;i<100;i++){try{await api('/api/library');break;}catch{if(server.exitCode!==null)throw Error('Server stopped');await new Promise(r=>setTimeout(r,100));if(i===99)throw Error('Startup timeout');}}
 const imported=await upload(biased),reference=await upload(clean);
 near(imported.dc.offset[0],.12);near(imported.dc.offset[1],-.07);
 const raw=await api(`/api/preview/${imported.id}`,off),corrected=await api(`/api/preview/${imported.id}`,on);
 assert.notEqual(raw.url,corrected.url);near(mean(await pcm(raw.url),0),.12);near(mean(await pcm(raw.url),1),-.07);
 const correctedPcm=await pcm(corrected.url);near(mean(correctedPcm,0),0);near(mean(correctedPcm,1),0);
 const rendered=await api('/api/render',{sides:[[entry(imported.id)],[]],settings:on});equalAudio(await pcm(rendered.urls[0]),correctedPcm);
 const cleanOff=await api(`/api/preview/${reference.id}`,off),cleanOn=await api(`/api/preview/${reference.id}`,on);equalAudio(await pcm(cleanOff.url),await pcm(cleanOn.url));
 for(const [offset,duration]of [[0,4],[4,4]]){
  const a=await api('/api/render',{sides:[[entry(imported.id,offset,duration,1,1)],[]],settings:{...on,side_seconds:4}});
  const b=await api('/api/render',{sides:[[entry(reference.id,offset,duration,1,1)],[]],settings:{...off,side_seconds:4}});
  equalAudio(await pcm(a.urls[0]),await pcm(b.urls[0]));
 }
 const lazy=await api(`/api/preview/${legacy.id}`,on);near(mean(await pcm(lazy.url),0),0);assert.ok(lazy.track.dc);
 for(const file of [edge,zeroEdge]){const t=await upload(file,on);near(t.trim_start,1,.03);near(t.trim_end,5,.03);near(t.dc.offset[0],.12);}
 const m=await upload(mono);near(m.dc.offset[0],m.dc.offset[1]);const mp=await api(`/api/preview/${m.id}`,on);near(mean(await pcm(mp.url),0),0);near(mean(await pcm(mp.url),1),0);
 const empty=await upload(silent,on);assert.equal(empty.trim_start,empty.trim_end);assert.deepEqual(empty.dc.offset,[0,0]);
 const normalized=await api(`/api/preview/${imported.id}`,{...on,normalize:true});
 const output=path.join(dir,'normalized.wav');await writeFile(output,Buffer.from(await(await fetch(base+normalized.url)).arrayBuffer()));
 const measured=spawnSync(ff,['-hide_banner','-nostats','-i',output,'-af','loudnorm=I=-18:TP=-1:LRA=50:print_format=json','-f','null','-'],{encoding:'utf8',windowsHide:true});assert.equal(measured.status,0,measured.stderr);
 const result=JSON.parse(measured.stderr.slice(measured.stderr.lastIndexOf('{'),measured.stderr.lastIndexOf('}')+1));near(+result.input_i,-18,.2);
 const h=await upload(hot),hp=await api(`/api/preview/${h.id}`,{...on,peak_db:-6});const hotPcm=await pcm(hp.url);let peak=0;for(let i=0;i<hotPcm.length;i+=2)peak=Math.max(peak,Math.abs(hotPcm.readInt16LE(i)/32768));assert.ok(peak<=10**(-6/20));
 const tape={id:randomUUID(),name:'DC test',kind:'cassette',tracks:[],sides:[[entry(imported.id)],[]],settings:on};await api('/api/collections',tape);
 const saved=JSON.parse(await readFile(path.join(data,'library.json'),'utf8'));assert.equal(saved.collections[0].settings.remove_dc,true);assert.ok(saved.tracks.find(t=>t.id===legacy.id).dc);
 assert.equal(createHash('sha256').update(await readFile(biased)).digest('hex'),before);assert.equal(createHash('sha256').update(await readFile(path.join(data,'sources',legacy.id))).digest('hex'),legacyHash);
 await writeFile(path.join(dir,'result.json'),JSON.stringify({passed:true,dc:imported.dc,normalized:result,peak,checks:['stereo DC','mono','silence','DC and digital padding','OFF preservation','preview/export agreement','fades and A/B continuation','legacy analysis','corrected loudness','peak protection','cache isolation','settings persistence','unchanged sources']},null,2));
 console.log(`PASS: DC correction, channel independence, preview/WAV agreement, fades, continuation, trimming, normalization, peak protection, legacy data, persistence and untouched sources.\n${dir}`);
}finally{server.kill();}
