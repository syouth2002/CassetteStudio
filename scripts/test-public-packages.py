"""Validate release contents without extracting user data into a package."""
import io,pathlib,sys,tarfile,zipfile
release=pathlib.Path(sys.argv[1])
def check(names,root,executable):
    paths={n.removeprefix('./') for n in names}
    for suffix in [executable,'README.md','README.ja.md','LICENSE','THIRD_PARTY_NOTICES.md','licenses/dependencies.json','licenses/rust-standard-library/COPYRIGHT-library.html','licenses/option-ext-0.2.0/source/src/lib.rs']:
        assert f'{root}/{suffix}' in paths,suffix
    for language in ['ja','en']:
        for screen in ['library','cassette','settings']:
            assert f'{root}/docs/screenshots/{screen}-{language}.jpg' in paths
    for name in paths:
        parts=pathlib.PurePosixPath(name).parts
        assert '..' not in parts and not name.startswith('/'),name
        assert not any(p in {'data','preview','exports','webview'} for p in parts),name
        assert pathlib.PurePosixPath(name).suffix.lower() not in {'.mp3','.wav','.m4a','.flac','.mp4'},name
        assert not any(p.lower() in {'ffmpeg.exe','ffprobe.exe','yt-dlp.exe','node.exe','ffmpeg','ffprobe','yt-dlp','node'} for p in parts),name
    return paths
with zipfile.ZipFile(next(release.glob('*Windows-x64.zip'))) as archive:
    assert archive.testzip() is None
    paths=check(archive.namelist(),'windows-x64','CassetteStudio.exe')
    assert 'windows-x64/SetupAudioTools.cmd' in paths
    assert 'windows-x64/licenses/microsoft-webview2-sdk-1.0.3800.47/LICENSE.txt' in paths
with tarfile.open(next(release.glob('*Linux-Ubuntu24-x64.tar.gz'))) as archive:
    check(archive.getnames(),'linux-x64','CassetteStudio')
    assert archive.getmember('linux-x64/CassetteStudio').mode&0o111
print('PASS: Windows/Linux archives, executables, bilingual docs, six screenshots, crate/runtime licenses, MPL source, no user media or library')
