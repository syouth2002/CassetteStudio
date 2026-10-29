"""Create a source snapshot and a concrete PKGBUILD with its SHA-256."""
import hashlib,pathlib,subprocess
root=pathlib.Path(__file__).resolve().parent.parent
output=root/'dist/arch';output.mkdir(parents=True,exist_ok=True)
source=output/'CassetteStudio-0.1.4-Arch-source.tar.gz'
subprocess.run(['git','archive','--format=tar.gz','--prefix=cassette-studio-0.1.4/',
                '-o',str(source),'HEAD'],cwd=root,check=True)
digest=hashlib.sha256(source.read_bytes()).hexdigest()
template=(root/'packaging/arch/PKGBUILD.in').read_text()
(output/'PKGBUILD').write_text(template.replace('@SOURCE_SHA256@',digest))
print('Prepared Arch source snapshot and checksummed PKGBUILD:',digest)
