"""Collect exact dependency notices for an application-only binary release."""
import argparse,json,pathlib,shutil,subprocess,urllib.request,urllib.error,re,io,zipfile,hashlib

parser=argparse.ArgumentParser()
parser.add_argument('--target',required=True)
parser.add_argument('--output',required=True)
args=parser.parse_args()
output=pathlib.Path(args.output);output.mkdir(parents=True,exist_ok=True)
metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--all-features','--filter-platform',args.target,'--format-version','1']))
rows=[]
for package in sorted(metadata['packages'],key=lambda p:(p['name'],p['version'])):
    if package['source'] is None:continue
    root=pathlib.Path(package['manifest_path']).parent
    folder=output/(package['name']+'-'+package['version']);folder.mkdir(exist_ok=True)
    files=[p for p in root.rglob('*') if p.is_file() and p.name.upper().startswith(('LICENSE','LICENCE','COPYING','COPYRIGHT','NOTICE','AUTHORS'))]
    if package.get('license_file'):files.append(root/package['license_file'])
    for file in set(files):
        relative=file.relative_to(root);dest=folder/relative;dest.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(file,dest)
    if not files:
        # Some workspace crates omit the repository's license when published.
        # Retrieve root notices from their exact source commit, never a branch.
        repo=(package.get('repository') or '').removesuffix('.git').rstrip('/')
        if not re.fullmatch(r'https://github.com/[\w.-]+/[\w.-]+',repo):
            raise RuntimeError('Missing license text for '+package['name'])
        commit=json.loads((root/'.cargo_vcs_info.json').read_text())['git']['sha1']
        found=False
        for name in ['LICENSE','LICENSE-MIT','LICENSE-APACHE','LICENSE.md','LICENSE.txt','NOTICE']:
            url=f'https://raw.githubusercontent.com/{repo.removeprefix("https://github.com/")}/{commit}/{name}'
            try:body=urllib.request.urlopen(url,timeout=30).read()
            except urllib.error.HTTPError as error:
                if error.code==404:continue
                raise
            (folder/name).write_bytes(body);found=True
        if not found:raise RuntimeError('Missing upstream license text for '+package['name'])
    if 'MPL' in (package.get('license') or ''):
        shutil.copytree(root,folder/'source',dirs_exist_ok=True)
    rows.append({'name':package['name'],'version':package['version'],'license':package['license'],'repository':package.get('repository')})
if args.target.endswith('windows-msvc'):
    package=next(p for p in metadata['packages'] if p['name']=='webview2-com-sys')
    if package['version']!='0.39.1':raise RuntimeError('Recheck the WebView2 SDK version for this crate release')
    sdk='1.0.3800.47'
    url=f'https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/{sdk}/microsoft.web.webview2.{sdk}.nupkg'
    archive=zipfile.ZipFile(io.BytesIO(urllib.request.urlopen(url,timeout=60).read()))
    loader=next(n for n in archive.namelist() if n.endswith('/x64/WebView2LoaderStatic.lib'))
    local=pathlib.Path(package['manifest_path']).parent/'x64/WebView2LoaderStatic.lib'
    if hashlib.sha256(archive.read(loader)).digest()!=hashlib.sha256(local.read_bytes()).digest():
        raise RuntimeError('WebView2 SDK loader does not match the compiled dependency')
    folder=output/f'microsoft-webview2-sdk-{sdk}';folder.mkdir(exist_ok=True)
    for name in ['LICENSE.txt','NOTICE.txt']:(folder/name).write_bytes(archive.read(name))
    (folder/'source-url.txt').write_text(url+'\n',encoding='utf-8')
sysroot=pathlib.Path(subprocess.check_output(['rustc','--print','sysroot'],text=True).strip())
rustdocs=sysroot/'share/doc/rust';rust=output/'rust-standard-library';rust.mkdir(exist_ok=True)
for name in ['COPYRIGHT-library.html','COPYRIGHT.html']:
    if (rustdocs/name).is_file():shutil.copyfile(rustdocs/name,rust/name)
if (rustdocs/'licenses').is_dir():shutil.copytree(rustdocs/'licenses',rust/'licenses',dirs_exist_ok=True)
if not (rust/'COPYRIGHT-library.html').exists() and sysroot==pathlib.Path('/usr'):
    # Arch's rust package keeps notices separately from rustup's doc layout.
    for name in ['COPYRIGHT-library.html','COPYRIGHT.html']:
        notice=pathlib.Path('/usr/share/licenses/rust')/(name+'.rustc')
        if notice.is_file():shutil.copyfile(notice,rust/name)
    license_dir=pathlib.Path('/usr/share/doc/rustc/licenses')
    if license_dir.is_dir():shutil.copytree(license_dir,rust/'licenses',dirs_exist_ok=True)
if not (rust/'COPYRIGHT-library.html').exists():raise RuntimeError('Missing Rust standard library copyright notices')
(output/'dependencies.json').write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
(output/'README.txt').write_text('Third-party notices for the bundled Rust application.\nExact crate versions and license expressions: dependencies.json\nMPL-licensed crate source is included in the corresponding source/ folder.\nExternal audio tools and OS runtimes are not bundled.\n\nRustアプリ本体に含まれる依存関係のライセンスです。\nバージョンとライセンスはdependencies.jsonを参照してください。\nMPL対象のクレートにはsource/を同梱しています。音声ツールやOSランタイムは別途導入します。\n',encoding='utf-8')
print(f'Collected notices for {len(rows)} crates and the Rust standard library.')
