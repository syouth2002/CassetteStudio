"""Refresh guides outside signed app bundles without changing executable bytes."""
import argparse,pathlib,zipfile,os
root=pathlib.Path(__file__).resolve().parent.parent
parser=argparse.ArgumentParser();parser.add_argument('archives',nargs='+',type=pathlib.Path);args=parser.parse_args()
for archive in args.archives:
    pending=archive.with_suffix('.refresh.zip')
    with zipfile.ZipFile(archive)as original,zipfile.ZipFile(pending,'w',zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False)as target:
        top=original.namelist()[0].split('/')[0]
        replacements={}
        for name in ['README.md','README.ja.md','LICENSE','THIRD_PARTY_NOTICES.md']:replacements[top+'/'+name]=root/name
        for file in (root/'docs').rglob('*'):
            if file.is_file():replacements[top+'/docs/'+file.relative_to(root/'docs').as_posix()]=file
        for info in original.infolist():
            if info.filename in replacements:continue
            target.writestr(info,original.read(info.filename))
        for name,file in replacements.items():target.write(file,name)
    os.replace(pending,archive)
    print('Updated guides; signed application payload unchanged:',archive)
