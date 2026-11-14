"""Launch a disposable library for real Rust-canvas browser interaction tests."""
import argparse,json,math,os,pathlib,re,struct,subprocess,tempfile,time,wave
root=pathlib.Path(__file__).resolve().parent.parent
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--engine',type=pathlib.Path,default=root/'target/debug'/('cassette-studio.exe'if os.name=='nt'else'cassette-studio'))
parser.add_argument('--node',default='node')
parser.add_argument('--browser-script',type=pathlib.Path,default=root/'scripts/test-rust-ui-browser.cjs')
parser.add_argument('--output',type=pathlib.Path,default=root/'work/rust-ui-browser')
args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
with tempfile.TemporaryDirectory(prefix='rust-canvas-',dir=root/'work',ignore_cleanup_errors=True)as folder:
    data=pathlib.Path(folder);(data/'sources').mkdir()
    tracks=[]
    names=['Sunlit Walk / 陽だまりの散歩','Paper Clouds / 紙の雲','Blue Hour / 青い時間','Night Train / 夜行列車','Small Lantern / 小さな灯り','Ocean Letters / 海からの手紙','After the Rain / 雨のあと']
    for i,name in enumerate(names):
        id=f'00000000-0000-4000-8000-{i+1:012d}'
        with wave.open(str(data/'sources'/id),'wb')as w:
            w.setnchannels(1);w.setsampwidth(2);w.setframerate(44100)
            w.writeframes(b''.join(struct.pack('<h',round(1800*math.sin(2*math.pi*(220+i*65)*n/44100)))for n in range(44100*8)))
        tracks.append(dict(id=id,title=name,artist='Cassette Studio Demo',filename='demo.wav',source='local',duration=8,trim_start=0,trim_end=8,mono=True))
    def entry(t):return dict(track_id=t['id'],offset=0,duration=8,fade_in=0,fade_out=0,continuation=False)
    settings=dict(side_seconds=26,gap=1)
    cassette=dict(id='00000000-0000-4000-8000-000000000019',name='Evening Sketches',kind='cassette',tracks=[],sides=[[entry(t)for t in tracks[:3]],[entry(t)for t in tracks[3:6]]],settings=settings)
    playlist=dict(id='00000000-0000-4000-8000-000000001902',name='Weekend Collection',kind='playlist',tracks=[t['id']for t in tracks],sides=[[],[]],settings=settings)
    (data/'library.json').write_text(json.dumps(dict(tracks=tracks,collections=[cassette,playlist])),encoding='utf-8')
    with (args.output/'engine.log').open('w+',encoding='utf-8')as log:
        p=subprocess.Popen([str(args.engine.resolve())],env={**os.environ,'CASSETTE_DATA':str(data),'CASSETTE_PORT':'0'},stdout=log,stderr=log,**({'creationflags':0x08000000}if os.name=='nt'else{}))
        try:
            for _ in range(150):
                log.seek(0);m=re.search(r'http://127\.0\.0\.1:\d+',log.read())
                if m:break
                time.sleep(.1)
            else:raise AssertionError('Engine startup timeout')
            subprocess.run([args.node,str(args.browser_script.resolve()),m.group()+'/',str(args.output.resolve())],cwd=root,check=True)
        finally:p.terminate();p.wait(timeout=15)
