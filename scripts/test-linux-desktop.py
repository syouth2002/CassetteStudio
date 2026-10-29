import json,sys,time,uuid,urllib.request,urllib.error,struct,wave,math
from pathlib import Path
project=Path(sys.argv[1]);test_dir=Path(sys.argv[2]);base='http://127.0.0.1:3215'
def get(route): return urllib.request.urlopen(base+route,timeout=10)
def post(route,body):
    req=urllib.request.Request(base+route,data=json.dumps(body).encode(),headers={'Content-Type':'application/json'})
    return json.load(urllib.request.urlopen(req,timeout=90))
for attempt in range(100):
    try:
        status=json.load(get('/api/status'));break
    except (OSError,urllib.error.URLError):time.sleep(.2)
else:raise RuntimeError((test_dir/'window.log').read_text())
time.sleep(3)
assert status['version']=='0.1.4' and status['ffmpeg'] and status['ffprobe'],status
assert 'Cassette' in get('/').read().decode()
catalog=json.load(get('/en.json'));assert catalog['すべての曲']=='All tracks'
source=test_dir/'quiet.wav'
with wave.open(str(source),'wb') as fixture:
    fixture.setnchannels(2);fixture.setsampwidth(2);fixture.setframerate(44100)
    fixture.writeframes(b''.join(struct.pack('<hh',sample,sample) for sample in (round(200*math.sin(2*math.pi*440*i/44100)) for i in range(8*44100))))
boundary='cassette-'+uuid.uuid4().hex
body=(f'--{boundary}\r\nContent-Disposition: form-data; name="files"; filename="quiet.wav"\r\nContent-Type: audio/wav\r\n\r\n'.encode()+source.read_bytes()+f'\r\n--{boundary}--\r\n'.encode())
request=urllib.request.Request(base+'/api/import',data=body,headers={'Content-Type':f'multipart/form-data; boundary={boundary}'})
result=json.load(urllib.request.urlopen(request,timeout=90));assert not result['errors'],result
t=(result['tracks'] or [d['track'] for d in result.get('duplicates',[])])[0]
s={'side_seconds':10,'gap':0,'normalize':True,'remove_dc':True,'target_lufs':-18,'peak_db':-1}
t=post('/api/preview/'+t['id'],s)['track']
assert t['loudness']['integrated_lufs']<0 and t['dc']['corrected_loudness']
entry={'track_id':t['id'],'offset':0,'duration':t['trim_end']-t['trim_start'],'fade_in':0,'fade_out':0,'continuation':False}
rendered=post('/api/render',{'sides':[[entry],[]],'settings':s})
audio=get(rendered['urls'][0]).read();assert audio[:4]==b'RIFF';assert len(audio)==44+44100*4*10
preview=post('/api/preview/'+t['id'],s);assert preview['track']['loudness']
result={'passed':True,'platform':'Ubuntu 24.04 x64 / WSL2 / Xvfb','checks':['native window startup','English catalog','audio import','loudness analysis','normalized WAV export','exact duration','preview'],'status':status}
(project/'work/linux-desktop-result.json').write_text(json.dumps(result,indent=2))
print('PASS: Linux native desktop startup, import, loudness, WAV export, preview, English catalog')
