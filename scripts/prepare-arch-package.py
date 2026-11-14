"""Create a checksummed source snapshot including the generated common WASM UI."""
import hashlib,pathlib,subprocess,tarfile,io,tomllib,os
root=pathlib.Path(__file__).resolve().parent.parent
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
output=root/'dist/arch';output.mkdir(parents=True,exist_ok=True)
source=output/f'CassetteStudio-{version}-Arch-source.tar.gz'
prefix=f'cassette-studio-{version}/'
archive=subprocess.check_output(['git','archive','--format=tar','--prefix='+prefix,'HEAD'],cwd=root)
with tarfile.open(fileobj=io.BytesIO(archive))as original,tarfile.open(source,'w:gz')as target:
    for member in original:
        target.addfile(member,original.extractfile(member)if member.isfile()else None)
    for name in ['cassette_ui.js','cassette_ui_bg.wasm']:
        asset=pathlib.Path(os.environ.get('CASSETTE_UI_ASSETS',str(root/'ui/web/pkg')))/name
        if not asset.is_file():raise RuntimeError('Build the common WASM UI first: '+str(asset))
        target.add(asset,arcname=prefix+'ui/web/pkg/'+name)
digest=hashlib.sha256(source.read_bytes()).hexdigest()
template=(root/'packaging/arch/PKGBUILD.in').read_text()
(output/'PKGBUILD').write_text(template.replace('@SOURCE_SHA256@',digest))
print('Prepared Arch source snapshot and checksummed PKGBUILD:',digest)
