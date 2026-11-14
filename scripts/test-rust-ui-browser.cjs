// Run against the synthetic fixture created by test-rust-ui-browser.py.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { chromium } = require(process.env.CASSETTE_PLAYWRIGHT_MODULE || 'playwright');
(async () => {
  const base = process.argv[2], out = process.argv[3];
  fs.mkdirSync(out, { recursive: true });
  const browser = await chromium.launch({ headless: true, args: ['--use-angle=swiftshader', '--enable-unsafe-swiftshader', '--autoplay-policy=no-user-gesture-required'] });
  try {
    const context = await browser.newContext({ viewport: { width: 1280, height: 850 }, locale: 'ja-JP', acceptDownloads: true });
    await context.addInitScript(() => {
      window.audioActions=[];
      const proto=HTMLMediaElement.prototype, seek=Object.getOwnPropertyDescriptor(proto,'currentTime');
      Object.defineProperty(proto,'currentTime',{...seek,set(value){window.audioActions.push(['seek',value]);seek.set.call(this,value);}});
      for(const method of ['play','pause']){const original=proto[method];proto[method]=function(...args){window.audioActions.push([method]);return original.apply(this,args);};}
    });
    const page = await context.newPage(), errors = [];
    page.on('pageerror', e => errors.push(e.message));
    await page.goto(base);
    await page.waitForFunction(() => !document.getElementById('loading'));
    await page.waitForTimeout(1600);
    assert.equal(await page.locator('canvas').count(), 1);
    const shot = name => page.screenshot({ path: path.join(out, name + '.png') });
    await shot('library-ja');
    // Real browser file drops must attach to the selected playlist and reuse duplicates.
    await page.mouse.click(100,230); await page.waitForTimeout(300);
    const wav=Buffer.alloc(44+44100*2*2);wav.write('RIFF');wav.writeUInt32LE(wav.length-8,4);wav.write('WAVEfmt ',8);wav.writeUInt32LE(16,16);wav.writeUInt16LE(1,20);wav.writeUInt16LE(1,22);wav.writeUInt32LE(44100,24);wav.writeUInt32LE(88200,28);wav.writeUInt16LE(2,32);wav.writeUInt16LE(16,34);wav.write('data',36);wav.writeUInt32LE(wav.length-44,40);
    for(let i=0;i<44100*2;i++)wav.writeInt16LE(Math.round(1800*Math.sin(2*Math.PI*777*i/44100)),44+i*2);
    async function drop(){
      const transfer=await page.evaluateHandle(b64=>{const d=new DataTransfer();d.items.add(new File([Uint8Array.from(atob(b64),c=>c.charCodeAt(0))],'Rust drop.wav',{type:'audio/wav'}));return d;},wav.toString('base64'));
      await Promise.all([page.waitForResponse(r=>r.url().endsWith('/api/import')&&r.status()===200),page.locator('canvas').dispatchEvent('drop',{dataTransfer:transfer})]);await transfer.dispose();
      await page.waitForTimeout(1200);
    }
    await drop();
    let imported=await(await page.request.get(new URL('/api/library',base).href)).json();
    assert.equal(imported.tracks.length,8);assert.equal(imported.collections.find(c=>c.kind==='playlist').tracks.length,8);
    await drop();
    imported=await(await page.request.get(new URL('/api/library',base).href)).json();
    assert.equal(imported.tracks.length,8);assert.equal(imported.collections.find(c=>c.kind==='playlist').tracks.length,8);
    await page.mouse.click(90, 330); await page.waitForTimeout(500);
    await shot('cassette-ja');
    const cardId = '00000000-0000-4000-8000-000000000019';
    const original = await (await page.request.get(new URL('/api/library', base).href)).json();
    await page.mouse.click(890, 214); await page.waitForTimeout(1200);
    await shot('jcard-outside-ja');
    await page.mouse.click(775, 15); await page.waitForTimeout(1300);
    let card = await (await page.request.get(new URL('/api/jcards/' + cardId, base).href)).json();
    assert.equal(card.elements.length, 6, 'Adding a text box must autosave');
    // Move and resize the new text frame using actual pointer gestures.
    await page.mouse.move(300, 91); await page.mouse.down();
    await page.mouse.move(340, 131, { steps: 10 }); await page.mouse.up();
    await page.waitForTimeout(1200);
    card = await (await page.request.get(new URL('/api/jcards/' + cardId, base).href)).json();
    assert.equal(card.elements[5].x, 15); assert.equal(card.elements[5].y, 15);
    await page.mouse.move(418, 179); await page.mouse.down();
    await page.mouse.move(458, 199, { steps: 10 }); await page.mouse.up();
    await page.waitForTimeout(1200);
    card = await (await page.request.get(new URL('/api/jcards/' + cardId, base).href)).json();
    assert.equal(card.elements[5].w, 50); assert.equal(card.elements[5].h, 25);
    await page.mouse.click(1090, 46); await page.waitForTimeout(900);
    await page.mouse.click(435, 15); await page.waitForTimeout(500);
    await page.mouse.click(1235, 46); await page.waitForTimeout(1300);
    await shot('jcard-inside-ja');
    card = await (await page.request.get(new URL('/api/jcards/' + cardId, base).href)).json();
    assert.equal(card.elements.length, 5); assert.equal(card.backElements.length, 1);
    assert.equal(card.backElements[0].w, 50);
    await page.mouse.click(402,817);
    await page.locator('#rfd-output').waitFor({state:'visible'});
    const [pdfDownload]=await Promise.all([page.waitForEvent('download'),page.locator('#rfd-output').click()]);
    await pdfDownload.saveAs(path.join(out,'jcard.pdf'));
    assert.equal(await pdfDownload.failure(),null);
    assert.equal(fs.readFileSync(path.join(out,'jcard.pdf')).subarray(0,5).toString(),'%PDF-');
    await page.waitForTimeout(300);
    await page.mouse.click(320, 15); await page.waitForTimeout(500);
    await page.mouse.click(1070, 100); await page.waitForTimeout(300);
    await shot('settings-ja');
    await page.keyboard.press('Escape'); await page.waitForTimeout(300);
    await page.mouse.click(670,774);
    await page.waitForFunction(()=>window.audioActions.some(a=>a[0]==='play')).catch(async error=>{
      await shot('player-failed-ja');
      console.log('Audio actions:',await page.evaluate(()=>window.audioActions));
      throw error;
    });
    await page.waitForTimeout(400);
    await page.mouse.click(752,774); await page.waitForTimeout(300);
    assert.ok(await page.evaluate(()=>window.audioActions.some(a=>a[0]==='seek'&&Math.abs(a[1]-9)<0.1)), 'Next must seek to the next track on the rendered side');
    await page.mouse.click(628,774); await page.waitForTimeout(300);
    assert.ok(await page.evaluate(()=>window.audioActions.filter(a=>a[0]==='seek'&&a[1]===0).length>=2), 'Previous must return to the first track');
    await page.mouse.click(670,774); await page.waitForTimeout(200);
    assert.ok(await page.evaluate(()=>window.audioActions.some(a=>a[0]==='pause')), 'Play button must also pause');
    await shot('player-paused-ja');
    await page.mouse.click(710,774); await page.waitForTimeout(200);
    const after = await (await page.request.get(new URL('/api/library', base).href)).json();
    assert.deepEqual(after.collections, original.collections, 'Artwork editing must not change mix lists');
    assert.deepEqual(errors, []);
    const english = await browser.newContext({ viewport: { width: 1280, height: 850 }, locale: 'en-US' });
    const en = await english.newPage(); await en.goto(base);
    await en.waitForFunction(() => !document.getElementById('loading'));
    await en.waitForTimeout(800); await en.screenshot({ path: path.join(out, 'library-en.png') });
    console.log('PASS: Rust canvas, Japanese/English startup, direct file drop into playlist, duplicate reuse, cassette selection, text dragging/resizing, cross-face cut/paste, autosave, PDF download, recording modal, play/pause/previous/next/stop and unchanged mix lists');
  } finally { await browser.close(); }
})().catch(e => { console.error(e); process.exitCode = 1; });
