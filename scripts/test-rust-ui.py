"""Check the embedded Rust frontend and non-destructive audition with synthetic audio."""
import hashlib,io,json,math,os,pathlib,re,struct,subprocess,tempfile,time,urllib.request,urllib.error,wave

ROOT=pathlib.Path(__file__).resolve().parent.parent
exe=ROOT/'target/debug'/('cassette-studio.exe'if os.name=='nt'else'cassette-studio')
with tempfile.TemporaryDirectory(prefix='rust-ui-',dir=ROOT/'work',ignore_cleanup_errors=True)as folder:
    data=pathlib.Path(folder)
    with (data/'server.log').open('w+',encoding='utf-8')as log:
        p=subprocess.Popen([str(exe)],env={**os.environ,'CASSETTE_DATA':str(data),'CASSETTE_PORT':'0'},stdout=log,stderr=log,**({'creationflags':0x08000000}if os.name=='nt'else{}))
        try:
            for _ in range(150):
                log.seek(0);m=re.search(r'http://127\.0\.0\.1:\d+',log.read())
                if m:base=m.group();break
                time.sleep(.1)
            else:raise AssertionError('Server did not start')
            def api(route,body=None,method='POST'):
                req=urllib.request.Request(base+route)if body is None else urllib.request.Request(base+route,data=json.dumps(body).encode(),method=method,headers={'Content-Type':'application/json'})
                return json.load(urllib.request.urlopen(req,timeout=120))
            html=urllib.request.urlopen(base).read()
            assert b'cassette-canvas'in html and b'/ui/cassette_ui.js'in html,'Build the Rust UI before this check'
            assert b'\x00asm'==urllib.request.urlopen(base+'/ui/cassette_ui_bg.wasm').read(4)
            assert b'make-auto'in urllib.request.urlopen(base+'/legacy/').read()
            fixture=io.BytesIO()
            with wave.open(fixture,'wb')as w:
                w.setnchannels(1);w.setsampwidth(2);w.setframerate(44100)
                w.writeframes(b''.join(struct.pack('<h',round(2000*math.sin(2*math.pi*440*i/44100)))for i in range(44100*3)))
            body=b'--rust-ui\r\nContent-Disposition: form-data; name="files"; filename="test.wav"\r\nContent-Type: audio/wav\r\n\r\n'+fixture.getvalue()+b'\r\n--rust-ui--\r\n'
            imported=json.load(urllib.request.urlopen(urllib.request.Request(base+'/api/import',data=body,headers={'Content-Type':'multipart/form-data; boundary=rust-ui'}),timeout=120))
            assert not imported['errors'];track=imported['tracks'][0];track_id=track['id'];source=data/'sources'/track_id
            digest=hashlib.sha256(source.read_bytes()).digest()
            settings={'normalize':True,'remove_dc':True,'side_seconds':5,'lead_in':.5}
            before=api('/api/library')
            audition=api('/api/audition/'+track_id,{'settings':settings,'gain_db':3,'strength':2})
            audio=urllib.request.urlopen(base+audition['url']).read()
            with wave.open(io.BytesIO(audio),'rb')as w:
                assert w.getnchannels()==2
                assert any(abs(l-r)>2 for l,r in struct.iter_unpack('<hh',w.readframes(w.getnframes())))
            after=api('/api/library');original=before['tracks'][0];saved=after['tracks'][0]
            for key in ['manual_gain_db','pseudo_stereo','trim_start','trim_end','title']:assert original[key]==saved[key],key
            assert hashlib.sha256(source.read_bytes()).digest()==digest
            try:api('/api/audition/'+track_id,{'settings':settings,'gain_db':100,'strength':2})
            except urllib.error.HTTPError as e:assert e.code==400
            else:raise AssertionError('Invalid gain was accepted')
            print('PASS: embedded Rust WASM/default page, compatibility view, temporary gain/pseudo-stereo audition, invalid request rejection and unchanged source/settings')
        finally:p.terminate();p.wait(timeout=15)
