// Load the embedded scripts in the real HTML order, without source-text extraction.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const {spawn} = require('node:child_process');
const assert = require('node:assert/strict');

(async () => {
  const root = path.resolve(__dirname, '..');
  const work = path.join(root, 'work', `frontend-startup-${Date.now()}`);
  fs.mkdirSync(work, {recursive:true});
  const executable = path.join(root, 'target/debug/cassette-studio' + (process.platform === 'win32' ? '.exe' : ''));
  const server = spawn(executable, [], {cwd:root, windowsHide:true,
    env:{...process.env, CASSETTE_DATA:path.join(work,'data'), CASSETTE_PORT:'0'}});
  let logs = '';
  server.stdout.on('data', data => logs += data);
  server.stderr.on('data', data => logs += data);
  try {
    let base;
    for (let i=0; i<100; i++) {
      base = logs.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
      if (base) break;
      await new Promise(resolve => setTimeout(resolve, 50));
    }
    assert.ok(base, logs);
    const html = await (await fetch(base)).text();
    const scripts = [...html.matchAll(/<script\s+src="([^"]+)"/g)].map(match => match[1]);
    const elements = new Map();
    const handlers = new Map();
    const resizeObservers = [];
    const listen = (map,name,fn) => map.set(name,[...(map.get(name)||[]),fn]);
    const document = {
      documentElement:{},
      getElementById(id) { assert.ok(elements.has(id), `Missing DOM control: ${id}`); return elements.get(id); },
      querySelectorAll(selector) { return selector==='dialog[open]' ? [...elements.values()].filter(el => el.open) : []; },
      querySelector(selector) { return this.querySelectorAll(selector)[0] || null; },
      addEventListener(name,fn) { listen(handlers,name,fn); },
    };
    class Element {
      constructor(id) { this.id=id; this.value=''; this.style={setProperty(name,value){this[name]=value;}}; this.dataset={}; this.listeners=new Map(); this.classList={toggle(){},remove(){}}; }
      getBoundingClientRect() { return {height:this.mockHeight ?? 48}; }
      addEventListener(name,fn) { listen(this.listeners,name,fn); }
      appendChild(child) { child.parentNode=this; }
      contains(child) { return child.parentNode===this; }
      replaceChildren() { this.innerHTML=''; }
      showModal() { this.open=true; this.tagName='DIALOG'; }
      close() { this.open=false; for (const fn of this.listeners.get('close')||[]) fn({target:this}); for (const fn of handlers.get('close')||[]) fn({target:this}); }
      pause() { this.paused=true; }
      play() { this.paused=false; return Promise.resolve(); }
      load() {}
      removeAttribute(name) { this[name]=''; }
    }
    for (const match of html.matchAll(/\bid="([^"]+)"/g)) elements.set(match[1],new Element(match[1]));
    document.body=new Element('body');
    const storage=new Map();
    const context={document, console, navigator:{language:'ja-JP'}, Blob, URL,
      ResizeObserver:class { constructor(callback){this.callback=callback;} observe(target){resizeObservers.push({callback:this.callback,target});} },
      localStorage:{getItem:key=>storage.get(key),setItem:(key,value)=>storage.set(key,value)},
      fetch:(url,options)=>fetch(new URL(url,base),options), structuredClone,
      setTimeout:(fn,delay)=>{const timer=setTimeout(fn,delay); timer.unref(); return timer;}, clearTimeout,
      setInterval:(fn,delay)=>{const timer=setInterval(fn,delay); timer.unref(); return timer;}, clearInterval,
      addEventListener(){},
    };
    context.window=context;
    vm.createContext(context);
    for (const url of scripts) {
      const response=await fetch(base+url);
      assert.equal(response.status,200,url);
      assert.match(response.headers.get('content-type'),/javascript/,url);
      vm.runInContext(await response.text(),context,{filename:url});
    }
    await context.csI18nReady;
    for (let i=0; i<100 && vm.runInContext('busy',context); i++) await new Promise(resolve=>setTimeout(resolve,20));
    assert.equal(vm.runInContext('busy',context),0);
    assert.match(elements.get('tool-status').textContent,/音声/);
    assert.equal(elements.get('drop-zone').style['--track-controls-height'],'48px');
    assert.equal(resizeObservers[0].target,elements.get('track-controls'));
    elements.get('track-controls').mockHeight=80;resizeObservers[0].callback();
    assert.equal(elements.get('drop-zone').style['--track-controls-height'],'80px');
    elements.get('track-controls').mockHeight=48;resizeObservers[0].callback();


    for (const id of ['options-btn','analyze-volume','settings-btn','play-btn','export-btn','entry-auto','tone-start']) assert.equal(typeof elements.get(id).onclick,'function',id);
    assert.equal(elements.get('output-btn').hidden,true);
    context.navigator.mediaDevices={selectAudioOutput:async()=>({deviceId:'usb',label:'USB Audio'})};
    context.bindPlaybackControls();
    assert.equal(elements.get('output-btn').hidden,true);
    elements.get('audio').setSinkId=async id=>{elements.get('audio').sinkId=id;};
    context.bindPlaybackControls();
    assert.equal(elements.get('output-btn').hidden,false);
    await elements.get('output-btn').onclick();
    assert.equal(elements.get('audio').sinkId,'usb');
    delete context.navigator.mediaDevices.selectAudioOutput;
    context.bindPlaybackControls();
    assert.equal(elements.get('output-btn').hidden,true);
    elements.get('options-btn').onclick();
    context.toast('Completed');
    elements.get('tone-open').onclick();
    assert.equal(elements.get('options-dialog').open,false);
    assert.equal(elements.get('tone-dialog').open,true);
    elements.get('tone-frequency').value='400'; elements.get('tone-level').value='-12';
    await elements.get('tone-start').onclick();
    assert.equal(elements.get('audio').loop,true);
    elements.get('tone-dialog').close();
    assert.equal(elements.get('audio').loop,false);
    // Test actual recording controls with a prepared cassette and a deterministic clock.
    const audio=elements.get('audio'), recordDialog=elements.get('record-dialog');
    const originals={rendered:context.rendered,showDownloads:context.showDownloads,loadTape:context.loadTape,
      setInterval:context.setInterval,clearInterval:context.clearInterval,play:audio.play};
    const intervals=new Map();let timerId=0,playCalls=0;
    context.setInterval=fn=>{intervals.set(++timerId,fn);return timerId;};
    context.clearInterval=id=>intervals.delete(id);
    context.rendered=async()=>({name:'Recording test',settings:{side_seconds:60}});
    context.showDownloads=()=>{};
    context.loadTape=async()=>{audio.paused=true;};
    audio.play=()=>{playCalls++;audio.paused=false;return Promise.resolve();};
    const prepare=async()=>{
      await elements.get('record-btn').onclick();
      assert.equal(recordDialog.open,true);
      assert.equal(elements.get('record-start').disabled,false);
      assert.equal(elements.get('record-play-now').disabled,false);
    };
    const tick=()=>{for(const fn of [...intervals.values()])fn();};
    await prepare();elements.get('record-play-now').onclick();await Promise.resolve();
    assert.equal(playCalls,1);assert.equal(recordDialog.open,false);assert.equal(intervals.size,0);
    assert.equal(elements.get('play-btn').textContent,'Ⅱ');
    await prepare();elements.get('record-start').onclick();tick();tick();
    assert.equal(playCalls,1);assert.equal(elements.get('record-play-now').disabled,false);
    elements.get('record-play-now').onclick();await Promise.resolve();tick();
    assert.equal(playCalls,2);assert.equal(recordDialog.open,false);assert.equal(intervals.size,0);
    await prepare();elements.get('record-start').onclick();
    for(let i=0;i<4;i++)tick();assert.equal(playCalls,2);
    tick();await Promise.resolve();assert.equal(playCalls,3);assert.equal(intervals.size,0);
    await prepare();elements.get('record-start').onclick();tick();recordDialog.close();tick();
    assert.equal(playCalls,3);assert.equal(intervals.size,0);
    await prepare();recordDialog.close();assert.equal(playCalls,3);
    Object.assign(context,{rendered:originals.rendered,showDownloads:originals.showDownloads,
      loadTape:originals.loadTape,setInterval:originals.setInterval,clearInterval:originals.clearInterval});
    audio.play=originals.play;
    vm.runInContext(`lib={tracks:[{id:'demo',title:'Demo',duration:10,trim_start:0,trim_end:10}],collections:[{id:'tape',name:'Test',kind:'cassette',tracks:[],sides:[[{track_id:'demo',offset:0,duration:10,fade_in:0,fade_out:0}],[]],settings:{...defaults,side_seconds:5,fade:1,gap:0}}]};active='tape';side=0;selected=new Set([0]);draw();`,context);

    elements.get('edit-entry').onclick(); elements.get('entry-auto').onclick();
    assert.equal(elements.get('entry-duration').value,'5.0');
    assert.equal(Number(elements.get('entry-out').value),1);
    context.csSetLanguage('en');
    assert.equal(elements.get('settings-btn').textContent,'⚙ Recording settings');
    vm.runInContext("lib.collections.push({id:'playlist',name:'Playlist',kind:'playlist',tracks:['demo'],sides:[[],[]],settings:{...defaults}});active='playlist';selected=new Set();draw();",context);
    assert.equal(elements.get('add-library').hidden,false);
    assert.equal(typeof elements.get('settings-btn').onclick,'function');
    vm.runInContext("active='all';draw();",context);
    assert.equal(elements.get('add-library').hidden,true);
    assert.equal(elements.get('settings-btn').textContent,'⚙ Creation / import settings');

    // Selection is mandatory; failed work must leave the old library and unlock editing.
    const realApi=context.api;let reanalysisCalls=0;
    context.api=async(route,body,method)=>{
      reanalysisCalls++;
      assert.equal(route,'/api/reanalyze');assert.equal(method,'PUT');
      assert.deepEqual(Array.from(body.tracks),['demo']);
      assert.equal(elements.get('workspace').inert,true);
      const library=structuredClone(vm.runInContext('lib',context));
      library.tracks[0].trim_end=9;
      return library;
    };
    elements.get('options-dialog').showModal();
    await elements.get('reanalyze-btn').onclick();
    assert.equal(reanalysisCalls,0);assert.equal(elements.get('options-dialog').open,true);
    vm.runInContext("selected=new Set([0]);renderCache={key:'old'};downloadCaches.set('tape',{key:'old'});",context);
    await elements.get('reanalyze-btn').onclick();
    assert.equal(reanalysisCalls,1);assert.equal(elements.get('options-dialog').open,false);
    assert.equal(elements.get('workspace').inert,false);
    assert.equal(vm.runInContext('lib.tracks[0].trim_end',context),9);
    assert.equal(vm.runInContext('selected.size',context),0);
    assert.equal(vm.runInContext('downloadCaches.size',context),0);
    assert.equal(vm.runInContext('renderCache',context),null);
    const beforeError=vm.runInContext('JSON.stringify(lib)',context);
    context.api=async()=>{throw Error('Synthetic reanalysis failure');};
    vm.runInContext('selected=new Set([0])',context);
    await elements.get('reanalyze-btn').onclick();
    assert.equal(elements.get('workspace').inert,false);
    assert.equal(vm.runInContext('JSON.stringify(lib)',context),beforeError);
    context.api=realApi;
    // Duplicate file imports reuse a source in a new target, without duplicating either list.
    const originalFetch=context.fetch;const originalApi=context.api;let importSaves=0;
    context.FormData=FormData;
    context.fetch=async()=>({ok:true,json:async()=>({tracks:[],duplicates:[{filename:'copy.wav',track:structuredClone(vm.runInContext('lib.tracks[0]',context))}],track_ids:['demo','demo'],errors:[]})});
    context.api=async(route,body)=>{assert.equal(route,'/api/collections');importSaves++;return body;};
    vm.runInContext("active='playlist';lib.collections.find(c=>c.id==='playlist').tracks=[];selected=new Set();",context);
    const file=Object.assign(new Blob(['fixture']),{name:'copy.wav'});
    await context.importFiles([file]);
    assert.equal(vm.runInContext('lib.tracks.length',context),1);
    assert.equal(vm.runInContext("lib.collections.find(c=>c.id==='playlist').tracks.join(',')",context),'demo');
    assert.equal(importSaves,1);
    await context.importFiles([file]);assert.equal(importSaves,1);
    assert.equal(vm.runInContext('lib.tracks.length',context),1);
    assert.equal(vm.runInContext("lib.collections.find(c=>c.id==='playlist').tracks.length",context),1);
    context.fetch=originalFetch;context.api=originalApi;
    // All-tracks deletion must enable on selection, honor cancellation, and keep failed changes local.
    const deleteApi=context.api,deleteConfirm=context.askConfirm;let deleteCalls=0;
    vm.runInContext("active='all';selected=new Set();draw();",context);
    assert.equal(elements.get('remove-selected').disabled,true);
    vm.runInContext('selected=new Set([0]);drawSelection();',context);
    assert.equal(elements.get('remove-selected').disabled,false);
    vm.runInContext('draw();',context);assert.equal(elements.get('remove-selected').disabled,false);
    const deleteBefore=vm.runInContext('JSON.stringify(lib)',context);
    context.askConfirm=async()=>false;
    context.api=async()=>{deleteCalls++;throw Error('Must not delete after cancellation');};
    await elements.get('remove-selected').onclick();assert.equal(deleteCalls,0);
    assert.equal(vm.runInContext('JSON.stringify(lib)',context),deleteBefore);
    context.askConfirm=async()=>true;
    context.api=async(route,body,method)=>{
      deleteCalls++;assert.equal(route,'/api/tracks');assert.equal(method,'DELETE');
      assert.deepEqual(Array.from(body.tracks),['demo']);assert.equal(elements.get('workspace').inert,true);
      return {tracks:[],collections:[]};
    };
    await elements.get('remove-selected').onclick();assert.equal(deleteCalls,1);
    assert.equal(vm.runInContext('lib.tracks.length',context),0);
    assert.equal(vm.runInContext('selected.size',context),0);
    assert.equal(elements.get('workspace').inert,false);
    assert.equal(elements.get('remove-selected').disabled,true);
    vm.runInContext('lib='+deleteBefore+';selected=new Set([0]);draw();',context);
    context.api=async()=>{throw Error('Synthetic delete failure');};
    await elements.get('remove-selected').onclick();
    assert.equal(vm.runInContext('JSON.stringify(lib)',context),deleteBefore);
    assert.equal(elements.get('workspace').inert,false);
    assert.equal(elements.get('remove-selected').disabled,false);
    context.api=deleteApi;context.askConfirm=deleteConfirm;
    console.log(`PASS: ${scripts.length} embedded scripts, startup, button bindings, modal notifications, tone lifecycle, auto fit, language, recording immediate/delayed/cancel`);
  } finally { server.kill(); }
})().catch(error=>{console.error(error);process.exitCode=1;});
