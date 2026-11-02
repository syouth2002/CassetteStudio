#!/usr/bin/env python3
"""Exercise installed Flatpak and an AppImage beside the native build, with synthetic media only.
Run inside dbus-run-session and xvfb-run. Flatpak must be installed for the current user.
"""
import argparse,signal,functools,hashlib,http.server,json,os,pathlib,re,struct,subprocess,tempfile,threading,time,urllib.error,urllib.request,wave,math
from check_lufs_native import check_lufs
ROOT=pathlib.Path(__file__).resolve().parent.parent
APP_ID='io.github.syouth2002.CassetteStudio'

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--appimage',type=pathlib.Path,required=True)
    parser.add_argument('--native',type=pathlib.Path,default=ROOT/'dist/linux-x64/cassette-studio-browser')
    parser.add_argument('--output',type=pathlib.Path,default=ROOT/'work/linux-bundle-test')
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    host_ffmpeg=pathlib.Path('/usr/bin/ffmpeg');host_hash=hashlib.sha256(host_ffmpeg.read_bytes()).digest()
    with tempfile.TemporaryDirectory(prefix='cassette-bundles-',dir='/var/tmp') as folder:
        test=pathlib.Path(folder);processes=[];clients=[]
        fixture=test/'tone.wav'
        with wave.open(str(fixture),'wb')as stream:
            stream.setnchannels(2);stream.setsampwidth(2);stream.setframerate(44100)
            stream.writeframes(b''.join(struct.pack('<hh',v,v)for v in (0 if i<26460 or i>=326340 else round(1000+2000*math.sin(2*math.pi*440*i/44100))for i in range(8*44100))))
        class QuietHandler(http.server.SimpleHTTPRequestHandler):
            def log_message(self,*args):pass
        httpd=http.server.ThreadingHTTPServer(('127.0.0.1',0),functools.partial(QuietHandler,directory=str(test)))
        thread=threading.Thread(target=httpd.serve_forever,daemon=True);thread.start()
        try:
            for label,command,extra in [
                ('native',[str(args.native)],{'CASSETTE_DATA':str(test/'native')}),
                ('appimage',[str(args.appimage.resolve())],{'APPIMAGE_EXTRACT_AND_RUN':'1','XDG_DATA_HOME':str(test/'appimage-data')}),
                ('flatpak',['flatpak','run','--nosocket=wayland','--socket=x11','--filesystem='+str(test)+':rw','--env=CASSETTE_DATA='+str(test/'flatpak-data/cassette-studio-flatpak'),'--env=CASSETTE_PORT=0','--env=GDK_BACKEND=x11',APP_ID+'//local'],{}),
            ]:
                log=(args.output/(label+'.log')).open('w+')
                env={**os.environ,'CASSETTE_PORT':'0','GDK_BACKEND':'x11',**extra}
                # Inherited package overrides must never redirect tests to a personal library.
                if label!='native':
                    for key in ['CASSETTE_DATA','CASSETTE_FFMPEG','CASSETTE_FFPROBE','CASSETTE_YT_DLP']:env.pop(key,None)
                p=subprocess.Popen(command,env=env,stdout=log,stderr=log,start_new_session=True);processes.append((p,log))
                base=None
                for _ in range(300):
                    log.seek(0);text=log.read();m=re.search(r'http://127\.0\.0\.1:\d+',text)
                    if m:base=m.group();break
                    if p.poll() is not None:raise RuntimeError(label+' exited:\n'+text)
                    time.sleep(.1)
                if not base:raise RuntimeError(label+' startup timeout')
                def request(route,body=None,method='POST'):
                    req=urllib.request.Request(base+route)if body is None else urllib.request.Request(base+route,data=json.dumps(body).encode(),method=method,headers={'Content-Type':'application/json'})
                    try:return json.load(urllib.request.urlopen(req,timeout=120))
                    except urllib.error.HTTPError as e:raise AssertionError(e.read().decode())from e
                status=request('/api/status');assert status['ffmpeg']and status['ffprobe']
                if label!='native':assert status['yt_dlp'],status
                boundary='cassette-bundle-test'
                body=(f'--{boundary}\r\nContent-Disposition: form-data; name="files"; filename="{label}-test.wav"\r\nContent-Type: audio/wav\r\n\r\n'.encode()+fixture.read_bytes()+f'\r\n--{boundary}--\r\n'.encode())
                imported=json.load(urllib.request.urlopen(urllib.request.Request(base+'/api/import',data=body,headers={'Content-Type':f'multipart/form-data; boundary={boundary}'}),timeout=120));assert not imported['errors'],imported
                t=imported['tracks'][0];e={'track_id':t['id'],'offset':0,'duration':3,'fade_in':0,'fade_out':.5,'continuation':False}
                settings={'side_seconds':8,'gap':1,'lead_in':.5,'normalize':True,'remove_dc':True,'target_lufs':-18,'wav_sample_rate':48000}
                if label!='native':
                    remote=request('/api/url',{'url':f'http://127.0.0.1:{httpd.server_port}/tone.wav','settings':settings})
                    assert remote['id']==t['id'],'URL import must reuse the same synthetic media'
                c=request('/api/collections',{'id':'','name':label+' test','kind':'cassette','tracks':[],'sides':[[e],[e]],'settings':settings})
                check_lufs(base,t['id'],c['id'])
                result=request('/api/render',{'sides':c['sides'],'settings':settings})
                audio=urllib.request.urlopen(base+result['urls'][0]).read();assert audio[:4]==b'RIFF'and len(audio)==44+8*48000*4
                assert not any(audio[44:44+24000*4])
                card={'version':2,'front':65,'spine':12,'flap':25,'height':100,'paper':'a4','margin':5,'extensions':[],'background':'#ffffff','backBackground':'#ffffff','elements':[],'backElements':[]}
                assert request('/api/jcards/'+c['id'],card,'PUT')==card
                assert request('/api/jcards/'+c['id'])==card
                log.seek(0);text=log.read()
                assert 'Failed to initialize GTK' not in text,text
                assert 'Attempting to add a widget with type WebKitWebView' not in text,text
                data=re.search(r'Data: ([^\n]+)',text)[1].strip()
                expected=test/('native'if label=='native'else label+'-data')
                assert pathlib.Path(data).is_relative_to(expected),(label,data,expected)
                if label!='native':assert data.endswith('/cassette-studio-'+label),(label,data)
                clients.append({'format':label,'url':base,'data':data,'track':t['id']})
                print('PASS:',label,'startup, bundled tools, isolated data, import, DC/LUFS, 48 kHz WAV, leading silence and J-CARD storage',flush=True)
            assert len({x['url']for x in clients})==3 and len({x['data']for x in clients})==3
            for client in clients:
                lib=json.load(urllib.request.urlopen(client['url']+'/api/library'))
                assert len(lib['tracks'])==1 and lib['tracks'][0]['id']==client['track']
            assert hashlib.sha256(host_ffmpeg.read_bytes()).digest()==host_hash
            # Capture the actual rendered desktop. No personal window/display is used.
            try:
                import gi;gi.require_version('Gdk','3.0');from gi.repository import Gdk
                time.sleep(3);window=Gdk.get_default_root_window();pixbuf=Gdk.pixbuf_get_from_window(window,0,0,window.get_width(),window.get_height());pixbuf.savev(str(args.output/'desktop.png'),'png',[],[])
            except ImportError:pass
            (args.output/'result.json').write_text(json.dumps({'passed':True,'clients':clients,'checks':['three simultaneous processes','separate persistent stores','distinct random ports','host FFmpeg unchanged','import/DC/LUFS/WAV/J-CARD']},indent=2))
            print('PASS: native/Flatpak/AppImage coexist, distinct ports/libraries and unchanged host FFmpeg',flush=True)
        finally:
            httpd.shutdown();httpd.server_close();thread.join(timeout=5)
            for p,log in reversed(processes):
                try:os.killpg(p.pid,signal.SIGTERM)
                except ProcessLookupError:pass
                try:p.wait(timeout=15)
                except subprocess.TimeoutExpired:p.kill();p.wait()
                log.close()

if __name__=='__main__':main()
