// Disposable synthetic engine fixture only. Exercises the actual shared Rust UI.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');const {chromium}=require(process.env.CASSETTE_PLAYWRIGHT_MODULE);
(async()=>{const b=await chromium.launch({headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader','--autoplay-policy=no-user-gesture-required']});try{
const p=await b.newPage({viewport:{width:1280,height:850},locale:'ja-JP'}),base=process.argv[2],out=process.argv[3];
await p.addInitScript(()=>{window.audioPlays=0;window.audioSeeks=[];const proto=HTMLMediaElement.prototype,play=proto.play,seek=Object.getOwnPropertyDescriptor(proto,'currentTime');proto.play=function(...args){window.currentAudio=this;window.audioPlays++;return play.apply(this,args)};Object.defineProperty(proto,'currentTime',{...seek,set(v){window.audioSeeks.push(v);seek.set.call(this,v)}})});
for(const name of ['cassette_ui.js','cassette_ui_bg.wasm'])await p.route('**/ui/'+name,r=>r.fulfill({body:fs.readFileSync(path.join(process.env.CASSETTE_UI_ASSET_DIR,name)),contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript'}));
const library=await(await p.request.get(base+'api/library')).json();assert.equal(library.collections.find(c=>c.kind==='cassette').id,'00000000-0000-4000-8000-000000000019');
const requests=[],errors=[];p.on('pageerror',e=>errors.push(e.message));p.on('request',r=>{if(r.method()==='POST'&&r.url().includes('/api/preview/'))requests.push(r.url().split('/').pop())});
await p.goto(base);await p.waitForFunction(()=>!document.getElementById('loading'));await p.waitForTimeout(900);
const id=n=>library.tracks[n-1].id;const click=async(x,y)=>{await p.mouse.click(x,y,{delay:65});await p.waitForTimeout(180)};
async function actionTrack(n,action){const count=requests.length,plays=await p.evaluate(()=>window.audioPlays);await action();for(let i=0;i<100;i++){if(requests.length>count&&requests.at(-1)===id(n)&&await p.evaluate(v=>window.audioPlays>v,plays))return;await p.waitForTimeout(100)}await p.screenshot({path:path.join(out,'failure-'+n+'.png')});console.log(requests);await p.screenshot({path:path.join(out,'failure-'+n+'.png')});console.log(requests);assert.equal(requests.at(-1),id(n));throw Error('Audio did not start')}
await click(450,370);await actionTrack(3,()=>click(670,798));
await actionTrack(4,()=>click(752,798));await actionTrack(3,()=>click(627,798));
await p.evaluate(()=>window.currentAudio.currentTime=5);await actionTrack(2,()=>click(627,798));
await click(711,798);await actionTrack(5,()=>p.mouse.dblclick(450,478,{delay:40}));
await click(90,330);await actionTrack(6,()=>click(752,798));await actionTrack(5,()=>click(627,798));
await click(711,798);await click(100,230);await p.mouse.click(450,318,{button:'right'});await p.waitForTimeout(200);await p.screenshot({path:path.join(out,'context-play.png')});
await actionTrack(2,()=>click(490,330));await actionTrack(3,()=>click(752,798));
// Selection playback retains the selected subset when more than one row is checked.
await click(711,798);await click(450,318);await click(281,422);await actionTrack(2,()=>click(670,798));await actionTrack(4,()=>click(752,798));
await click(711,798);await click(90,330);
// Delay render completion to exercise navigation while audio is being prepared.
await p.route('**/api/render',async r=>{const response=await r.fetch();await new Promise(resolve=>setTimeout(resolve,650));await r.fulfill({response})});
const renders=p.waitForRequest(r=>r.method()==='POST'&&r.url().endsWith('/api/render'));const before=await p.evaluate(()=>window.audioPlays);await click(670,798);await renders;await click(752,798);await click(752,798);
await p.waitForFunction(n=>window.audioPlays>n,before);await p.waitForTimeout(200);assert.ok(await p.evaluate(()=>window.audioSeeks.some(v=>Math.abs(v-18)<.01)),'Pending side next/next must seek to track 3');
await p.evaluate(()=>window.currentAudio.currentTime=23);await click(627,798);assert.ok(await p.evaluate(()=>Math.abs(window.currentAudio.currentTime-9)<1),'Previous after 3 seconds must seek to previous side track');
await p.screenshot({path:path.join(out,'side-playback.png')});assert.deepEqual(errors,[]);
await click(711,798);await click(65,122);await click(450,262);
await p.route('**/api/preview/*',async r=>{const response=await r.fetch();await new Promise(resolve=>setTimeout(resolve,350));await r.fulfill({response})});
const rapidBefore=await p.evaluate(()=>window.audioPlays);
const preparing=p.waitForRequest(r=>r.method()==='POST'&&r.url().includes('/api/preview/'));await click(670,798);await preparing;await click(752,798);await click(752,798);
for(let i=0;i<100;i++){if(requests.at(-1)===id(3)&&await p.evaluate(v=>window.audioPlays>v,rapidBefore))break;await p.waitForTimeout(100);}
assert.equal(requests.at(-1),id(3));assert.equal(await p.evaluate(()=>window.audioPlays),rapidBefore+1,'Stale earlier responses must not play');
const stopBefore=await p.evaluate(()=>window.audioPlays);await click(752,798);await click(711,798);await p.waitForTimeout(1500);assert.equal(await p.evaluate(()=>window.audioPlays),stopBefore,'Stop must reject pending audio');
console.log('PASS: rapid skips and stop during preparation; selected playback next/previous, previous after 3s, double-click, context playback, browse isolation, selected subset, pending side seeks and whole-side previous');
}finally{await b.close()}})().catch(e=>{console.error(e);process.exitCode=1});
