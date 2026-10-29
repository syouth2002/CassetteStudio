"""Verify the Arch release artifacts without installing or extracting them."""
import hashlib,pathlib,re,sys,tarfile
root=pathlib.Path(sys.argv[1])
package=root/'cassette-studio-0.1.4-1-x86_64.pkg.tar.zst'
source=root/'CassetteStudio-0.1.4-Arch-source.tar.gz'
recipe=(root/'PKGBUILD').read_text(encoding='utf-8')
assert hashlib.sha256(source.read_bytes()).hexdigest() in recipe
assert '@SOURCE_SHA256@' not in recipe and "sha256sums=('SKIP')" not in recipe
with tarfile.open(package) as archive:
    names={name.removeprefix('./') for name in archive.getnames()}
    for name in ['usr/bin/cassette-studio','usr/bin/cassette-studio-browser',
      'usr/share/applications/cassette-studio.desktop','usr/share/licenses/cassette-studio/LICENSE',
      'usr/share/doc/cassette-studio/README.md','usr/share/doc/cassette-studio/README.ja.md',
      'usr/share/doc/cassette-studio/licenses/rust-standard-library/COPYRIGHT-library.html',
      'usr/share/doc/cassette-studio/licenses/option-ext-0.2.0/source/src/lib.rs']:
        assert name in names,name
    for language in ['ja','en']:
        for screen in ['library','cassette','settings','options','reference-tone','entry-fit','settings-audio','recording','scroll']:
            assert f'usr/share/doc/cassette-studio/docs/screenshots/{screen}-{language}.jpg' in names
    binary=archive.extractfile('usr/bin/cassette-studio').read()
    assert binary[:4]==b'\x7fELF' and binary[4]==2 and int.from_bytes(binary[18:20],'little')==62
    metadata=archive.extractfile('.PKGINFO').read().decode()
    assert 'pkgver = 0.1.4-1' in metadata and 'arch = x86_64' in metadata
    for required in ['webkit2gtk-4.1','ffmpeg','xdotool']:
        assert f'depend = {required}' in metadata
    for name in names:
        parts=pathlib.PurePosixPath(name).parts
        assert not name.startswith('/') and '..' not in parts
        assert not any(part in {'data','preview','exports','webview'} for part in parts),name
        assert pathlib.PurePosixPath(name).suffix.lower() not in {'.mp3','.wav','.m4a','.mp4','.flac'},name
        assert pathlib.PurePosixPath(name).name not in {'ffmpeg','ffprobe','yt-dlp','node'},name
    for language in ['ja','en']:
        filename='README.ja.md' if language=='ja' else 'README.md'
        assert b'Arch Linux' in archive.extractfile('usr/share/doc/cassette-studio/'+filename).read()
with tarfile.open(source) as archive:
    names=archive.getnames()
    assert 'cassette-studio-0.1.4/packaging/arch/PKGBUILD.in' in names
    for name in names:
        assert not any(p in {'data','work','tools','target','dist'} for p in pathlib.PurePosixPath(name).parts),name
print('PASS: Arch package metadata/ELF, dependencies, 18 screenshots, notices, source checksum, no user media')
