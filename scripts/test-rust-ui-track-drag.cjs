const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {chromium}=require(process.env.CASSETTE_PLAYWRIGHT_MODULE);
(async()=>{const b=await chromium.launch({headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});try{
const p=await b.newPage({viewport:{width:1280,height:850},locale:'ja-JP'}),base=process.argv[2],out=process.argv[3];
for(const name of ['cassette_ui.js','cassette_ui_bg.wasm'])await p.route('**/ui/'+name,r=>r.fulfill({body:fs.readFileSync(path.join(process.env.CASSETTE_UI_ASSET_DIR,name)),contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript'}));
const lib=async()=>await(await p.request.get(base+'api/library')).json();
assert.equal((await lib()).collections.find(c=>c.kind==='cassette').id,'00000000-0000-4000-8000-000000000019');
await p.goto(base);await p.waitForFunction(()=>!document.getElementById('loading'));await p.waitForTimeout(1200);
await p.mouse.click(90,360);await p.waitForTimeout(400);
await p.screenshot({path:path.join(out,'before.png')});
const tape=async()=>(await lib()).collections.find(c=>c.kind==='cassette');
const original=(await tape()).sides[0].map(e=>e.track_id), originalB=(await tape()).sides[1];
async function drag(x,y,tx,ty,name){await p.mouse.move(x,y);await p.waitForTimeout(80);await p.mouse.down();await p.waitForTimeout(80);await p.mouse.move(tx,ty,{steps:12});await p.waitForTimeout(250);if(name)await p.screenshot({path:path.join(out,name+'.png')});}
async function waitOrder(expected){for(let i=0;i<60;i++){if(JSON.stringify((await tape()).sides[0].map(e=>e.track_id))===JSON.stringify(expected)){await p.waitForTimeout(400);return;}await p.waitForTimeout(100);}assert.deepEqual((await tape()).sides[0].map(e=>e.track_id),expected);}
// Drop over the status column, below the final row: full-width target and end insertion.
await drag(450,465,1050,650,'drag-to-end');await p.mouse.up();await waitOrder([original[1],original[2],original[0]]);
await drag(450,610,1050,441,'drag-to-start');await p.mouse.up();await waitOrder(original);
await drag(450,465,1050,650,'cancel');await p.keyboard.press('Escape');await p.mouse.up();await p.waitForTimeout(500);await waitOrder(original);
await drag(450,465,100,200);await p.mouse.up();await p.waitForTimeout(500);await waitOrder(original);
await drag(450,465,1000,490);await p.mouse.up();await p.waitForTimeout(500);await waitOrder(original);
// Playlist uses the same insertion feedback, without modifying the global library order.
await p.mouse.click(100,260);await p.waitForTimeout(350);await p.screenshot({path:path.join(out,'playlist.png')});
const playlist=async()=>(await lib()).collections.find(c=>c.kind==='playlist');const before=(await playlist()).tracks;
await drag(450,310,1050,500,'playlist-drag');await p.mouse.up();
const expected=[before[1],before[2],before[0],...before.slice(3)];
for(let i=0;i<60;i++){if(JSON.stringify((await playlist()).tracks)===JSON.stringify(expected))break;await p.waitForTimeout(100);}
assert.deepEqual((await playlist()).tracks,expected);assert.deepEqual((await lib()).tracks.map(t=>t.id),before,'Playlist reorder must not reorder library');
assert.deepEqual((await tape()).sides[1],originalB);
// Small viewport forces scrolling while holding a track; no wheel input.
await p.setViewportSize({width:1280,height:650});await p.waitForTimeout(400);
const scrollBefore=(await playlist()).tracks;
await drag(450,310,1050,516,'auto-scroll-start');await p.waitForTimeout(1600);
await p.screenshot({path:path.join(out,'auto-scroll-bottom.png')});
await p.mouse.up();await p.waitForTimeout(700);
const scrollAfter=(await playlist()).tracks;
assert.ok(scrollAfter.indexOf(scrollBefore[0])>=5,'Holding near bottom must reach offscreen rows');
assert.deepEqual([...scrollAfter].sort(),[...scrollBefore].sort(),'Auto-scroll move must preserve all tracks');
await drag(450,470,1050,195,'auto-scroll-up-start');await p.waitForTimeout(1600);
await p.screenshot({path:path.join(out,'auto-scroll-top.png')});
await p.mouse.move(1050,290,{steps:4});await p.waitForTimeout(100);await p.mouse.up();
for(let i=0;i<60;i++){if((await playlist()).tracks[0]===scrollBefore[0])break;await p.waitForTimeout(100);}
assert.equal((await playlist()).tracks[0],scrollBefore[0],'Upward auto-scroll must allow moving back to the first row');
console.log('PASS: automatic upward and downward scrolling and offscreen reorder; playlist insertion, global order isolation; cassette downward/end and upward/start insertion, full-width drop, Escape cancellation, outside cancellation, no-op boundary');
}finally{await b.close()}})().catch(e=>{console.error(e);process.exitCode=1});
