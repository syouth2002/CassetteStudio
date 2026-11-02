#!/usr/bin/env python3
"""Build local x86_64 Flatpak/AppImage packages from the native Ubuntu 24.04 build.

No host FFmpeg or existing library is modified. Downloads are SHA-256 pinned.
"""
import argparse,functools,hashlib,json,os,pathlib,re,shutil,subprocess,tarfile,urllib.request,uuid,zipfile

ROOT=pathlib.Path(__file__).resolve().parent.parent
APP_ID='io.github.syouth2002.CassetteStudio'
VERSION=re.search(r'^version = "([^"]+)"',(ROOT/'Cargo.toml').read_text(),re.M)[1]
LOCK=json.loads((ROOT/'packaging/linux-bundles/tools.lock.json').read_text())
BASE_LIBS=re.compile(r'^(ld-linux|lib(c|m|pthread|dl|rt|resolv|util|nss_[^.]+)\.so)')


def run(args,**kwargs):
    return subprocess.run([str(a)for a in args],check=True,**kwargs)


def deploy_logged(command,build,**kwargs):
    log=build/'linuxdeploy.log'
    with log.open('a')as stream:
        result=subprocess.run([str(a)for a in command],stdout=stream,stderr=subprocess.STDOUT,**kwargs)
    if result.returncode:
        print('\n'.join(log.read_text(errors='replace').splitlines()[-35:]))
        raise RuntimeError('linuxdeploy failed; see '+str(log))
    print('linuxdeploy completed; log: '+str(log),flush=True)


def download(key,cache):
    spec=LOCK[key];file=cache/key
    if not file.exists() or hashlib.sha256(file.read_bytes()).hexdigest()!=spec['sha256']:
        pending=file.with_suffix('.'+str(uuid.uuid4())+'.download')
        with urllib.request.urlopen(spec['url'],timeout=120)as source,pending.open('wb')as dest:
            shutil.copyfileobj(source,dest)
        if hashlib.sha256(pending.read_bytes()).hexdigest()!=spec['sha256']:
            raise RuntimeError('Download checksum mismatch: '+key)
        pending.replace(file)
    return file


@functools.lru_cache(maxsize=1)
def package_files():
    files={}
    for listing in pathlib.Path('/var/lib/dpkg/info').glob('*.list'):
        package=listing.name[:-5]
        for name in listing.read_text(errors='replace').splitlines():
            files[name]=package
    return files


def owner(path):
    files=package_files()
    return files.get(str(path)) or files.get(str(path.resolve()))


def copy_libraries(binaries,dest,notices,provenance):
    dest.mkdir(parents=True,exist_ok=True)
    for binary in binaries:
        text=subprocess.check_output(['ldd',str(binary)],text=True)
        if 'not found' in text:raise RuntimeError(text)
        for line in text.splitlines():
            match=re.search(r'=>\s+(/\S+)',line)
            if not match:continue
            source=pathlib.Path(match[1]);name=source.name
            if BASE_LIBS.match(name):continue
            if not (dest/name).exists():shutil.copy2(source,dest/name)
            package=owner(source)or owner(source.resolve())
            if package and package not in provenance:
                info=subprocess.check_output(['dpkg-query','-W','-f','${binary:Package}\t${Version}\t${source:Package}\t${source:Version}',package],text=True)
                provenance[package]=info
                copyright=pathlib.Path('/usr/share/doc')/package.split(':')[0]/'copyright'
                if copyright.exists():
                    folder=notices/package.replace(':','_');folder.mkdir(parents=True,exist_ok=True)
                    shutil.copy2(copyright,folder/'copyright')


