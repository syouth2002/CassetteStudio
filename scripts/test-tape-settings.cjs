const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const app=fs.readFileSync('web/js/settings.js','utf8'),html=fs.readFileSync('web/index.html','utf8');
const controls=new Map();
const $=id=>{if(!controls.has(id))controls.set(id,{value:'',addEventListener(){}});return controls.get(id);};
let settings={side_seconds:1815,length_correction:15,gap:2,fade:5,normalize:false};
const context={$: $,config:()=>settings};vm.createContext(context);
vm.runInContext(app,context);
for(const [total,oldCorrection,minutes,seconds,preset] of [
 [1815,15,30,15,'custom'],[2205,-15,36,45,'custom'],[1800,undefined,30,0,60],
 [1800.5,.5,30,.5,'custom'],[59.5,0,0,59.5,'custom'],[7200,0,120,0,'custom'],
]){
 settings={...settings,side_seconds:total,length_correction:oldCorrection};
 context.fillSettings();
 assert.equal($('tape-preset').value,preset);
 assert.equal($('side-minutes').value,minutes);
 assert.equal($('side-seconds').value,seconds);
 const result=context.readSettings();
 assert.equal(result.side_seconds,total);
 assert.ok(!Object.hasOwn(result,'length_correction'));
}
vm.runInContext(fs.readFileSync('web/js/buttons/settings.js','utf8'),context);
context.bindSettingsControls();
for(const n of [46,60,74,80,90,120]){
 $('side-seconds').value=15;
 $('tape-preset').onchange({target:{value:String(n)}});
 assert.equal($('side-minutes').value,n/2);
 assert.equal($('side-seconds').value,0);
 assert.equal(context.readSettings().side_seconds,n*30);
}
$('side-minutes').value=31;$('side-seconds').value=12.5;
$('tape-preset').onchange({target:{value:'custom'}});
assert.equal(context.readSettings().side_seconds,1872.5);
for(const id of ['side-minutes','side-seconds']){
 $('tape-preset').value='60';$(id).oninput();assert.equal($('tape-preset').value,'custom');
}
assert.ok(!html.includes('id="length-correction"'));
assert.ok(!html.includes('id="rewind"'));
assert.ok(!Object.hasOwn(context.readSettings(),'rewind'));
assert.match(html,/id="side-minutes"[^>]*step="1"/);
assert.match(html,/id="side-seconds"[^>]*step="any"/);
for(const n of [46,60,74,80,90,120])assert.ok(html.includes(`<option value="${n}"`));
for(const id of ['gap','fade','entry-in','entry-out'])assert.match(html,new RegExp('id="'+id+'"[^>]*step="0.5"'));
assert.ok(!html.includes('id="view-cards"'));


settings={...settings,lead_in:2.5,wav_sample_rate:48000};context.fillSettings();
assert.equal($('lead-in').value,2.5);assert.equal($('wav-sample-rate').value,'48000');
assert.equal(context.readSettings().lead_in,2.5);assert.equal(context.readSettings().wav_sample_rate,48000);
$('wav-sample-rate').value='';assert.equal(context.readSettings().wav_sample_rate,null);
const dialog=html.slice(html.indexOf('<dialog id="settings-dialog">'),html.indexOf('<dialog id="url-dialog">'));
assert.match(dialog,/id="lead-in"[^>]*step="0.5"/);assert.match(dialog,/id="wav-sample-rate"/);
assert.match(dialog,/DAT/);

console.log('PASS: minute/second capacity, preset reset, manual edits, legacy corrected lengths, removed correction, DAT settings');
