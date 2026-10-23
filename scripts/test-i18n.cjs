const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const catalog=JSON.parse(fs.readFileSync('web/en.json','utf8'));
const language={value:'ja',addEventListener(){},textContent:''};
const storage=new Map();
const context={window:{},document:{documentElement:{},querySelectorAll:()=>[],getElementById:()=>language},localStorage:{getItem:k=>storage.get(k),setItem:(k,v)=>storage.set(k,v)},fetch:async()=>({ok:true,json:async()=>catalog})};
context.navigator={language:'ja-JP'};
const source=fs.readFileSync('web/i18n.js','utf8');
for(const [environment,saved,expected] of [['ja-JP',null,'ja'],['ja',null,'ja'],['en-US',null,'en'],['de-DE',null,'en'],['zh-CN',null,'en'],['',null,'en'],['en-US','ja','ja'],['ja-JP','en','en'],['fr-FR','invalid','en']]){
 const test={...context,window:{},navigator:{language:environment},localStorage:{getItem:()=>saved,setItem(){}}};
 test.window=test;vm.createContext(test);vm.runInContext(source,test);
 assert.equal(vm.runInContext('csLanguage',test),expected,`Environment ${environment}, saved ${saved}`);
}
context.window=context;vm.createContext(context);vm.runInContext(source,context);
(async()=>{
 await context.csI18nReady;
 vm.runInContext('csSetLanguage("en")',context);
 assert.equal(context.csText('すべての曲'),'All tracks');
 assert.equal(context.csText('{0}曲',3),'3 tracks');
 assert.equal(context.csTranslateMessage('曲が見つかりません'),'Track not found');
 assert.equal(context.csTranslateMessage('片面の長さの値が範囲外です'),'Side length is out of range');
 assert.equal(context.csTranslateMessage('音声処理に失敗しました: Unknown decoder'),'Audio processing failed: Unknown decoder');
 vm.runInContext('csSetLanguage("ja")',context);
 assert.equal(context.csTranslateMessage('Remaining 3:00'),'残り 3:00');
 assert.equal(storage.get('cs-language'),'ja');
 assert.equal(context.csText('カセット名'),'カセット名');
 // Every explicitly marked static UI message has an English translation.
 const html=fs.readFileSync('web/index.html','utf8');
 for(const m of html.matchAll(/data-i18n(?:-(?:title|aria-label|placeholder))?="([^"]*)"/g)){
  const key=m[1].replace(/&quot;/g,'"').replace(/&#x27;/g,"'").replace(/&gt;/g,'>').replace(/&lt;/g,'<').replace(/&amp;/g,'&');
  assert.ok(Object.hasOwn(catalog,key),`Missing: ${key}`);
 }
 for(const [key,value]of Object.entries(catalog)){
  const placeholders=s=>[...s.matchAll(/\{([^{}]*)\}/g)].map(m=>m[1]).sort().join('|');
  assert.equal(placeholders(key),placeholders(value),`Placeholders: ${key}`);
 }
 console.log(`PASS: Japanese/English, runtime messages, nested errors, persistence, ${Object.keys(catalog).length} catalog entries, placeholder parity`);
})().catch(error=>{console.error(error);process.exitCode=1;});
