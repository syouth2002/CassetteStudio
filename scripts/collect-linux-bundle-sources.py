#!/usr/bin/env python3
"""Download exact Ubuntu source packages corresponding to bundled system libraries."""
import argparse,concurrent.futures,hashlib,json,pathlib,re,subprocess,tarfile,time,urllib.parse,urllib.request
ROOT=pathlib.Path(__file__).resolve().parent.parent

def fetch(url,path,attempts=4):
    if path.exists():return path
    path.parent.mkdir(parents=True,exist_ok=True)
    for attempt in range(attempts):
        try:
            request=urllib.request.Request(url,headers={'User-Agent':'CassetteStudio-source-collector/1'})
            with urllib.request.urlopen(request,timeout=120)as response,path.with_suffix(path.suffix+'.part').open('wb')as stream:
                while chunk:=response.read(1024*1024):stream.write(chunk)
            path.with_suffix(path.suffix+'.part').replace(path)
            return path
        except Exception:
            if attempt==attempts-1:raise
            time.sleep(2*(attempt+1))

def json_url(url):
    for attempt in range(4):
        try:
            with urllib.request.urlopen(url,timeout=90)as response:return json.load(response)
        except Exception:
            if attempt==3:raise
            time.sleep(2*(attempt+1))

def source_job(item,output):
    name,version=item;folder=output/'ubuntu'/name/version.replace(':','_');metadata=folder/'source.json'
    if metadata.exists():return json.loads(metadata.read_text())
    query=urllib.parse.urlencode({'ws.op':'getPublishedSources','source_name':name,'version':version,'exact_match':'true'})
    result=json_url('https://api.launchpad.net/1.0/ubuntu/+archive/primary?'+query)
    if not result['entries']:raise RuntimeError(f'No exact Ubuntu source: {name} {version}')
    entry=result['entries'][0]
    urls=json_url(entry['self_link']+'?ws.op=sourceFileUrls')
    records=[]
    for url in urls:
        filename=urllib.parse.unquote(url.rsplit('/',1)[1])
        prefix=name[:4]if name.startswith('lib')else name[0]
        mirror=f"https://archive.ubuntu.com/ubuntu/pool/{entry['component_name']}/{prefix}/{name}/{filename}"
        try:path=fetch(mirror,folder/filename,attempts=1)
        except Exception:path=fetch(url,folder/filename)
        records.append({'file':str(path.relative_to(output)),'url':url,'sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
    dsc=next((folder/r['file'].split('/')[-1] for r in records if r['file'].endswith('.dsc')),None)
    if not dsc:raise RuntimeError('Missing signed source manifest: '+name)
    text=dsc.read_text()
    block=text.split('Checksums-Sha256:\n',1)[1].split('\n\n',1)[0]
    for line in block.splitlines():
        parts=line.split()
        if len(parts)!=3 or not re.fullmatch('[0-9a-f]{64}',parts[0]):break
        sha,size,filename=parts;path=folder/filename
        if not path.exists()or path.stat().st_size!=int(size)or hashlib.sha256(path.read_bytes()).hexdigest()!=sha:raise RuntimeError('Source checksum mismatch: '+filename)
    result={'package':name,'version':version,'files':records}
    metadata.write_text(json.dumps(result,indent=2)+'\n')
    print('Collected source:',name,version,flush=True)
    return result

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--appdir',type=pathlib.Path,required=True);parser.add_argument('--output',type=pathlib.Path,required=True)
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    script=__import__('runpy').run_path(str(ROOT/'scripts/build-linux-bundles.py'));owner=script['owner']
    packages=set()
    for path in (args.appdir/'usr').rglob('*'):
        if not path.is_file():continue
        if '/libexec/yt-dlp/'in str(path):continue
        try:
            with path.open('rb')as stream:elf=stream.read(4)==b'\x7fELF'
        except OSError:continue
        if not elf:continue
        candidates=[pathlib.Path('/usr/lib/x86_64-linux-gnu')/path.name,pathlib.Path('/usr/bin')/path.name]
        # Also account for GStreamer and WebKit helper/plugin source packages.
        for marker in ['/lib/webkit2gtk-4.1/','/lib/gstreamer-1.0/']:
            if marker in str(path):candidates.append(pathlib.Path('/usr/lib/x86_64-linux-gnu')/str(path).split('/lib/',1)[1])
        for candidate in candidates:
            package=owner(candidate)
            if package:packages.add(package);break
    packages.add('ffmpeg')
    rows=set()
    fields=' '.join(chr(36)+'{'+field+'}'for field in ['source:Package','source:Version'])
    for package in packages:
        name,version=subprocess.check_output(['dpkg-query','-W','-f',fields,package],text=True).split()
        rows.add((name,version))
    print('Exact Ubuntu source packages:',len(rows),flush=True)
    with concurrent.futures.ThreadPoolExecutor(max_workers=12)as pool:
        records=list(pool.map(lambda item:source_job(item,args.output),sorted(rows)))
    (args.output/'ubuntu-sources.json').write_text(json.dumps(records,indent=2)+'\n')
    print('PASS: exact source packages downloaded and checked against source manifest hashes',flush=True)

if __name__=='__main__':main()