def stage(prefix,binaries,cache):
    for name in ['bin','libexec/cassette-tools','share/cassette-studio/licenses','share/applications','share/icons/hicolor/512x512/apps','share/metainfo']:
        (prefix/name).mkdir(parents=True,exist_ok=True)
    notices=prefix/'share/cassette-studio/licenses';provenance={}
    for source_name,dest_name in [('CassetteStudio','cassette-studio-desktop'),('cassette-studio-browser','cassette-studio-browser')]:
        source=binaries/source_name
        if not source.is_file():raise RuntimeError('Build native Linux binaries first: '+str(source))
        shutil.copy2(source,prefix/'bin'/dest_name);(prefix/'bin'/dest_name).chmod(0o755)
    config=subprocess.check_output(['ffmpeg','-buildconf'],text=True,stderr=subprocess.STDOUT)
    if '--enable-nonfree' in config:raise RuntimeError('Refusing to redistribute a nonfree FFmpeg build')
    (notices/'FFmpeg-build-configuration.txt').write_text(config)
    for tool in ['ffmpeg','ffprobe']:
        source=shutil.which(tool)
        if not source:raise RuntimeError('Build host needs '+tool)
        real=prefix/'libexec/cassette-tools'/tool;shutil.copy2(source,real);real.chmod(0o755)
        copy_libraries([real],prefix/'lib/cassette-tools',notices,provenance)
        (prefix/'bin'/tool).write_text('#!/bin/sh\nset -eu\nbase=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)\nexport LD_LIBRARY_PATH="$base/lib/cassette-tools"\nexec "$base/libexec/cassette-tools/'+tool+'" "$@"\n')
        (prefix/'bin'/tool).chmod(0o755)
        package=owner(pathlib.Path(source))
        if package:
            folder=notices/package.replace(':','_');folder.mkdir(exist_ok=True)
            shutil.copy2('/usr/share/doc/ffmpeg/copyright',folder/'copyright')
    with tarfile.open(download('node',cache))as archive:
        node=next(m for m in archive.getmembers()if m.name.endswith('/bin/node'))
        with archive.extractfile(node)as stream,(prefix/'bin/node').open('wb')as dest:shutil.copyfileobj(stream,dest)
        license=next(m for m in archive.getmembers()if m.name.count('/')==1 and m.name.endswith('/LICENSE'))
        (notices/'Node-LICENSE').write_bytes(archive.extractfile(license).read())
    (prefix/'bin/node').chmod(0o755)
    yt_dir=prefix/'libexec/yt-dlp';yt_dir.mkdir(exist_ok=True)
    with zipfile.ZipFile(download('yt_dlp',cache))as archive:
        for item in archive.infolist():
            target=yt_dir/item.filename
            if not target.resolve().is_relative_to(yt_dir.resolve()):raise RuntimeError('Unsafe yt-dlp archive path')
            if item.is_dir():target.mkdir(parents=True,exist_ok=True);continue
            target.parent.mkdir(parents=True,exist_ok=True)
            with archive.open(item)as source,target.open('wb')as dest:shutil.copyfileobj(source,dest)
    (yt_dir/'yt-dlp_linux').chmod(0o755)
    (prefix/'bin/yt-dlp').write_text('#!/bin/sh\nset -eu\nbase=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)\nexec "$base/libexec/yt-dlp/yt-dlp_linux" "$@"\n')
    (prefix/'bin/yt-dlp').chmod(0o755)
    shutil.copy2(download('yt_dlp_license',cache),notices/'yt-dlp-LICENSE')
    shutil.copy2(download('yt_dlp_notices',cache),notices/'yt-dlp-THIRD-PARTY-LICENSES.txt')
    fonts=prefix/'share/fonts';fonts.mkdir(exist_ok=True)
    for key,name in [('font_jp','NotoSansJP.ttf'),('font_symbols','NotoSansSymbols2.ttf')]:
        shutil.copy2(download(key,cache),fonts/name)
        shutil.copy2(download(key+'_license',cache),notices/(key+'-OFL.txt'))
    shutil.copy2(ROOT/'packaging/linux-bundles/fonts.conf',prefix/'share/cassette-studio/fonts.conf')
    shutil.copy2(ROOT/'packaging/linux-bundles/tools.lock.json',notices/'tools.lock.json')
    # Keep the exact existing Rust notices with the new dependency notices.
    release=ROOT/f'dist/releases/v{VERSION}/linux-x64/licenses'
    if release.exists():shutil.copytree(release,notices/'rust',dirs_exist_ok=True)
    else:run(['python3',ROOT/'scripts/collect-licenses.py','--target','x86_64-unknown-linux-gnu','--output',notices/'rust'])
    for name in ['README.md','README.ja.md','LICENSE','THIRD_PARTY_NOTICES.md']:
        shutil.copy2(ROOT/name,prefix/'share/cassette-studio'/name)
    shutil.copytree(ROOT/'docs',prefix/'share/cassette-studio/docs',dirs_exist_ok=True)
    shutil.copy2(ROOT/'assets/cassette-studio-app.png',prefix/f'share/icons/hicolor/512x512/apps/{APP_ID}.png')
    shutil.copy2(ROOT/f'packaging/linux-bundles/{APP_ID}.desktop',prefix/f'share/applications/{APP_ID}.desktop')
    shutil.copy2(ROOT/f'packaging/linux-bundles/{APP_ID}.metainfo.xml',prefix/f'share/metainfo/{APP_ID}.metainfo.xml')
    return notices,provenance


