"""Package the verified Windows binaries with docs, notices and tool setup only."""
import pathlib,shutil,zipfile,tomllib,subprocess
root=pathlib.Path(__file__).resolve().parent.parent
version=tomllib.loads((root/'Cargo.toml').read_text())['package']['version']
release=root/f'dist/releases/v{version}';stage=release/'windows-x64'
if(stage/'CassetteStudio.exe').exists() and (stage/'CassetteStudio.exe').read_bytes()!=(root/'target/release/cassette-studio-desktop.exe').read_bytes():raise RuntimeError('Stage contains a different executable')
for folder in ['docs','assets','scripts']: (stage/folder).mkdir(parents=True,exist_ok=True)
for source,dest in [('cassette-studio-desktop.exe','CassetteStudio.exe'),('cassette-studio.exe','cassette-studio-browser.exe')]:
    shutil.copy2(root/'target/release'/source,stage/dest)
for name in ['README.md','README.ja.md','LICENSE','THIRD_PARTY_NOTICES.md']:shutil.copy2(root/name,stage/name)
shutil.copytree(root/'docs',stage/'docs',dirs_exist_ok=True)
shutil.copy2(root/'assets/cassette-studio-app.png',stage/'assets/cassette-studio-app.png')
shutil.copy2(root/'scripts/setup-tools.ps1',stage/'scripts/setup-tools.ps1')
(stage/'SetupAudioTools.cmd').write_bytes(b'@echo off\r\npowershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\\setup-tools.ps1"\r\npause\r\n')
(stage/'portable.txt').write_text('Keep application data beside the executable.\n')
if not(stage/'licenses/dependencies.json').is_file():
    subprocess.run(['python',root/'scripts/collect-licenses.py','--target','x86_64-pc-windows-msvc','--output',stage/'licenses'],check=True)
with zipfile.ZipFile(release/f'CassetteStudio-{version}-Windows-x64.zip','w',zipfile.ZIP_DEFLATED,compresslevel=6,strict_timestamps=False)as archive:
    for file in stage.rglob('*'):
        if file.is_file():archive.write(file,file.relative_to(release))
print('Ready:',stage)
