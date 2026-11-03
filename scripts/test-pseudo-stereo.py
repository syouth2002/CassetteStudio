"""Synthetic full-path checks: mono-only eligibility, stereo output and mono compatibility."""
import hashlib,json,math,os,pathlib,struct,subprocess,tempfile,time,urllib.error,urllib.request,wave
ROOT=pathlib.Path(__file__).resolve().parent.parent

def main():
    with tempfile.TemporaryDirectory(prefix='pseudo-stereo-',dir=ROOT/'work',ignore_cleanup_errors=True)as folder:
        root=pathlib.Path(folder);data=root/'data';(data/'sources').mkdir(parents=True)
        def fixture(name,channels,sample,seconds=4):
            path=root/(name+'.wav')
            with wave.open(str(path),'wb')as stream:
                stream.setnchannels(channels);stream.setsampwidth(2);stream.setframerate(44100)
                stream.writeframes(b''.join(struct.pack('<h',round(32767*sample(i/44100,c)))for i in range(seconds*44100)for c in range(channels)))
            return path
        signal=lambda t:.09*math.sin(2*math.pi*440*t)+.06*math.sin(2*math.pi*1200*t)
        mono=fixture('mono',1,lambda t,c:signal(t))
        dual=fixture('dual',2,lambda t,c:signal(t))
        stereo=fixture('stereo',2,lambda t,c:.1*math.sin(2*math.pi*(440+440*c)*t))
        late=fixture('late-stereo',2,lambda t,c:signal(t)if t<3 else .1*math.sin(2*math.pi*(440+440*c)*t))
        anti=fixture('antiphase',2,lambda t,c:signal(t)*(-1 if c else 1))
        silent=fixture('silence',2,lambda t,c:0)
        multi=fixture('surround',6,lambda t,c:signal(t))
        biased=fixture('biased-mono',1,lambda t,c:signal(t)+.05)
        legacy={'id':'legacy','title':'Legacy mono','artist':'Synthetic','filename':'legacy.wav','source':'local','duration':4,'trim_start':0,'trim_end':4}
        legacy_bytes=bytearray(mono.read_bytes()+b'JUNK'+bytes(4))
        struct.pack_into('<I',legacy_bytes,4,len(legacy_bytes)-8)
        (data/'sources/legacy').write_bytes(legacy_bytes)
        (data/'library.json').write_text(json.dumps({'tracks':[legacy,{**legacy,'id':'missing-source'}],'collections':[]}))
        hashes={p:hashlib.sha256(p.read_bytes()).hexdigest()for p in [mono,dual,stereo,late,anti,silent,multi,biased]}
        executable=ROOT/'target/debug'/('cassette-studio.exe'if os.name=='nt'else'cassette-studio')
        log=(root/'server.log').open('w+',encoding='utf-8')
        server=subprocess.Popen([str(executable)],env={**os.environ,'CASSETTE_DATA':str(data),'CASSETTE_PORT':'0'},stdout=log,stderr=log,creationflags=0x08000000 if os.name=='nt'else 0)
        try:
            import re
            base=None
            for _ in range(150):
                log.seek(0);match=re.search(r'http://127\.0\.0\.1:\d+',log.read())
                if match:base=match.group();break
                time.sleep(.1)
            assert base,'Server did not start'
            def api(route,body=None,method='POST'):
                req=urllib.request.Request(base+route)if body is None else urllib.request.Request(base+route,data=json.dumps(body).encode(),headers={'Content-Type':'application/json'},method=method)
                return json.load(urllib.request.urlopen(req,timeout=120))
            settings={'side_seconds':4,'gap':0,'normalize':False,'remove_dc':False,'peak_db':-1}
            def upload(path):
                body=b'--fixture\r\nContent-Disposition: form-data; name="files"; filename="'+path.name.encode()+b'"\r\nContent-Type: audio/wav\r\n\r\n'+path.read_bytes()+b'\r\n--fixture--\r\n'
                result=json.load(urllib.request.urlopen(urllib.request.Request(base+'/api/import',data=body,headers={'Content-Type':'multipart/form-data; boundary=fixture'}),timeout=120))
                assert not result['errors'],result
                return (result['tracks']or [d['track']for d in result['duplicates']])[0]
            classified=api('/api/mono',{'tracks':['legacy','missing-source']})
            assert classified['errors']==1
            assert next(x for x in classified['tracks']if x['id']=='legacy')['mono']is True
            saved_legacy=next(x for x in api('/api/library')['tracks']if x['id']=='legacy')
            assert saved_legacy['mono']is True and saved_legacy['dc']is None and saved_legacy['loudness']is None
            tracks=[]
            for path,expected in [(mono,True),(dual,True),(stereo,False),(late,False),(anti,False),(silent,False),(multi,False),(biased,True)]:
                t=upload(path);assert t['mono']is expected,(path,t)
                assert t['pseudo_stereo']==0;tracks.append(t)
            for t in tracks[2:7]:
                try:api('/api/tracks/'+t['id']+'/pseudo-stereo',{'strength':2},'PUT')
                except urllib.error.HTTPError as error:assert error.code==400
                else:raise AssertionError('Stereo source accepted')
            migrated=api('/api/loudness',{'tracks':['legacy'],'settings':settings})['tracks'][0]
            assert migrated['mono']is True and migrated['pseudo_stereo']==0
            t=tracks[0];route='/api/tracks/'+t['id']+'/pseudo-stereo'
            def pcm(url):
                path=root/'render.wav';path.write_bytes(urllib.request.urlopen(base+url).read())
                with wave.open(str(path),'rb')as stream:
                    assert stream.getnchannels()==2 and stream.getframerate()==44100
                    raw=stream.readframes(stream.getnframes())
                return list(struct.iter_unpack('<hh',raw))
            raw=api('/api/preview/'+t['id'],settings);original=pcm(raw['url'])
            previous=raw['url']
            for strength in [1,2,3]:
                saved=api(route,{'strength':strength},'PUT');assert saved['pseudo_analysis']['strength']==strength
                processed=api('/api/preview/'+t['id'],settings);assert processed['url']!=previous;previous=processed['url']
                output=pcm(processed['url']);assert len(output)==len(original)
                assert sum((a-b)**2 for a,b in output)>100000
                report=api('/api/loudness',{'tracks':[t['id']],'settings':settings})['reports'][t['id']]
                scale=10**(report['gain_db']/20)
                assert max(abs((a+b)/2-ref[0]*scale)for (a,b),ref in zip(output,original))<2.1,'Mono fold-down changed'
                entry={'track_id':t['id'],'offset':0,'duration':4,'fade_in':0,'fade_out':0,'continuation':False}
                result=api('/api/render',{'sides':[[entry],[entry]],'settings':settings})
                assert pcm(result['urls'][0])==output and pcm(result['urls'][1])==output
            graph=api('/api/tracks/'+t['id']+'/loudness-graph',{'settings':settings,'offset':0,'duration':4,'fade_in':0,'fade_out':0})
            assert graph['processed']['integrated_lufs']is not None
            cassette=api('/api/collections',{'id':'','name':'Synthetic pseudo cassette','kind':'cassette','tracks':[],'sides':[[entry],[entry]],'settings':settings})
            for side in [0,1]:
                whole=api('/api/collections/'+cassette['id']+'/loudness-graph',{'side':side})['graph']
                assert whole['original']['integrated_lufs']==graph['original']['integrated_lufs']
                assert abs(whole['processed']['integrated_lufs']-graph['processed']['integrated_lufs'])<.2
                assert whole['original']['true_peak_db']!=whole['processed']['true_peak_db']
            biased_track=tracks[-1]
            api('/api/tracks/'+biased_track['id']+'/pseudo-stereo',{'strength':2},'PUT')
            corrected_settings={**settings,'normalize':True,'remove_dc':True,'target_lufs':-18,'peak_db':-12}
            corrected=api('/api/preview/'+biased_track['id'],corrected_settings)
            frames=pcm(corrected['url'])
            for channel in [0,1]:
                assert abs(sum(frame[channel]for frame in frames)/len(frames))<3
            audio_file=root/'render.wav'
            ffmpeg=os.environ.get('CASSETTE_FFMPEG') or (str(ROOT/'tools/ffmpeg.exe')if os.name=='nt'else'ffmpeg')
            measured=subprocess.run([ffmpeg,'-hide_banner','-nostats','-i',str(audio_file),'-af','loudnorm=I=-18:TP=-1:LRA=50:print_format=json','-f','null','-'],capture_output=True,text=True,check=True,creationflags=0x08000000 if os.name=='nt'else 0).stderr
            measured=json.loads(measured[measured.rfind('{'):measured.rfind('}')+1])
            assert float(measured['input_tp'])<=-12.0,measured
            assert float(measured['input_i'])<=-17.7,measured
            # Reanalysis must preserve the user's effect and refresh its measurements.
            refreshed=api('/api/reanalyze',{'tracks':[t['id']],'settings':settings},'PUT')
            saved=next(x for x in refreshed['tracks']if x['id']==t['id'])
            assert saved['pseudo_stereo']==3 and saved['pseudo_analysis']['strength']==3
            api(route,{'strength':0},'PUT')
            restored=api('/api/preview/'+t['id'],settings);assert pcm(restored['url'])==original
            stored=json.loads((data/'library.json').read_text())
            assert next(x for x in stored['tracks']if x['id']==t['id'])['pseudo_stereo']==0
            for path,sha in hashes.items():assert hashlib.sha256(path.read_bytes()).hexdigest()==sha
            print('PASS: mono/dual-mono detection, late stereo/antiphase/surround rejection, legacy migration, strengths, mono fold-down, preview/A-B WAV parity, LUFS, reanalysis, off restoration, unchanged sources')
        finally:
            server.terminate();server.wait(timeout=15);log.close()

if __name__=='__main__':main()
