#!/usr/bin/env python3
"""Verify and package the exact corresponding sources for bundled Linux libraries."""
import argparse,hashlib,json,pathlib,re,shutil,tarfile,urllib.request
ROOT=pathlib.Path(__file__).resolve().parent.parent

def digest(path):
    with path.open('rb')as stream:return hashlib.file_digest(stream,'sha256').hexdigest()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sources',type=pathlib.Path,required=True)
    parser.add_argument('--output',type=pathlib.Path,required=True)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    records=json.loads((args.sources/'ubuntu-sources.json').read_text())
    files={pathlib.Path('ubuntu-sources.json')}
    for package in records:
        for item in package['files']:
            path=args.sources/item['file']
            if not path.resolve().is_relative_to(args.sources.resolve()):raise RuntimeError('Unsafe source path')
            if digest(path)!=item['sha256']:raise RuntimeError('Source checksum mismatch: '+str(path))
            files.add(pathlib.Path(item['file']))
    upstream=json.loads((ROOT/'packaging/linux-bundles/sources.lock.json').read_text())
    for item in upstream:
        path=args.sources/'upstream'/item['file'];path.parent.mkdir(parents=True,exist_ok=True)
        if not path.exists():
            pending=path.with_suffix(path.suffix+'.part')
            with urllib.request.urlopen(item['url'],timeout=120)as response,pending.open('wb')as stream:shutil.copyfileobj(response,stream)
            pending.replace(path)
        if digest(path)!=item['sha256']:raise RuntimeError('Upstream checksum mismatch: '+str(path))
        files.add(pathlib.Path('upstream')/item['file'])
    version=re.search(r'^version = "([^"]+)"',(ROOT/'Cargo.toml').read_text(),re.M)[1]
    root=f'CassetteStudio-{version}-Linux-bundle-sources'
    output=args.output/(root+'.tar.gz')
    with tarfile.open(output,'w:gz',compresslevel=1)as archive:
        for relative in sorted(files):archive.add(args.sources/relative,arcname=root+'/'+relative.as_posix())
        for relative in ['docs/linux-bundle-sources.md','scripts/build-linux-bundles.py','scripts/collect-linux-bundle-sources.py','scripts/package-linux-bundle-sources.py','packaging/linux-bundles']:
            archive.add(ROOT/relative,arcname=root+'/build/'+relative)
    if output.stat().st_size>=2_000_000_000:raise RuntimeError('Source archive exceeds GitHub asset limit')
    print('PASS: source hashes verified and archived:',output,flush=True)

if __name__=='__main__':main()
