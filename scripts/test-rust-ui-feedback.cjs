// Run with the disposable library created by test-rust-ui-browser.py.
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {chromium}=require(process.env.CASSETTE_PLAYWRIGHT_MODULE||'playwright');
(async()=>{
 const browser=await chromium.launch({headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});
 let releaseImport=()=>{},releaseSave=()=>{};
 try {
  const context=await browser.newContext({viewport:{width:1280,height:850},locale:'ja-JP'}),page=await context.newPage();
  const errors=[];page.on('pageerror',e=>errors.push(e.message));
  if(process.env.CASSETTE_UI_ASSET_DIR)for(const name of ['cassette_ui.js','cassette_ui_bg.wasm'])await page.route('**/ui/'+name,r=>r.fulfill({body:fs.readFileSync(path.join(process.env.CASSETTE_UI_ASSET_DIR,name)),contentType:name.endsWith('.wasm')?'application/wasm':'text/javascript'}));
  const url=process.argv[2],out=process.argv[3],api=p=>new URL(p,url).href;
  const library=await(await page.request.get(api('/api/library'))).json(),tape=library.collections.find(c=>c.kind==='cassette');
  tape.sides=[[],[]];assert.equal((await page.request.post(api('/api/collections'),{data:tape})).status(),200);
  let importSeen,saveSeen;
  const importing=new Promise(r=>importSeen=r),saving=new Promise(r=>saveSeen=r),importGate=new Promise(r=>releaseImport=r),saveGate=new Promise(r=>releaseSave=r);
  let imports=0,failNextSave=false;
  await page.route('**/api/import',async route=>{
   if(imports++===0){importSeen();await importGate;}
   await route.continue();
  });
  await page.route('**/api/collections',async route=>{
   if(failNextSave&&route.request().method()==='POST'){failNextSave=false;await route.fulfill({status:500,contentType:'application/json',body:JSON.stringify({error:'Synthetic cassette-save failure'})});return;}
   if(route.request().method()==='POST'){saveSeen();await saveGate;}
   await route.continue();
  });
  await page.goto(url);await page.waitForFunction(()=>!document.getElementById('loading'));await page.waitForTimeout(1500);
  await page.mouse.click(90,360);await page.waitForTimeout(250);
  const wav=Buffer.alloc(44+44100*2);wav.write('RIFF');wav.writeUInt32LE(wav.length-8,4);wav.write('WAVEfmt ',8);wav.writeUInt32LE(16,16);wav.writeUInt16LE(1,20);wav.writeUInt16LE(1,22);wav.writeUInt32LE(44100,24);wav.writeUInt32LE(88200,28);wav.writeUInt16LE(2,32);wav.writeUInt16LE(16,34);wav.write('data',36);wav.writeUInt32LE(wav.length-44,40);for(let i=0;i<44100;i++)wav.writeInt16LE(Math.round(2500*Math.sin(2*Math.PI*777*i/44100)),44+i*2);
  async function choose(name,buffer){const chooser=page.waitForEvent('filechooser');await page.mouse.click(1000,46);await(await chooser).setFiles({name,mimeType:'audio/wav',buffer});}
  const shot=name=>page.screenshot({path:path.join(out,name+'.png')});
  await choose('進捗テスト.wav',wav);await importing;await page.waitForTimeout(350);await shot('analyzing');
  assert.equal((await(await page.request.get(api('/api/library'))).json()).tracks.length,library.tracks.length,'Pending imports must not be reported as added');
  releaseImport();await saving;await page.waitForTimeout(350);await shot('applying');
  let current=await(await page.request.get(api('/api/library'))).json();assert.equal(current.tracks.length,library.tracks.length+1);assert.equal(current.collections.find(c=>c.id===tape.id).sides[0].length,0,'Progress must remain until the mix is saved');
  const saved=page.waitForResponse(r=>r.url().endsWith('/api/collections')&&r.request().method()==='POST');releaseSave();assert.equal((await saved).status(),200);
  await page.waitForTimeout(500);await shot('completed');
  current=await(await page.request.get(api('/api/library'))).json();assert.equal(current.collections.find(c=>c.id===tape.id).sides[0].length,1);
  await page.keyboard.press('Escape');await page.waitForTimeout(1000);
  const failed=page.waitForResponse(r=>r.url().endsWith('/api/import'));await choose('読み込めない音源.wav',Buffer.from('not an audio file'));const failedResponse=await failed;assert.equal(failedResponse.status(),200);assert.equal((await failedResponse.json()).errors.length,1);
  await page.waitForTimeout(400);await shot('error');
  current=await(await page.request.get(api('/api/library'))).json();assert.equal(current.tracks.length,library.tracks.length+1);assert.equal(current.collections.find(c=>c.id===tape.id).sides[0].length,1);
  await page.keyboard.press('Escape');await page.waitForTimeout(800);
  const other=Buffer.from(wav);other.writeInt16LE(1,44);failNextSave=true;
  const importedAgain=page.waitForResponse(r=>r.url().endsWith('/api/import')),saveFailed=page.waitForResponse(r=>r.url().endsWith('/api/collections')&&r.request().method()==='POST');
  await choose('リスト保存エラーテスト.wav',other);assert.equal((await importedAgain).status(),200);assert.equal((await saveFailed).status(),500);
  await page.waitForTimeout(400);await shot('save-error');
  current=await(await page.request.get(api('/api/library'))).json();assert.equal(current.tracks.length,library.tracks.length+2,'Audio must remain in the library after cassette-save failure');assert.equal(current.collections.find(c=>c.id===tape.id).sides[0].length,1);
  assert.deepEqual(errors,[]);
  console.log('PASS: file-picker analysis progress, pending cassette-save progress, completion/import-error/save-error results and preserved library/mix data');
 }finally{releaseImport();releaseSave();await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
