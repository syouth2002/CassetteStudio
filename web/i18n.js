'use strict';
// Keys are the Japanese source messages. User-provided titles are never translated.
const csSavedLanguage=localStorage.getItem('cs-language');
const csEnvironmentLanguage=navigator.language||navigator.languages?.[0]||'en';
let csLanguage=['ja','en'].includes(csSavedLanguage)?csSavedLanguage:(/^ja(?:-|$)/i.test(csEnvironmentLanguage)?'ja':'en');
let csEnglish={};
window.csText=(key,...values)=>{
 const template=csLanguage==='en'?(csEnglish[key]??key):key;
 return template.replace(/\{(\d+)\}/g,(_,index)=>String(values[+index]??''));
};
const csEscapeRegex=text=>text.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
let csMessagePatterns=[];
window.csTranslateMessage=message=>{
 message=String(message??'');
 if(csLanguage==='en'&&csEnglish[message])return csEnglish[message];
 for(const item of csMessagePatterns){
  const match=item.pattern.exec(message);if(!match)continue;
  const params={};item.slots.forEach((slot,i)=>params[slot]=match[i+1]);
  const template=csLanguage==='en'?item.en:item.ja;
  return template.replace(/\{([^{}]*)\}/g,(_,slot)=>csTranslateMessage(params[slot]??''));
 }
 return message;
};
function csApplyStatic(){
 document.documentElement.lang=csLanguage;
 document.querySelectorAll('[data-i18n]').forEach(el=>el.textContent=csText(el.dataset.i18n));
 for(const attribute of ['title','aria-label','placeholder'])document.querySelectorAll(`[data-i18n-${attribute}]`).forEach(el=>el.setAttribute(attribute,csText(el.getAttribute(`data-i18n-${attribute}`))));
 document.getElementById('language-select').value=csLanguage;
}
function csSetLanguage(language){
 csLanguage=language==='en'?'en':'ja';localStorage.setItem('cs-language',csLanguage);csApplyStatic();
 if(typeof draw==='function')draw();
 if(typeof player!=='undefined'&&player)updatePlayer();
 else document.getElementById('playing-title').textContent=csText('再生する曲を選択');
 for(const id of ['tool-status','busy-label','toast','record-detail','record-start','gain-measurement','gain-dc-measurement','batch-source','name-prompt','confirm-message']){
  const el=document.getElementById(id);if(el)el.textContent=csTranslateMessage(el.textContent);
 }
}
window.askText=(title,value='')=>new Promise(resolve=>{
 const dialog=document.getElementById('name-dialog'),input=document.getElementById('name-value');
 document.getElementById('name-prompt').textContent=title;input.value=value;dialog.returnValue='';
 dialog.addEventListener('close',()=>resolve(dialog.returnValue==='ok'?input.value:null),{once:true});
 dialog.showModal();input.focus();input.select();
});
window.askConfirm=message=>new Promise(resolve=>{
 const dialog=document.getElementById('confirm-dialog');dialog.returnValue='';document.getElementById('confirm-message').textContent=message;
 dialog.addEventListener('close',()=>resolve(dialog.returnValue==='ok'),{once:true});dialog.showModal();
});
window.csI18nReady=fetch('/en.json').then(response=>{if(!response.ok)throw Error('Could not load translations');return response.json();}).then(catalog=>{
 csEnglish=catalog;
 for(const [ja,en]of Object.entries(catalog))for(const source of [ja,en]){
  const parts=source.split(/(\{[^{}]*\})/g),slots=[];
  const pattern=parts.map(part=>{if(/^\{[^{}]*\}$/.test(part)){slots.push(part.slice(1,-1));return '([\\s\\S]*?)';}return csEscapeRegex(part);}).join('');
  csMessagePatterns.push({ja,en,slots,pattern:new RegExp(`^${pattern}$`)});
 }
 // Specific sentences precede broad patterns such as track counts.
 csMessagePatterns.sort((a,b)=>b.ja.length-a.ja.length);
 csApplyStatic();document.getElementById('language-select').addEventListener('change',e=>csSetLanguage(e.target.value));
});
