"""Exercise the installed Arch native app and browser engine with synthetic media."""
import json,os,pathlib,subprocess,sys,tempfile,time,urllib.request,wave,math,struct,re
assert pathlib.Path('/etc/arch-release').exists()
with tempfile.TemporaryDirectory(prefix='cassette-arch-') as folder:
    root=pathlib.Path(folder)
    media=root/'tone.wav'
    with wave.open(str(media),'wb') as audio:
        audio.setnchannels(2);audio.setsampwidth(2);audio.setframerate(44100)
        audio.writeframes(b''.join(struct.pack('<hh',v,v) for v in
            (round(1000*math.sin(2*math.pi*400*i/44100)) for i in range(44100))))
    for executable in ['/usr/bin/cassette-studio','/usr/bin/cassette-studio-browser']:
        with (root/'server.log').open('w+') as log:
            env={**os.environ,'CASSETTE_DATA':str(root/'data'),'CASSETTE_PORT':'0','GDK_BACKEND':'x11'}
            process=subprocess.Popen([executable],env=env,stdout=log,stderr=log)
            try:
                base=None
                for _ in range(150):
                    log.seek(0);match=re.search(r'http://127\.0\.0\.1:\d+',log.read())
                    if match:base=match.group();break
                    if process.poll() is not None:raise RuntimeError('App exited: '+(root/'server.log').read_text())
                    time.sleep(.2)
                assert base,(root/'server.log').read_text()
                status=json.load(urllib.request.urlopen(base+'/api/status'))
                assert status['version']=='0.1.3' and status['ffmpeg'] and status['ffprobe'],status
                assert b'options-dialog' in urllib.request.urlopen(base).read()
                assert b'referenceToneWav' in urllib.request.urlopen(base+'/reference-tone.js').read()
                boundary='cassette-arch-fixture'
                body=(f'--{boundary}\r\nContent-Disposition: form-data; name="files"; filename="tone.wav"\r\nContent-Type: audio/wav\r\n\r\n'.encode()+media.read_bytes()+f'\r\n--{boundary}--\r\n'.encode())
                request=urllib.request.Request(base+'/api/import',data=body,headers={'Content-Type':f'multipart/form-data; boundary={boundary}'})
                result=json.load(urllib.request.urlopen(request,timeout=60));assert not result['errors'],result
                track=result['tracks'][0]
                entry={'track_id':track['id'],'offset':0,'duration':.8,'fade_in':0,'fade_out':.5,'continuation':False}
                for rate in [None,44100,48000]:
                    settings={'side_seconds':2,'gap':0,'fade':.5,'remove_dc':True,'lead_in':.5,'wav_sample_rate':rate}
                    request=urllib.request.Request(base+'/api/render',data=json.dumps({'sides':[[entry],[entry]],'settings':settings}).encode(),headers={'Content-Type':'application/json'})
                    rendered=json.load(urllib.request.urlopen(request,timeout=60))
                    for url in rendered['urls']:
                        data=urllib.request.urlopen(base+url).read()
                        actual=rate or 44100
                        assert data[:4]==b'RIFF' and len(data)==44+2*actual*4
                        assert struct.unpack_from('<I',data,24)[0]==actual
                        assert not any(data[44:44+actual*2]),'Leading silence missing'
                print('PASS:',executable,'startup, import, DC/fade WAV, A/B leading silence, DAT formats, embedded UI')
            finally:
                process.terminate()
                try:process.wait(timeout=15)
                except subprocess.TimeoutExpired:process.kill();process.wait()
