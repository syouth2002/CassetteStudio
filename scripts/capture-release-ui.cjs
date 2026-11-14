const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');const {chromium}=require(process.env.CASSETTE_PLAYWRIGHT_MODULE);
(async()=>{const browser=await chromium.launch({headless:true,args:['--use-angle=swiftshader','--enable-unsafe-swiftshader']});try{
const base=process.argv[2],out=process.argv[3];const api=await browser.newPage();let library=await(await api.request.get(base+'api/library')).json();assert.equal(library.collections[0].id,'00000000-0000-4000-8000-000000000019');await api.request.post(base+'api/loudness',{data:{tracks:library.tracks.map(t=>t.id),settings:{side_seconds:26,gap:1,normalize:true,remove_dc:true}}});await api.close();
for(const [locale,lang] of [['ja-JP','ja'],['en-US','en']]){
const p=await browser.newPage({viewport:{width:1280,height:850},locale});const click=async(x,y)=>{await p.mouse.move(x,y);await p.waitForTimeout(100);await p.mouse.click(x,y,{delay:60});await p.waitForTimeout(250)},shot=async name=>{await p.mouse.move(1275,845);await p.screenshot({path:path.join(out,name+'-'+lang+'.jpg'),type:'jpeg',quality:91})};
async function reset(tape=false){await p.goto(base);await p.waitForFunction(()=>!document.getElementById('loading'));await p.waitForTimeout(650);if(tape)await click(90,330)}
await reset();await shot('library');await click(1180,118);await shot('automatic-cassettes');await reset();await click(1200,31);await shot('options');await click(330,415);await shot('reference-tone');
await reset();await p.mouse.click(450,262,{button:'right',delay:60});await p.waitForTimeout(150);await shot('context-menu');await click(500,345);await shot('track-audio');
await reset();await p.mouse.click(450,262,{button:'right',delay:60});await p.waitForTimeout(150);await click(500,464);await p.waitForTimeout(500);await shot('properties');
await reset();await p.mouse.click(450,262,{button:'right',delay:60});await p.waitForTimeout(150);const graph=p.waitForResponse(r=>r.url().includes('/loudness-graph'));await click(500,371);await graph;await p.waitForTimeout(800);await shot('lufs-track');
await reset(true);await shot('cassette');await click(1050,118);await shot('settings');await shot('settings-audio');
await reset(true);await p.mouse.click(450,415,{button:'right',delay:60});await p.waitForTimeout(200);await shot('tape-context');await click(500,616);await shot('entry-fit');
await reset(true);const side=p.waitForResponse(r=>r.url().includes('/loudness-graph'));await click(1018,266);await side;await p.waitForTimeout(650);await shot('lufs-side-a');
await reset(true);await click(350,325);const sideB=p.waitForResponse(r=>r.url().includes('/loudness-graph'));await click(1018,266);await sideB;await p.waitForTimeout(650);await shot('lufs-side-b');
await reset(true);await click(890,266);await p.waitForFunction(()=>document.getElementById('cassette-canvas').hasAttribute('data-editor-viewport'));await p.mouse.dblclick(650,400,{button:'middle',delay:40});await p.waitForTimeout(200);await shot('jcard-outside');await click(85,82);await shot('jcard-inside');await click(30,82);await click(106,155);await p.mouse.dblclick(650,400,{button:'middle',delay:40});await p.waitForTimeout(200);await shot('jcard-rotation');
await reset(true);const render=p.waitForResponse(r=>r.url().endsWith('/api/render'));await click(1180,266);await render;await p.waitForTimeout(700);await shot('recording');
await reset();await p.setViewportSize({width:1280,height:650});await p.mouse.move(1000,480);await p.mouse.wheel(0,500);await p.waitForTimeout(300);await shot('scroll');await p.close();console.log('Screenshots:',lang);
}
}finally{await browser.close()}})().catch(e=>{console.error(e);process.exitCode=1});
