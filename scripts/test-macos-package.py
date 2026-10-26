"""Check the packaged native app and CLI using an isolated synthetic library."""
import json,os,pathlib,subprocess,sys,tempfile,time,urllib.request,wave,math,struct
stage=pathlib.Path(sys.argv[1]).resolve()
app=stage/'Cassette Studio.app'
subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
with tempfile.TemporaryDirectory(prefix='cassette-macos-') as folder:
    root=pathlib.Path(folder)
    media=root/'tone.wav'
    with wave.open(str(media),'wb') as audio:
        audio.setnchannels(2);audio.setsampwidth(2);audio.setframerate(44100)
        audio.writeframes(b''.join(struct.pack('<hh',v,v) for v in
            (round(1000*math.sin(2*math.pi*400*i/44100)) for i in range(44100))))
    for executable in [app/'Contents/MacOS/CassetteStudio',stage/'cassette-studio-browser']:
        with (root/'server.log').open('w+') as log:
            env={**os.environ,'CASSETTE_DATA':str(root/'data'),'CASSETTE_PORT':'0'}
            process=subprocess.Popen([str(executable)],env=env,stdout=log,stderr=log)
            try:
                base=None
                for _ in range(150):
                    log.seek(0)
                    for line in log:
                        if 'http://127.0.0.1:' in line:base=line[line.index('http://'):].strip().rstrip('/')
                    if base:break
                    if process.poll() is not None:raise RuntimeError('App exited: '+(root/'server.log').read_text())
                    time.sleep(.2)
                assert base,(root/'server.log').read_text()
                status=json.load(urllib.request.urlopen(base+'/api/status'))
                assert status['ffmpeg'] and status['ffprobe'],status
                assert b'options-dialog' in urllib.request.urlopen(base).read()
                assert b'referenceToneWav' in urllib.request.urlopen(base+'/reference-tone.js').read()
                boundary='cassette-macos-fixture'
                body=(f'--{boundary}\r\nContent-Disposition: form-data; name="files"; filename="tone.wav"\r\nContent-Type: audio/wav\r\n\r\n'.encode()+media.read_bytes()+f'\r\n--{boundary}--\r\n'.encode())
                request=urllib.request.Request(base+'/api/import',data=body,headers={'Content-Type':f'multipart/form-data; boundary={boundary}'})
                result=json.load(urllib.request.urlopen(request,timeout=60));assert not result['errors'],result
                track=result['tracks'][0]
                settings={'side_seconds':2,'gap':0,'fade':.5,'remove_dc':True}
                entry={'track_id':track['id'],'offset':0,'duration':.8,'fade_in':0,'fade_out':.5,'continuation':False}
                request=urllib.request.Request(base+'/api/render',data=json.dumps({'sides':[[entry],[]],'settings':settings}).encode(),headers={'Content-Type':'application/json'})
                rendered=json.load(urllib.request.urlopen(request,timeout=60))
                data=urllib.request.urlopen(base+rendered['urls'][0]).read()
                assert data[:4]==b'RIFF' and len(data)==44+2*44100*4
                print('PASS:',executable.name,status['version'],'startup, import, DC/fade WAV, embedded UI')
            finally:
                process.terminate()
                try:process.wait(timeout=15)
                except subprocess.TimeoutExpired:process.kill();process.wait()