def flatpak(args,build,cache):
    stage_dir=build/'flatpak-build'
    if stage_dir.exists() and not args.resume:raise RuntimeError('Use a fresh build directory or --resume: '+str(stage_dir))
    if not stage_dir.exists():run(['flatpak','build-init',stage_dir,APP_ID,'org.gnome.Platform','org.gnome.Platform','49'])
    prefix=stage_dir/'files';notices,provenance=stage(prefix,args.binaries,cache)
    xdo=pathlib.Path(subprocess.check_output(['pkg-config','--variable=libdir','libxdo'],text=True).strip())/'libxdo.so.3' if shutil.which('pkg-config') and subprocess.run(['pkg-config','--exists','libxdo']).returncode==0 else pathlib.Path('/usr/lib/x86_64-linux-gnu/libxdo.so.3')
    shutil.copy2(xdo,prefix/'lib/libxdo.so.3')
    xdo_package=owner(xdo)
    if xdo_package:
        folder=notices/xdo_package.replace(':','_');folder.mkdir(exist_ok=True)
        shutil.copy2('/usr/share/doc/libxdo3/copyright',folder/'copyright')
        fields=' '.join(chr(36)+'{'+field+'}' for field in ['binary:Package','Version','source:Package','source:Version'])
        provenance[xdo_package]=subprocess.check_output(['dpkg-query','-W','-f',fields,xdo_package],text=True)
    shutil.copy2(ROOT/'packaging/linux-bundles/launch-flatpak.sh',prefix/'bin/cassette-studio');(prefix/'bin/cassette-studio').chmod(0o755)
    permissions=['--share=network','--share=ipc','--socket=wayland','--socket=fallback-x11','--socket=pulseaudio','--device=dri','--filesystem=host:ro','--env=GTK_USE_PORTAL=1']
    if not (stage_dir/'export').exists():run(['flatpak','build-finish',stage_dir,'--command=cassette-studio',*permissions])
    (notices/'system-packages.json').write_text(json.dumps(provenance,indent=2)+'\n')
    repo=build/'flatpak-repo';run(['flatpak','build-export',repo,stage_dir,'local'])
    output=args.output/f'CassetteStudio-{args.version}-x86_64.flatpak'
    run(['flatpak','build-bundle','--runtime-repo=https://flathub.org/repo/flathub.flatpakrepo',repo,output,APP_ID,'local'])
    print(output)


