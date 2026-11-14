// Run with the disposable fixture in test-rust-ui-browser.py.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {chromium}=require(process.env.CASSETTE_PLAYWRIGHT_MODULE);
(async()=>{const browser=await chromium.launch({headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});try{
const context=await browser.newContext({viewport:{width:1280,height:850},locale:'ja-JP'}),p=await context.newPage();
const errors=[];p.on('pageerror',e=>errors.push(e.message));
if(process.env.CASSETTE_UI_ASSET_DIR)for(const name of ['cassette_ui.js','cassette_ui_bg.wasm'])await p.route('**/ui/'+name,r=>r.fulfill({body:fs.readFileSync(path.join(process.env.CASSETTE_UI_ASSET_DIR,name)),contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript'}));
const url=process.argv[2], library=await(await p.request.get(url+'api/library')).json(),tape=library.collections.find(c=>c.kind==='cassette');
tape.sides=[[],[]];assert.equal((await p.request.post(url+'api/collections',{data:tape})).status(),200);
await p.goto(url);await p.waitForFunction(()=>!document.getElementById('loading'));await p.waitForTimeout(1200);
await p.mouse.click(90,360);await p.waitForTimeout(350);await p.mouse.click(965,149);await p.waitForTimeout(350);
await p.screenshot({path:path.join(process.argv[3],'choose-tracks.png')});
for(let i=0;i<6;i++)await p.mouse.click(307,354+i*26);
const saved=p.waitForResponse(r=>r.url().endsWith('/api/collections')&&r.request().method()==='POST');
await p.mouse.click(322,540);assert.equal((await saved).status(),200);await p.waitForTimeout(500);
const after=await(await p.request.get(url+'api/library')).json(),recorded=after.collections.find(c=>c.id===tape.id);
assert.deepEqual(recorded.sides[0].map(e=>e.track_id),library.tracks.slice(0,3).map(t=>t.id));
assert.deepEqual(recorded.sides[1].map(e=>e.track_id),library.tracks.slice(3,6).map(t=>t.id));
assert.equal(after.tracks.length,library.tracks.length,'Appending must retain every library source');
await p.mouse.click(370,356);await p.waitForTimeout(300);await p.screenshot({path:path.join(process.argv[3],'side-b.png')});
assert.deepEqual(errors,[]);
console.log('PASS: adding six selected tracks to a blank tape places the first three on A and the next three on B, in order, without removing library sources');
}finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
