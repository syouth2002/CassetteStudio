"""ZIP a prepared release folder, including license files with old source timestamps."""
import pathlib,sys,zipfile
source=pathlib.Path(sys.argv[1]);destination=pathlib.Path(sys.argv[2])
with zipfile.ZipFile(destination,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False) as archive:
    for file in sorted(source.rglob('*')):
        if file.is_file():archive.write(file,file.relative_to(source.parent))
print(destination)
