"""Exercise the native Rust HTTP upload against a disposable real audio engine."""
import os,pathlib,re,subprocess,tempfile,time
root=pathlib.Path(__file__).resolve().parent.parent
engine=root/'target/debug'/('cassette-studio.exe'if os.name=='nt'else'cassette-studio')
with tempfile.TemporaryDirectory(prefix='native-upload-',dir=root/'work',ignore_cleanup_errors=True)as folder:
    data=pathlib.Path(folder)
    with (data/'engine.log').open('w+',encoding='utf-8')as log:
        p=subprocess.Popen([str(engine)],cwd=root,env={**os.environ,'CASSETTE_DATA':str(data),'CASSETTE_PORT':'0'},stdout=log,stderr=log,**({'creationflags':0x08000000}if os.name=='nt'else{}))
        try:
            for _ in range(150):
                log.seek(0);match=re.search(r'http://127\.0\.0\.1:\d+',log.read())
                if match:break
                if p.poll()is not None:raise RuntimeError('Engine exited during startup')
                time.sleep(.1)
            else:raise RuntimeError('Engine startup timeout')
            subprocess.run(['cargo','test','-p','cassette-ui','--lib','transport::tests::native_unicode_media_import','--','--ignored','--nocapture'],cwd=root,env={**os.environ,'CASSETTE_TEST_BASE':match.group()},check=True)
        finally:p.terminate();p.wait(timeout=15)
