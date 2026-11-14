import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {mkdir,writeFile,readFile} from 'node:fs/promises';
import path from 'node:path';
import {randomUUID,createHash} from 'node:crypto';
import {engineUrl} from './engine-test-url.mjs';
const root=process.cwd(),dir=path.join(root,'work',`properties-${Date.now()}`),data=path.join(dir,'data'),ext=process.platform==='win32'?'.exe':'';
await mkdir(path.join(data,'sources'),{recursive:true});
const cases=[['wav','pcm_s16le',48000,16,'WAV'],['flac','flac',96000,24,'FLAC'],['mp3','libmp3lame',44100,null,'MP3'],['ipod','aac',48000,null,'M4A'],['mp4','aac',48000,null,'MP4'],['mp4','aac',48000,null,'MP4','video'],['ipod','aac',48000,null,'M4A','cover']];
const cover=path.join(dir,'cover.jpg');
const coverResult=spawnSync(path.join(root,'tools',`ffmpeg${ext}`),['-v','error','-f','lavfi','-i','color=c=black:s=16x16','-frames:v','1','-update','1',cover],{windowsHide:true,encoding:'utf8'});assert.equal(coverResult.status,0,coverResult.stderr);
const tracks=[];
for(const[format,codec,rate,bits,,mode]of cases){
 const id=randomUUID(),file=path.join(data,'sources',id);
 const args=['-v','error','-f','lavfi','-i',`sine=frequency=440:duration=1:sample_rate=${rate}`];
 if(mode==='video')args.push('-f','lavfi','-i','color=c=black:s=16x16:r=1:d=1','-map','1:v','-map','0:a','-c:v','mpeg4');
 if(mode==='cover')args.push('-i',cover,'-map','0:a','-map','1:v','-c:v','copy','-disposition:v:0','attached_pic');
 args.push('-ac','2','-c:a',codec);
 if(format==='flac')args.push('-sample_fmt','s32','-bits_per_raw_sample','24');
 args.push('-metadata','album=Metadata fixture','-metadata','artist=Properties test','-f',format,file);
 const out=spawnSync(path.join(root,'tools',`ffmpeg${ext}`),args,{windowsHide:true,encoding:'utf8'});assert.equal(out.status,0,out.stderr);
 tracks.push({id,title:format,artist:'Properties test',filename:`test.${format}`,source:'local',duration:1,trim_start:0,trim_end:1});
}
await writeFile(path.join(data,'library.json'),JSON.stringify({tracks,collections:[]}));
const hash=b=>createHash('sha256').update(b).digest('hex');
const server=spawn(process.env.CASSETTE_TEST_ENGINE || path.join(root,'target','debug',`cassette-studio${ext}`),[],{cwd:root,windowsHide:true,env:{...process.env,CASSETTE_DATA:data,CASSETTE_PORT:'0'}});
try{
 const base=await engineUrl(server),before=await(await fetch(base+'/api/library')).json();
 for(let i=0;i<cases.length;i++){
  const [format,codec,rate,bits,container,mode]=cases[i],t=tracks[i],file=path.join(data,'sources',t.id),original=hash(await readFile(file));
  const response=await fetch(`${base}/api/tracks/${t.id}/properties`);assert.equal(response.status,200);const m=await response.json();
  assert.equal(m.sample_rate,rate);assert.equal(m.channels,2);assert.equal(m.bits,bits);assert.equal(m.bit_depth_applicable,!['aac','libmp3lame'].includes(codec));assert.equal(m.container_name,container);assert.equal(m.has_video,mode==='video');assert.equal(m.has_cover_art,mode==='cover');assert.equal(m.tags.album,'Metadata fixture');assert.equal(m.track.id,t.id);assert.equal(m.track.loudness,null);assert.ok(m.size>0);assert.ok(m.stored_path.endsWith(t.id));assert.equal(hash(await readFile(file)),original);
 }
 assert.deepEqual(await(await fetch(base+'/api/library')).json(),before,'Opening properties must not alter the library or run analysis');
 assert.equal((await fetch(base+'/api/tracks/missing/properties')).status,400);
 console.log('PASS: WAV/FLAC/MP3, audio-only M4A and MP4, actual video, cover art excluded from video, friendly containers, no analysis or source changes');console.log(dir);
}finally{server.kill();}
