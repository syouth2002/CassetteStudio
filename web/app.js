'use strict';
const $=id=>document.getElementById(id);
const esc=s=>String(s??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const defaults={side_seconds:1800,length_correction:0,gap:2,fill_threshold:30,fade:5,continuation:'restart',rewind:3,silence_db:-50,silence_min:.2,normalize:false,remove_dc:false,target_lufs:-18,peak_db:-1};
let gainTrackId=null;
const downloadCaches=new Map();
let lib={tracks:[],collections:[]}, active='all',side=0, selected=new Set();
let settings={...defaults},busy=0,saveChain=Promise.resolve(),drag=null,renderCache=null,batchTracks=[];
let player=null,playGeneration=0,gapTimer=null,countdown=null,toastTimer=null;
const audio=$('audio'); audio.volume=.8;
try{settings={...defaults,...JSON.parse(localStorage.getItem('cs-settings')||'{}')};}catch{}
const time=n=>{n=Math.max(0,Math.round(n||0));return `${Math.floor(n/60)}:${String(n%60).padStart(2,'0')}`;};
const duration=t=>Math.max(0,t.trim_end-t.trim_start);
const track=id=>lib.tracks.find(t=>t.id===id);
const current=()=>lib.collections.find(c=>c.id===active);
const config=()=>({...defaults,...(current()?.settings||settings)});
const db=n=>`${n>=0?'+':''}${n.toFixed(1)} dB`;
function gainInfo(t,s=config()){
 const l=s.remove_dc?t.dc?.corrected_loudness:t.loudness,m=t.manual_gain_db||0;
 if(!s.normalize&&!m&&!s.remove_dc)return {text:csText("補正なし"),gain:0,limited:false};
 if(!l)return {text:csText("音量未解析"),gain:0,limited:false};
 const requested=(s.normalize&&l.integrated_lufs!=null?s.target_lufs-l.integrated_lufs:0)+m;
 const gain=Math.min(requested,l.true_peak_db==null?Infinity:s.peak_db-.1-l.true_peak_db);
 const limited=gain<requested-.01;
 return {text:`${db(gain)}${limited?csText(" · ピーク制限"):''}`,gain,limited};
}
function mergeTracks(tracks){for(const t of tracks){const i=lib.tracks.findIndex(v=>v.id===t.id);if(i>=0)lib.tracks[i]=t;}}
function mergeMeasurements(tracks){for(const t of tracks){const old=track(t.id);if(old){old.loudness=t.loudness;old.dc=t.dc;}}}
const isTape=()=>current()?.kind==='cassette';
const total=(entries,s=config())=>entries.reduce((n,e)=>n+e.duration,0)+Math.max(0,entries.length-1)*s.gap;
function entries(){const c=current();return !c?lib.tracks.map(t=>whole(t.id)):c.kind==='playlist'?c.tracks.map(whole):c.sides[side];}
function whole(id){return {track_id:id,offset:0,duration:duration(track(id)),fade_in:0,fade_out:0,continuation:false};}
function toast(msg){clearTimeout(toastTimer);$('toast').textContent=csTranslateMessage(msg);$('toast').hidden=false;toastTimer=setTimeout(()=>$('toast').hidden=true,8000);}
async function api(path,body,method='POST'){
 const response=await fetch(path,body===undefined?{}:{method,headers:{'content-type':'application/json'},body:JSON.stringify(body)});
 let data;try{data=await response.json();}catch{throw new Error(csText("サーバーから応答を取得できません ({0})", response.status));}
 if(!response.ok)throw new Error(data.error||csText("処理に失敗しました ({0})", response.status));return data;
}
async function task(message,fn){busy++;$('busy-label').textContent=message;$('busy').hidden=false;document.querySelectorAll('.toolbar button,#export-btn,#record-btn,#save-settings,#reanalyze-btn').forEach(b=>b.disabled=true);try{return await fn();}catch(e){toast(e.message);return null;}finally{busy--;if(!busy){$('busy').hidden=true;document.querySelectorAll('.toolbar button,#export-btn,#record-btn,#save-settings,#reanalyze-btn').forEach(b=>b.disabled=false);}}}
function persist(c){
 const copy=structuredClone(c);
 saveChain=saveChain.catch(()=>{}).then(()=>api('/api/collections',copy));
 saveChain.catch(e=>toast(csText("保存に失敗しました: {0}", e.message)));return saveChain;
}
function changed(){if(player?.collection===active)stop();renderCache=null;$('downloads').replaceChildren();if(current())persist(current());draw();}
function selectCollection(id){active=id;side=0;selected.clear();draw();}
function drawNav(){
 $('track-count').textContent=lib.tracks.length;$('all-tracks').classList.toggle('active',active==='all');
 for(const [kind,target]of [['playlist','playlists'],['cassette','cassettes']]){
  $(target).innerHTML=lib.collections.filter(c=>c.kind===kind).map(c=>`<button class="collection-item ${c.id===active?'active':''}" data-collection="${esc(c.id)}">${kind==='cassette'?'<span class="cassette-art" aria-hidden="true">● ●</span>':''}<span class="collection-text">${esc(c.name)}<small>${kind==='cassette'?csText("A {0}曲 · B {1}曲", c.sides[0].length, c.sides[1].length):csText("{0}曲", c.tracks.length)}</small></span></button>`).join('');
 }
}
function drawSelection(){
 const list=entries();
 document.querySelectorAll('#tracks tr').forEach(row=>{const chosen=selected.has(+row.dataset.index);row.classList.toggle('selected',chosen);row.querySelector('input').checked=chosen;});
 $('selection-count').textContent=selected.size?csText("{0}曲選択", selected.size):'';
 $('adjust-volume').disabled=selected.size!==1;
 $('edit-entry').disabled=selected.size!==1;$('move-side').disabled=!selected.size;$('remove-selected').disabled=!selected.size||active==='all';
 $('select-all').checked=list.length>0&&selected.size===list.length;$('select-all').indeterminate=selected.size>0&&selected.size<list.length;
}
function draw(){
 syncDownloads();drawNav();const c=current(),tape=isTape(),list=entries();selected=new Set([...selected].filter(i=>i<list.length));
 $('collection-name').textContent=c?.name||csText("すべての曲");$('collection-type').textContent=tape?'YOUR PERSONAL MIXTAPE':c?'PLAYLIST':'YOUR MUSIC COLLECTION';
 $('collection-summary').textContent=tape?csText("{0} × 2面 / 曲間 {1}秒 / {2}", time(config().side_seconds), config().gap, config().continuation==='restart'?csText("引き継ぎ：最初から"):csText("引き継ぎ：途中からフェードイン")):csText("{0}曲 · 無音除去後 {1}", list.length, time(list.reduce((n,e)=>n+e.duration,0)));
 for(const id of ['rename-btn','delete-collection','add-library'])$(id).hidden=!c;
 for(const id of ['cassette-overview','side-tabs','export-btn','record-btn','move-side','edit-entry'])$(id).hidden=!tape;
 $('list-label').hidden=tape;$('edit-entry').disabled=selected.size!==1;$('move-side').disabled=!selected.size;$('remove-selected').disabled=!selected.size||active==='all';
 $('selection-count').textContent=selected.size?csText("{0}曲選択", selected.size):'';$('select-all').checked=list.length>0&&selected.size===list.length;
 $('adjust-volume').disabled=selected.size!==1;
 $('select-all').indeterminate=selected.size>0&&selected.size<list.length;
 $('make-manual').hidden=tape;$('make-auto').hidden=tape&&!c.sides.every(es=>es.length===0);
 $('make-auto').textContent=tape?csText("✦ 空のカセットへ自動配置"):csText("✦ 自動でカセット作成");
 $('edit-help').textContent=active==='all'?csText("曲を選択してカセットやプレイリストを作成できます"):csText("⋮⋮ ドラッグで曲順変更 · ダブルクリックで再生");
 $('tracks').innerHTML=list.map((e,i)=>{const t=track(e.track_id);if(!t)return '';const badges=[];
  if(e.continuation)badges.push(csText("<span class=\"badge\">A面から引き継ぎ</span>"));
  if(e.fade_in)badges.push(csText("<span class=\"badge fade\">IN {0}秒</span>", e.fade_in.toFixed(1)));
  if(e.fade_out)badges.push(csText("<span class=\"badge fade\">OUT {0}秒</span>", e.fade_out.toFixed(1)));
  if(e.offset)badges.push(`<span class="badge">${time(e.offset)}〜</span>`);
  if(duration(t)<=0)badges.push(csText("<span class=\"badge warning\">全体が無音</span>"));
  else if(t.duration-duration(t)>.05)badges.push(csText("<span class=\"badge\">無音除去</span>"));
  const correction=gainInfo(t);
  if(config().remove_dc)badges.push(`<span class="badge" title="${esc(t.dc?csText("DCオフセット：L {0}% / R {1}%",(t.dc.offset[0]*100).toFixed(4),(t.dc.offset[1]*100).toFixed(4)):csText("未解析"))}">${csText(t.dc?"DC除去":"DC未解析")}</span>`);
  if(config().normalize||config().remove_dc||t.manual_gain_db)badges.push(`<span class="badge ${correction.limited?'warning':'gain-badge'}" title="${esc(t.loudness?.integrated_lufs!=null?csText("元の音量 {0} LUFS", t.loudness.integrated_lufs.toFixed(1)):csText("未解析"))}">${esc(correction.text)}</span>`);
  return csText("<tr data-index=\"{0}\" draggable=\"{1}\" class=\"{2}\"><td><input type=\"checkbox\" aria-label=\"{3}を選択\" data-index=\"{4}\" {5}></td><td>{6}{7}</td><td class=\"title-cell\"><span class=\"song-title\">{8}</span><small>{9}</small></td><td>{10}</td><td>{11}</td><td>{12}</td></tr>", i, active!=='all', selected.has(i)?'selected':'', esc(t.title), i, selected.has(i)?'checked':'', active==='all'?'':'<span class="handle">⋮⋮</span>', String(i+1).padStart(2,'0'), esc(t.title), esc(t.artist||t.filename), time(e.duration), time(t.duration), badges.join(''));
 }).join('');
 $('empty-state').hidden=list.length>0;
 if(tape){
  $('cassette-overview').innerHTML=c.sides.map((es,i)=>{const n=total(es),room=config().side_seconds-n;return `<div class="side-card ${i===side?'active':''} ${room<-.001?'over':''}" data-show-side="${i}"><div class="side-title"><span>SIDE ${i?'B':'A'} · ${es.length} TRACKS</span><span>${time(n)} / ${time(config().side_seconds)}</span></div><div class="bar"><div style="width:${Math.min(100,n/config().side_seconds*100)}%"></div></div><p>${room<-.001?csText("時間超過 {0} — 曲を移動・削除してください", time(-room)):csText("残り {0}", time(room))}</p></div>`;}).join('');
  document.querySelectorAll('[data-side]').forEach(b=>b.classList.toggle('active',+b.dataset.side===side));
 }
 markPlaying();
}
function numberedName(base){
 const prefix=`${base.trim()} · `;let highest=0n;
 for(const c of lib.collections){const name=c.name.trim();if(!name.startsWith(prefix))continue;const suffix=name.slice(prefix.length);if(/^\d+$/.test(suffix)){const n=BigInt(suffix);if(n>highest)highest=n;}}
 return `${prefix}${highest+1n}`;
}
async function newCollection(kind){const name=await askText(kind==='playlist'?csText("プレイリスト名"):csText("カセット名"),numberedName(kind==='playlist'?csText("新しいプレイリスト"):'Untitled Mixtape'));if(!name?.trim())return;
 const chosen=[...selected].sort((a,b)=>a-b).map(i=>entries()[i].track_id);
 const c=await task(csText("作成しています…"),()=>api('/api/collections',{id:'',name:name.trim(),kind,tracks:kind==='playlist'?chosen:[],sides:[[],[]],settings:structuredClone(config())}));
 if(c){lib.collections.push(c);selectCollection(c.id);if(kind==='cassette'){fillSettings();$('settings-dialog').returnValue='';$('settings-dialog').showModal();}}
}
async function makeTape(optimize){
 const blanks=lib.collections.filter(c=>c.kind==='cassette'&&c.sides.every(es=>es.length===0));
 if(optimize&&blanks.length){
  const source=isTape()?lib.tracks.map(t=>whole(t.id)):entries();
  batchTracks=[...new Set((!isTape()&&selected.size?[...selected].sort((a,b)=>a-b).map(i=>source[i]):source).filter(e=>e.duration>0).map(e=>e.track_id))];
  if(!batchTracks.length){toast(csText("収録する音源をライブラリに追加してください"));return;}
  $('batch-source').textContent=csText("選曲元：{0}（{1}曲）", isTape()?csText("すべての曲"):current()?.name||csText("すべての曲"), batchTracks.length);
  $('batch-targets').innerHTML=blanks.map(c=>csText("<label class=\"batch-target\"><input type=\"checkbox\" value=\"{0}\" checked><span>{1}<small>片面 {2} × 2面 · 曲間 {3}秒</small></span></label>", esc(c.id), esc(c.name), time(c.settings.side_seconds), c.settings.gap)).join('');
  $('batch-dialog').showModal();return;
 }
 const list=entries(),ids=(selected.size?[...selected].sort((a,b)=>a-b).map(i=>list[i]):list).filter(e=>e.duration>0).map(e=>e.track_id);
 if(!ids.length){toast(csText("音楽・動画ファイルを追加し、収録する曲を選択してください"));return;}
 const s=structuredClone(config()),name=await askText(csText("作成するカセットの名前"),numberedName(current()?.name||'My Mixtape'));if(!name?.trim())return;
 await task(csText("曲の長さを計算してA・B面へ配置しています…"),async()=>{
  const sides=await api('/api/plan',{tracks:ids,settings:s,optimize});
  const c=await api('/api/collections',{id:'',name:name.trim(),kind:'cassette',tracks:ids,sides,settings:s});
  lib.collections.push(c);selectCollection(c.id);
  const used=new Set(sides.flat().map(e=>e.track_id));const omitted=ids.filter(id=>!used.has(id)).length;
  toast(omitted?csText("カセットを作成しました。収まらなかった曲：{0}曲", omitted):csText("カセットを作成しました"));
 });
}
async function importFiles(files){if(!files.length)return;const destination=active;
 await task(csText("{0}件の音源を取り込み、前後の無音を解析しています…", files.length),async()=>{
  const form=new FormData();form.append('settings',JSON.stringify(config()));for(const f of files)form.append('files',f);
  const r=await fetch('/api/import',{method:'POST',body:form});const data=await r.json();if(!r.ok)throw Error(data.error||csText("取り込みに失敗しました"));
  lib.tracks.push(...data.tracks);await attachTracks(data.tracks.map(t=>t.id),destination);draw();
  toast(data.errors.length?csText("{0}件を追加。\n{1}", data.tracks.length, data.errors.join('\n')):csText("{0}件の音源を追加しました", data.tracks.length));
 });
}
async function attachTracks(ids,destination=active){const c=lib.collections.find(c=>c.id===destination);if(!c)return;
 if(c.kind==='playlist')c.tracks.push(...ids);else c.sides[side].push(...ids.filter(id=>duration(track(id))>0).map(whole));
 renderCache=null;await persist(c);
}
const settingFields=[['gap','gap'],['fill-threshold','fill_threshold'],['fade','fade'],['continuation','continuation'],['rewind','rewind'],['silence-db','silence_db'],['silence-min','silence_min'],['target-lufs','target_lufs'],['peak-db','peak_db']];
function fillSettings(){const s=config();const nominal=s.side_seconds-(s.length_correction||0);$('side-minutes').value=nominal/60;$('length-correction').value=s.length_correction||0;const preset=[46,60,74,80,90,120].find(n=>Math.abs(n*30-nominal)<.001);$('tape-preset').value=preset||'custom';$('normalize').value=String(s.normalize);$('remove-dc').value=String(s.remove_dc||false);for(const [id,key]of settingFields)$(id).value=['gap','fade','rewind'].includes(key)?Math.max(key==='fade'?.5:0,Math.round(s[key]*2)/2):s[key];}
function readSettings(){const s={side_seconds:+$('side-minutes').value*60+(+$('length-correction').value),length_correction:+$('length-correction').value,normalize:$('normalize').value==='true',remove_dc:$('remove-dc').value==='true'};for(const [id,key]of settingFields)s[key]=key==='continuation'?$(id).value:+$(id).value;return s;}
function saveSettings(){stop();settings=readSettings();localStorage.setItem('cs-settings',JSON.stringify(settings));if(current()){current().settings=structuredClone(settings);changed();}renderCache=null;draw();}
async function rendered(){
 const c=current();if(!c||c.kind!=='cassette')throw Error(csText("カセットを選択してください"));
 if(c.sides.some(es=>total(es)>config().side_seconds+.001))throw Error(csText("面の長さを超えています。曲を移動・削除してください"));
 const sides=structuredClone(c.sides),s=structuredClone(c.settings),id=c.id,name=c.name;
 const key=JSON.stringify([id,name,sides,s,lib.tracks.map(t=>[t.id,t.manual_gain_db||0,t.dc||null])]);if(renderCache?.key===key)return renderCache;
 const data=await api('/api/render',{sides,settings:s});mergeMeasurements(data.tracks||[]);draw();renderCache={key:JSON.stringify([id,name,sides,s,lib.tracks.map(t=>[t.id,t.manual_gain_db||0,t.dc||null])]),urls:data.urls,sides,settings:s,collection:id,name};return renderCache;
}
function showDownloads(cache){downloadCaches.set(cache.collection,cache);syncDownloads();}
function syncDownloads(){const c=current(),cache=c?.kind==='cassette'?downloadCaches.get(c.id):null;const key=c?JSON.stringify([c.id,c.name,c.sides,c.settings,lib.tracks.map(t=>[t.id,t.manual_gain_db||0,t.dc||null])]):null;if(!cache||cache.key!==key){$('downloads').replaceChildren();return;}$('downloads').innerHTML=cache.urls.map((u,i)=>csText("<a href=\"{0}\" download=\"{1}-{2}.wav\">↓ SIDE {3} · WAVを保存</a>", esc(u), esc(cache.name), i?'B':'A', i?'B':'A')).join('');}
function stop(){playGeneration++;clearTimeout(gapTimer);clearInterval(countdown);gapTimer=null;countdown=null;audio.pause();if(audio.src)audio.currentTime=player?.mode==='playlist'?player.list[player.index].offset+track(player.list[player.index].track_id).trim_start:0;player=null;$('play-btn').textContent='▶';$('playing-title').textContent=csText("再生する曲を選択");$('playing-detail').textContent='Cassette Studio';$('seek').value=0;$('elapsed').textContent='0:00';$('remaining').textContent='−0:00';markPlaying();}
function markPlaying(){document.querySelectorAll('#tracks tr').forEach(row=>row.classList.toggle('playing',!!player&&active===player.collection&&(!isTape()||side===player.side)&&+row.dataset.index===(player.rows?.[player.index]??player.index)));}
function tapeIndex(position,p){let elapsed=0;for(let i=0;i<p.list.length;i++){elapsed+=p.list[i].duration;if(position<elapsed+p.gap||i===p.list.length-1)return i;elapsed+=p.gap;}return 0;}
function startAt(i,p){return p.list.slice(0,i).reduce((n,e)=>n+e.duration+p.gap,0);}
function updatePlayer(){if(!player)return;const p=player;let position=audio.currentTime,length=audio.duration||0;
 if(p.mode==='playlist'){const e=p.list[p.index],t=track(e.track_id);position-=t.trim_start+e.offset;length=e.duration;
  if(position>=length-.02&&!audio.paused){audio.pause();if(p.index<p.list.length-1){gapTimer=setTimeout(()=>playPlaylistIndex(p.index+1).catch(e=>toast(e.message)),p.gap*1000);}else{$('play-btn').textContent='▶';} }
 }else{p.index=tapeIndex(position,p);}
 const e=p.list[p.index],t=e&&track(e.track_id);$('playing-title').textContent=t?.title||p.name;
 $('playing-detail').textContent=p.mode==='tape'?`${p.name} · SIDE ${p.side?'B':'A'} · ${p.index+1}/${p.list.length}`:`${t?.artist||csText("プレイリスト")} · ${p.index+1}/${p.list.length}`;
 $('elapsed').textContent=time(position);$('remaining').textContent=`−${time(length-position)}`;$('seek').value=length?Math.min(1000,position/length*1000):0;markPlaying();
}
async function playPlaylistIndex(i){const p=player;if(!p||p.mode!=='playlist'||i<0||i>=p.list.length)return;clearTimeout(gapTimer);gapTimer=null;const gen=++playGeneration;
 audio.pause();p.index=i;const e=p.list[i],t=track(e.track_id);const data=await api(`/api/preview/${encodeURIComponent(t.id)}`,p.settings);if(gen!==playGeneration)return;mergeMeasurements(data.track?[data.track]:[]);draw();
 audio.src=data.url;await new Promise((resolve,reject)=>{audio.onloadedmetadata=()=>resolve();audio.onerror=()=>reject(Error(csText("音声を再生できません")));audio.load();});if(gen!==playGeneration)return;
 audio.currentTime=t.trim_start+e.offset;await audio.play();$('play-btn').textContent='Ⅱ';updatePlayer();
}
async function startPlayback(index=0){if(busy)return;
 if(isTape()){
  const chosenSide=side;
  const cache=await task(csText("曲間とフェードを反映した録音用音声を準備しています…"),rendered);if(!cache)return;
  await loadTape(cache,chosenSide,index,true);
 }else{
  const playable=entries().map((e,row)=>({e,row})).filter(v=>v.e.duration>0),list=playable.map(v=>v.e);if(!list.length){toast(csText("再生できる曲がありません"));return;}
  const start=playable.findIndex(v=>v.row>=index);
  stop();player={mode:'playlist',list:structuredClone(list),rows:playable.map(v=>v.row),index:0,gap:config().gap,settings:structuredClone(config()),collection:active,name:current()?.name||csText("すべての曲")};
  await task(csText("再生を準備しています…"),()=>playPlaylistIndex(start<0?list.length-1:start));
 }
}
async function loadTape(cache,chosenSide,index,play){stop();const list=cache.sides[chosenSide];if(!list.length){toast(csText("この面には曲がありません"));return;}
 const gen=playGeneration;player={mode:'tape',list,index:Math.min(index,list.length-1),side:chosenSide,gap:cache.settings.gap,collection:cache.collection,name:cache.name};
 audio.src=cache.urls[chosenSide];await new Promise((resolve,reject)=>{audio.onloadedmetadata=resolve;audio.onerror=()=>reject(Error(csText("音声を読み込めません")));audio.load();});if(gen!==playGeneration)return;
 audio.currentTime=startAt(player.index,player);if(play){await audio.play();$('play-btn').textContent='Ⅱ';}updatePlayer();
}

document.addEventListener('click',e=>{const col=e.target.closest('[data-collection]');if(col)selectCollection(col.dataset.collection);const s=e.target.closest('[data-side],[data-show-side]');if(s){side=+(s.dataset.side??s.dataset.showSide);selected.clear();draw();}const close=e.target.closest('[data-close]');if(close)$(close.dataset.close).close();});
$('all-tracks').onclick=()=>selectCollection('all');
$('new-playlist').onclick=()=>newCollection('playlist');$('new-cassette').onclick=()=>newCollection('cassette');
$('rename-btn').onclick=async()=>{const c=current(),name=await askText(csText("新しい名前"),c.name);if(name?.trim()){c.name=name.trim();changed();}};
$('delete-collection').onclick=async()=>{const c=current();if(!await askConfirm(csText("「{0}」を削除しますか？ 音源ファイルはライブラリに残ります。", c.name)))return;await saveChain;const r=await task(csText("削除しています…"),()=>api(`/api/collections/${c.id}`,{},'DELETE'));if(r){lib.collections=lib.collections.filter(v=>v.id!==c.id);selectCollection('all');}};
$('make-manual').onclick=()=>makeTape(false);$('make-auto').onclick=()=>makeTape(true);
$('analyze-volume').onclick=()=>task(csText("曲全体の音量とピークを解析しています…"),async()=>{
 const es=entries(),ids=[...new Set((selected.size?[...selected].map(i=>es[i]):es).map(e=>e.track_id))];
 if(!ids.length){toast(csText("音源を追加してください"));return;}
 const data=await api('/api/loudness',{tracks:ids,settings:config()});mergeTracks(data.tracks);draw();toast(csText("{0}曲の音量・DC解析が完了しました。録音設定で補正をONにできます", ids.length));
});
$('adjust-volume').onclick=()=>task(csText("曲の音量を確認しています…"),async()=>{
 if(selected.size!==1)return;
 gainTrackId=entries()[[...selected][0]].track_id;
 const data=await api('/api/loudness',{tracks:[gainTrackId],settings:config()});mergeTracks(data.tracks);
 const t=track(gainTrackId),l=config().remove_dc?t.dc?.corrected_loudness:t.loudness,report=data.reports[gainTrackId];
 $('gain-title').textContent=t.title;$('manual-gain').value=t.manual_gain_db||0;
 $('gain-measurement').textContent=csText(config().remove_dc?"DC補正後の音量：{0} / ピーク：{1} / 現在の補正：{2}{3}":"元の音量：{0} / ピーク：{1} / 現在の補正：{2}{3}", l?.integrated_lufs!=null?`${l.integrated_lufs.toFixed(1)} LUFS`:csText("測定対象外"), l?.true_peak_db!=null?`${l.true_peak_db.toFixed(1)} dBTP`:csText("測定対象外"), db(report.gain_db), report.limited?csText("（ピーク制限）"):'');
 $('gain-dc-measurement').textContent=t.dc?csText('DCオフセット：L {0}% / R {1}%',(t.dc.offset[0]*100).toFixed(4),(t.dc.offset[1]*100).toFixed(4)):csText('DC未解析');
 draw();$('gain-dialog').showModal();
});
$('gain-form').onsubmit=async e=>{e.preventDefault();const id=gainTrackId,value=+$('manual-gain').value;$('gain-dialog').close();await task(csText("音量補正を保存しています…"),async()=>{
 const t=await api(`/api/tracks/${encodeURIComponent(id)}/gain`,{gain_db:value},'PUT');mergeTracks([t]);
 if(player?.list.some(e=>e.track_id===id))stop();renderCache=null;$('downloads').replaceChildren();draw();toast(csText("曲の音量補正を保存しました"));
});};
$('batch-start').onclick=async()=>{
 const targets=[...$('batch-targets').querySelectorAll('input:checked')].map(el=>el.value);
 if(!targets.length){toast(csText("配置先の空カセットを選択してください"));return;}
 const tracks=[...batchTracks];$('batch-dialog').close();
 await task(csText("カセットごとの録音時間に合わせて、重複しないように配置しています…"),async()=>{
  await saveChain;
  const result=await api('/api/fill-cassettes',{tracks,targets});
  for(const c of result.collections){const i=lib.collections.findIndex(v=>v.id===c.id);lib.collections[i]=c;}
  if(player&&result.collections.some(c=>c.id===player.collection))stop();
  renderCache=null;$('downloads').replaceChildren();selectCollection(result.collections[0].id);
  toast(csText("{0}本に配置しました。未配置：{1}曲 · 収録済みのため除外：{2}曲{3}", result.collections.length, result.remaining, result.excluded, result.collections.length<targets.length?csText("。曲を配置できなかったカセットは空のままです"):''));
 });
};
$('import-btn').onclick=()=>$('file-input').click();$('file-input').onchange=e=>{importFiles([...e.target.files]);e.target.value='';};
let dragDepth=0;
document.addEventListener('dragenter',e=>{if([...e.dataTransfer.types].includes('Files')){e.preventDefault();dragDepth++;$('drop-overlay').style.display='block';}});
document.addEventListener('dragleave',e=>{if([...e.dataTransfer.types].includes('Files')&&--dragDepth<=0){dragDepth=0;$('drop-overlay').style.display='none';}});
document.addEventListener('dragover',e=>e.preventDefault());document.addEventListener('drop',e=>{e.preventDefault();dragDepth=0;$('drop-overlay').style.display='none';if(e.dataTransfer.files.length)importFiles([...e.dataTransfer.files]);});
$('url-btn').onclick=()=>$('url-dialog').showModal();$('url-form').onsubmit=async e=>{e.preventDefault();const url=$('url-input').value,destination=active,s=structuredClone(config());$('url-dialog').close();await task(csText("URLから音声を取得して解析しています…"),async()=>{const t=await api('/api/url',{url,settings:s});lib.tracks.push(t);await attachTracks([t.id],destination);draw();toast(csText("URLの音源を追加しました"));});};
$('tracks').onchange=e=>{if(e.target.type==='checkbox'){const i=+e.target.dataset.index;e.target.checked?selected.add(i):selected.delete(i);draw();}};
$('tracks').onclick=e=>{if(e.target.closest('input'))return;const row=e.target.closest('tr');if(!row)return;const i=+row.dataset.index;if(e.ctrlKey||e.metaKey){selected.has(i)?selected.delete(i):selected.add(i);}else{selected=new Set([i]);}drawSelection();};
$('tracks').ondblclick=e=>{const row=e.target.closest('tr');if(row)startPlayback(+row.dataset.index).catch(e=>toast(e.message));};
$('select-all').onchange=e=>{selected=e.target.checked?new Set(entries().map((_,i)=>i)):new Set();draw();};
$('tracks').ondragstart=e=>{const row=e.target.closest('tr');if(!row||active==='all')return;drag={index:+row.dataset.index,collection:active,side};e.dataTransfer.setData('text/plain',String(drag.index));e.dataTransfer.effectAllowed='move';};
$('tracks').ondragover=e=>{e.preventDefault();document.querySelectorAll('.drag-over').forEach(el=>el.classList.remove('drag-over'));e.target.closest('tr')?.classList.add('drag-over');};
$('tracks').ondragend=()=>{drag=null;document.querySelectorAll('.drag-over').forEach(el=>el.classList.remove('drag-over'));};
$('tracks').ondrop=e=>{if(!drag)return;e.preventDefault();const row=e.target.closest('tr');if(!row||drag.collection!==active)return;const c=current(),to=+row.dataset.index;
 if(c.kind==='playlist'){const [v]=c.tracks.splice(drag.index,1);c.tracks.splice(to,0,v);}else{const [v]=c.sides[drag.side].splice(drag.index,1);c.sides[side].splice(to,0,v);}drag=null;selected.clear();changed();};
$('remove-selected').onclick=()=>{const c=current();if(!c||!selected.size)return;if(c.kind==='playlist')c.tracks=c.tracks.filter((_,i)=>!selected.has(i));else c.sides[side]=c.sides[side].filter((_,i)=>!selected.has(i));selected.clear();changed();};
$('move-side').onclick=()=>{const c=current();if(!isTape())return;c.sides[1-side].push(...c.sides[side].filter((_,i)=>selected.has(i)));c.sides[side]=c.sides[side].filter((_,i)=>!selected.has(i));selected.clear();changed();};
$('add-library').onclick=()=>{$('picker-list').innerHTML=lib.tracks.map(t=>`<label><input type="checkbox" value="${esc(t.id)}"><span>${esc(t.title)} <small> · ${time(duration(t))}</small></span></label>`).join('');$('picker-dialog').showModal();};
$('picker-add').onclick=async()=>{const ids=[...$('picker-list').querySelectorAll('input:checked')].map(el=>el.value);await attachTracks(ids);$('picker-dialog').close();changed();};
$('edit-entry').onclick=()=>{if(selected.size!==1)return;const e=entries()[[...selected][0]];for(const [id,key]of [['entry-offset','offset'],['entry-duration','duration'],['entry-in','fade_in'],['entry-out','fade_out']])$(id).value=key.startsWith('fade_')?Math.round(e[key]*2)/2:e[key];$('entry-dialog').showModal();};
$('entry-form').onsubmit=e=>{e.preventDefault();const entry=entries()[[...selected][0]],candidate={...entry};for(const [id,key]of [['entry-offset','offset'],['entry-duration','duration'],['entry-in','fade_in'],['entry-out','fade_out']])candidate[key]=+$(id).value;if(candidate.offset+candidate.duration>duration(track(entry.track_id))+.001||candidate.fade_in>candidate.duration||candidate.fade_out>candidate.duration){toast(csText("曲の長さとフェードの範囲を確認してください"));return;}Object.assign(entry,candidate);$('entry-dialog').close();changed();};
$('settings-btn').onclick=()=>{fillSettings();$('settings-dialog').returnValue='';$('settings-dialog').showModal();};$('tape-preset').onchange=e=>{if(e.target.value!=='custom')$('side-minutes').value=+e.target.value/2;};$('side-minutes').oninput=()=>$('tape-preset').value='custom';
$('settings-dialog').addEventListener('close',()=>{if($('settings-dialog').returnValue==='save')saveSettings();});
$('reanalyze-btn').onclick=async()=>{if(!await askConfirm(csText("ライブラリ全曲の無音を再解析します。作成済みカセットがある場合は先に削除が必要です。続けますか？")))return;const s=readSettings();$('settings-dialog').close('cancel');await task(csText("ライブラリの無音を再解析しています…"),async()=>{lib=await api('/api/reanalyze',s,'PUT');settings=s;localStorage.setItem('cs-settings',JSON.stringify(s));renderCache=null;draw();toast(csText("無音の再解析が完了しました"));});};
$('export-btn').onclick=()=>task(csText("A・B面をWAVに書き出しています…"),async()=>{const cache=await rendered();showDownloads(cache);toast(csText("書き出しが完了しました。A・B面をそれぞれ保存できます"));});
$('record-btn').onclick=()=>task(csText("デッキ録音用の音声を準備しています…"),async()=>{const cache=await rendered();showDownloads(cache);await loadTape(cache,side,0,false);$('record-detail').textContent=`${cache.name} / SIDE ${side?'B':'A'} / ${time(cache.settings.side_seconds)}`;$('record-start').textContent=csText("5秒後に再生開始");$('record-start').disabled=false;$('record-dialog').showModal();});
$('record-start').onclick=()=>{$('record-start').disabled=true;let remaining=5;$('record-start').textContent=csText("{0}秒後に開始…", remaining);countdown=setInterval(()=>{remaining--;$('record-start').textContent=csText("{0}秒後に開始…", remaining);if(remaining<=0){clearInterval(countdown);countdown=null;$('record-dialog').close();audio.play().then(()=>$('play-btn').textContent='Ⅱ').catch(e=>toast(e.message));}},1000);};
$('record-dialog').addEventListener('close',()=>{if(countdown){clearInterval(countdown);countdown=null;}});
$('play-btn').onclick=async()=>{if(!player){await startPlayback(selected.size?Math.min(...selected):0);return;}if(gapTimer){clearTimeout(gapTimer);gapTimer=null;$('play-btn').textContent='▶';return;}if(audio.paused){if(player.mode==='playlist'){const e=player.list[player.index],t=track(e.track_id);if(audio.currentTime>=t.trim_start+e.offset+e.duration-.03){if(player.index<player.list.length-1){await playPlaylistIndex(player.index+1);return;}audio.currentTime=t.trim_start+e.offset;}}await audio.play();$('play-btn').textContent='Ⅱ';}else{audio.pause();$('play-btn').textContent='▶';}};
$('stop-btn').onclick=stop;
async function skip(delta){if(!player)return;const i=Math.max(0,Math.min(player.list.length-1,player.index+delta));if(player.mode==='playlist')await task(csText("再生を準備しています…"),()=>playPlaylistIndex(i));else{audio.currentTime=startAt(i,player);updatePlayer();}}
$('prev-btn').onclick=()=>skip(-1);$('next-btn').onclick=()=>skip(1);
$('seek').oninput=e=>{if(!player)return;const ratio=+e.target.value/1000;if(player.mode==='tape')audio.currentTime=ratio*(audio.duration||0);else{const entry=player.list[player.index];audio.currentTime=track(entry.track_id).trim_start+entry.offset+ratio*entry.duration;}updatePlayer();};
$('volume').oninput=e=>audio.volume=+e.target.value;
$('output-btn').onclick=async()=>{if(!navigator.mediaDevices?.selectAudioOutput||!audio.setSinkId){toast(csText("このブラウザでは出力先選択に対応していません。OSの音声設定で出力先を選んでください"));return;}try{const device=await navigator.mediaDevices.selectAudioOutput();await audio.setSinkId(device.deviceId);toast(csText("出力先：{0}", device.label||csText("選択したデバイス")));}catch(e){toast(e.message);}};
audio.addEventListener('timeupdate',updatePlayer);audio.addEventListener('ended',()=>{if(player?.mode==='tape'){$('play-btn').textContent='▶';toast(player.side===0?csText("A面が終了しました。テープを裏返し、B面を選んで録音してください"):csText("B面が終了しました"));}});
document.addEventListener('keydown',e=>{if(e.code==='Space'&&!['INPUT','TEXTAREA','SELECT','BUTTON'].includes(e.target.tagName)&&!document.querySelector('dialog[open]')){e.preventDefault();$('play-btn').click();}});
window.addEventListener('unhandledrejection',e=>{e.preventDefault();toast(e.reason?.message||String(e.reason));});
async function init(){await task(csText("ライブラリを読み込んでいます…"),async()=>{lib=await api('/api/library');draw();const s=await api('/api/status');$('tool-status').textContent=s.ffmpeg&&s.ffprobe?csText("● 音声エンジン準備完了{0}", s.yt_dlp?csText(" · URL対応"):''):csText("△ 音声ツールのセットアップが必要");if(!s.ffmpeg||!s.ffprobe)toast(csText("FFmpegが見つかりません。scripts/setup-tools.ps1 またはセットアップ手順を実行してください"));});}
csI18nReady.then(init).catch(error=>toast(error.message));