def appimage(args,build,cache):
    appdir=build/'CassetteStudio.AppDir'
    if appdir.exists() and not args.resume:raise RuntimeError('Use a fresh build directory or --resume: '+str(appdir))
    print('Staging AppImage payload',flush=True)
    prefix=appdir/'usr';notices,provenance=stage(prefix,args.binaries,cache)
    # WebKit launches helpers and GStreamer loads plugins dynamically, outside ldd's graph.
    helpers=pathlib.Path('/usr/lib/x86_64-linux-gnu/webkit2gtk-4.1')
    shutil.copytree(helpers,prefix/'lib/webkit2gtk-4.1',ignore=shutil.ignore_patterns('MiniBrowser'),dirs_exist_ok=True)
    gst=prefix/'lib/gstreamer-1.0';gst.mkdir(exist_ok=True)
    for name in ['coreelements','playback','typefindfunctions','wavparse','audioconvert','audioresample','volume','pulseaudio','autodetect','app','gio','soup']:
        source=pathlib.Path('/usr/lib/x86_64-linux-gnu/gstreamer-1.0')/f'libgst{name}.so'
        shutil.copy2(source,gst/source.name)
    scanner=pathlib.Path('/usr/lib/x86_64-linux-gnu/gstreamer1.0/gstreamer-1.0/gst-plugin-scanner')
    (prefix/'libexec/gstreamer-1.0').mkdir(parents=True,exist_ok=True);shutil.copy2(scanner,prefix/'libexec/gstreamer-1.0/gst-plugin-scanner')
    binaries=list((prefix/'lib/webkit2gtk-4.1').rglob('*'))+list(gst.glob('*.so'))+[prefix/'libexec/gstreamer-1.0/gst-plugin-scanner']
    copy_libraries([p for p in binaries if p.is_file()and p.read_bytes()[:4]==b'\x7fELF'],prefix/'lib',notices,provenance)
    desktop=prefix/f'share/applications/{APP_ID}.desktop';desktop.write_text(desktop.read_text().replace('(Flatpak)','(AppImage)').replace('Exec=cassette-studio','Exec=cassette-studio-desktop'))
    deploy=download('linuxdeploy',cache);deploy.chmod(0o755)
    deploydir=build/'linuxdeploy';deploydir.mkdir(exist_ok=True);run([deploy,'--appimage-extract'],cwd=deploydir,stdout=subprocess.DEVNULL)
    deploybin=deploydir/'squashfs-root/AppRun'
    plugin=deploybin.parent/'linuxdeploy-plugin-gtk.sh';shutil.copy2(download('gtk_plugin',cache),plugin);plugin.chmod(0o755)
    env={**os.environ,'PATH':str(deploybin.parent)+':/usr/local/bin:/usr/bin:/bin','DEPLOY_GTK_VERSION':'3','APPIMAGE_EXTRACT_AND_RUN':'1','ARCH':'x86_64','VERSION':args.version,'OUTPUT':str(args.output/f'CassetteStudio-{args.version}-x86_64.AppImage')}
    deploy_logged([deploybin,'--appdir',appdir,'--executable',prefix/'bin/cassette-studio-desktop','--executable',prefix/'bin/cassette-studio-browser','--desktop-file',desktop,'--icon-file',prefix/f'share/icons/hicolor/512x512/apps/{APP_ID}.png','--plugin','gtk'],build,env=env)
    (notices/'system-packages.json').write_text(json.dumps(provenance,indent=2)+'\n')
    finish_appimage(args,build,appdir)


def finish_appimage(args,build,appdir):
    # A second linuxdeploy pass would replace the custom isolation launcher.
    if (appdir/'AppRun').is_symlink():(appdir/'AppRun').unlink()
    shutil.copy2(ROOT/'packaging/linux-bundles/AppRun',appdir/'AppRun');(appdir/'AppRun').chmod(0o755)
    shutil.copy2(ROOT/'assets/cassette-studio-app.png',appdir/f'{APP_ID}.png')
    deploybin=build/'linuxdeploy/squashfs-root/usr/bin/linuxdeploy-plugin-appimage'
    env={**os.environ,'PATH':'/usr/local/bin:/usr/bin:/bin','APPIMAGE_EXTRACT_AND_RUN':'1','ARCH':'x86_64','VERSION':args.version,'OUTPUT':str(args.output/f'CassetteStudio-{args.version}-x86_64.AppImage')}
    deploy_logged([deploybin,'--appdir',appdir],build,env=env,cwd=build)
    print(env['OUTPUT'])


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('format',choices=['flatpak','appimage'])
    parser.add_argument('--resume',action='store_true',help='Resume an interrupted staging build')
    parser.add_argument('--binaries',type=pathlib.Path,default=ROOT/'dist/linux-x64')
    parser.add_argument('--build-dir',type=pathlib.Path,required=True)
    parser.add_argument('--cache',type=pathlib.Path,default=pathlib.Path('/var/tmp/cassette-bundle-downloads'))
    parser.add_argument('--output',type=pathlib.Path,default=ROOT/'dist/linux-bundles')
    args=parser.parse_args();args.version=VERSION
    if os.uname().machine!='x86_64':raise RuntimeError('This builder currently targets x86_64 only')
    for path in [args.build_dir,args.output,args.cache]:path.mkdir(parents=True,exist_ok=True)
    globals()[args.format](args,args.build_dir.resolve(),args.cache.resolve())


if __name__=='__main__':main()
