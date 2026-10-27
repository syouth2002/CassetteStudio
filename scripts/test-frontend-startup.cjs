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
    const listen = (map,name,fn) => map.set(name,[...(map.get(name)||[]),fn]);
    const document = {
      documentElement:{},
      getElementById(id) { assert.ok(elements.has(id), `Missing DOM control: ${id}`); return elements.get(id); },
      querySelectorAll(selector) { return selector==='dialog[open]' ? [...elements.values()].filter(el => el.open) : []; },
      querySelector(selector) { return this.querySelectorAll(selector)[0] || null; },
      addEventListener(name,fn) { listen(handlers,name,fn); },
    };
    class Element {
      constructor(id) { this.id=id; this.value=''; this.style={}; this.dataset={}; this.listeners=new Map(); this.classList={toggle(){},remove(){}}; }
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
    assert.equal(elements.get('toast').parentNode,elements.get('options-dialog'));
    elements.get('tone-open').onclick();
    assert.equal(elements.get('options-dialog').open,false);
    assert.equal(elements.get('tone-dialog').open,true);
    elements.get('tone-frequency').value='400'; elements.get('tone-level').value='-12';
    await elements.get('tone-start').onclick();
    assert.equal(elements.get('audio').loop,true);
    elements.get('tone-dialog').close();
    assert.equal(elements.get('audio').loop,false);
    vm.runInContext(`lib={tracks:[{id:'demo',title:'Demo',duration:10,trim_start:0,trim_end:10}],collections:[{id:'tape',name:'Test',kind:'cassette',tracks:[],sides:[[{track_id:'demo',offset:0,duration:10,fade_in:0,fade_out:0}],[]],settings:{...defaults,side_seconds:5,fade:1,gap:0}}]};active='tape';side=0;selected=new Set([0]);draw();`,context);
    elements.get('edit-entry').onclick(); elements.get('entry-auto').onclick();
    assert.equal(elements.get('entry-duration').value,'5.0');
    assert.equal(Number(elements.get('entry-out').value),1);
    context.csSetLanguage('en');
    assert.equal(elements.get('settings-btn').textContent,'⚙ Recording settings');
    console.log(`PASS: ${scripts.length} embedded scripts, startup, button bindings, modal notifications, tone lifecycle, auto fit, language`);
  } finally { server.kill(); }
})().catch(error=>{console.error(error);process.exitCode=1;});
