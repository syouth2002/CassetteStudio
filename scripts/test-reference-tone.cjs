const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const elements=new Map();
const element=id=>{if(!elements.has(id))elements.set(id,{value:id==='tone-frequency'?'400':'-12',addEventListener(){}});return elements.get(id);};
let paused=0,revoked=0,timer=null,resolvePlay;
const context={ArrayBuffer,DataView,Math,Number,Blob,$:element,csText:s=>s,toast:()=>{},
 audio:{pause(){paused++;},load(){},removeAttribute(){},play:()=>new Promise(r=>resolvePlay=r)},
 URL:{createObjectURL:()=> 'blob:test',revokeObjectURL:()=>revoked++},
 setTimeout:fn=>{timer=fn;return 1;},clearTimeout:()=>{timer=null;},addEventListener(){}};
context.window=context;context.stop=()=>context.csStopReferenceTone();vm.createContext(context);
vm.runInContext(fs.readFileSync('web/reference-tone.js','utf8'),context);
for(const frequency of [400,1000])for(const level of [-30,-18,-12,-3]){
 const view=new DataView(vm.runInContext(`referenceToneWav(${frequency},${level})`,context));
 assert.equal(view.getUint32(24,true),48000);assert.equal(view.getUint16(22,true),2);
 let sum=0,squares=0,peak=0,crossings=0,previous=0;
 for(let i=0;i<48000;i++){
  const sample=view.getInt16(44+i*4,true);assert.equal(sample,view.getInt16(46+i*4,true));
  sum+=sample;squares+=sample*sample;peak=Math.max(peak,Math.abs(sample));
  if(previous<=0&&sample>0)crossings++;previous=sample;
 }
 assert.equal(crossings,frequency);assert.ok(Math.abs(sum/48000)<.01);
 assert.ok(Math.abs(20*Math.log10(peak/32767)-level)<.02);
 assert.ok(Math.abs(20*Math.log10(Math.sqrt(squares/48000)/32767)-(level-3.0103))<.02);
}
(async()=>{
 let play=element('tone-start').onclick();assert.equal(context.audio.loop,true);
 context.stop();resolvePlay();await play;assert.equal(timer,null,'Stopped pending play cannot arm a timer');
 play=element('tone-start').onclick();resolvePlay();await play;assert.ok(timer);
 timer();assert.equal(context.audio.loop,false);assert.equal(element('tone-stop').disabled,true);
 assert.ok(paused>=2&&revoked>=2);
 console.log('PASS: stereo tone frequency, peak/RMS level, DC, stop during play, automatic stop');
})();
