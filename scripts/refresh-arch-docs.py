"""Repack native Arch results after guide-only updates; retain verified ELF payloads."""
import pathlib,subprocess,tempfile,shutil,hashlib,re,os
root=pathlib.Path(__file__).resolve().parent.parent
release=root/'dist/releases/v0.2.0';package=release/'cassette-studio-0.2.0-1-x86_64.pkg.tar.zst'
with tempfile.TemporaryDirectory(prefix='cassette-arch-guides-',dir='/var/tmp')as folder:
    stage=pathlib.Path(folder)
    subprocess.run(['bsdtar','-xf',package,'-C',stage],check=True)
    binary=stage/'usr/bin/cassette-studio';before=hashlib.sha256(binary.read_bytes()).hexdigest()
    docs=stage/'usr/share/doc/cassette-studio'
    shutil.copytree(root/'docs',docs/'docs',dirs_exist_ok=True)
    fonts=release/'linux-x64/licenses/epaint_default_fonts-0.33.3/fonts'
    shutil.copytree(fonts,docs/'licenses/epaint_default_fonts-0.33.3/fonts',dirs_exist_ok=True)
    for name in ['README.md','README.ja.md','THIRD_PARTY_NOTICES.md']:shutil.copy2(root/name,docs/name)
    info=stage/'.PKGINFO';text=info.read_text();size=sum(p.stat().st_size for p in (stage/'usr').rglob('*')if p.is_file());info.write_text(re.sub(r'^size = .*$',f'size = {size}',text,flags=re.M))
    info=stage/'.BUILDINFO';text=info.read_text();digest=hashlib.sha256((release/'PKGBUILD').read_bytes()).hexdigest();info.write_text(re.sub(r'^pkgbuild_sha256sum = .*$',f'pkgbuild_sha256sum = {digest}',text,flags=re.M))
    entries=sorted(p.name for p in stage.iterdir()if p.name!='.MTREE')
    subprocess.run(['bsdtar','--uid','0','--gid','0','-czf','.MTREE','--format=mtree','--options=!all,use-set,type,uid,gid,mode,time,size,md5,sha256,link',*entries],cwd=stage,check=True)
    pending=package.with_suffix('.refresh.zst')
    subprocess.run(['bsdtar','--uid','0','--gid','0','--zstd','-cf',pending,*sorted(p.name for p in stage.iterdir())],cwd=stage,check=True)
    assert hashlib.sha256(binary.read_bytes()).hexdigest()==before
    os.replace(pending,package)
    print('Updated Arch guides and mtree; verified native executable unchanged:',before)
