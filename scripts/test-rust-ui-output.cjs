// Uses only the synthetic library created by test-rust-ui-browser.py.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');const {chromium}=require(process.env.CASSETTE_PLAYWRIGHT_MODULE);
(async()=>{const b=await chromium.launch({headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});try{const p=await b.newPage({viewport:{width:1280,height:850},locale:'ja-JP',acceptDownloads:true}),base=process.argv[2],out=process.argv[3];for(const name of ['cassette_ui.js','cassette_ui_bg.wasm'])await p.route('**/ui/'+name,r=>r.fulfill({body:fs.readFileSync(path.join(process.env.CASSETTE_UI_ASSET_DIR,name)),contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript'}));
const lib=await(await p.request.get(base+'api/library')).json();assert.equal(lib.collections.find(c=>c.kind==='cassette').id,'00000000-0000-4000-8000-000000000019');
let renders=0;p.on('request',r=>{if(r.method()==='POST'&&r.url().endsWith('/api/render'))renders++});await p.route('**/api/render',async r=>{const response=await r.fetch();await new Promise(resolve=>setTimeout(resolve,900));await r.fulfill({response})});
await p.goto(base);await p.waitForFunction(()=>!document.getElementById('loading'));await p.waitForTimeout(900);
const click=async(x,y)=>{await p.mouse.move(x,y);await p.waitForTimeout(120);await p.mouse.click(x,y,{delay:80});await p.waitForTimeout(300)},shot=name=>p.screenshot({path:path.join(out,name+'.png')});
await click(1180,118);await shot('automatic-name');await p.keyboard.press('Escape');await p.waitForTimeout(150);await click(1030,118);await shot('ordered-name');await p.keyboard.press('Escape');await p.waitForTimeout(150);await click(90,330);
let response=p.waitForResponse(r=>r.url().endsWith('/api/render'));await click(1018,266);await shot('export-working');await response;await p.waitForTimeout(450);await shot('export-ready');assert.equal(renders,1);
for(const [side,y] of [['A',480],['B',507]]) {
await p.waitForTimeout(500);await shot('before-download-'+side);await click(400,y);await p.locator('#rfd-output').waitFor({state:'visible',timeout:5000}).catch(async e=>{await shot('download-failed-'+side);throw e});const [download]=await Promise.all([p.waitForEvent('download'),p.locator('#rfd-output').click()]);assert.equal(download.suggestedFilename(),'Evening Sketches-'+side+'.wav');const file=path.join(out,'export-'+side+'.wav');await download.saveAs(file);assert.equal(fs.readFileSync(file).subarray(0,4).toString(),'RIFF');await p.waitForTimeout(250);
}assert.equal(renders,1,'Downloading must reuse generated files');
await click(942,350);await p.waitForTimeout(500);
response=p.waitForResponse(r=>r.url().endsWith('/api/render'));await click(1180,266);await shot('deck-working');await response;await p.waitForTimeout(650);await shot('deck-ready');assert.equal(renders,2);
for(const [side,y] of [['A',431],['B',458]]) {
await click(400,y);await p.locator('#rfd-output').waitFor({state:'visible'});const [download]=await Promise.all([p.waitForEvent('download'),p.locator('#rfd-output').click()]);assert.equal(download.suggestedFilename(),'Evening Sketches-'+side+'.wav');await download.saveAs(path.join(out,'deck-'+side+'.wav'));await p.waitForTimeout(500);
}assert.equal(renders,2,'Deck downloads must not render again');
console.log('PASS: export and deck dialogs show generation and per-side links; both export WAV files saved without regeneration');}finally{await b.close()}})().catch(e=>{console.error(e);process.exitCode=1});
