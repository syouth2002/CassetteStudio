"""Build the same Rust UI for the local browser, then rebuild the server separately."""
import pathlib,subprocess,tomllib
root=pathlib.Path(__file__).resolve().parent.parent
lock=tomllib.loads((root/'Cargo.lock').read_text(encoding='utf-8'))
version=next(p['version']for p in lock['package']if p['name']=='wasm-bindgen')
subprocess.run(['rustup','target','add','wasm32-unknown-unknown'],cwd=root,check=True)
try: installed=subprocess.check_output(['wasm-bindgen','--version'],text=True).strip().split()[-1]
except FileNotFoundError: installed=None
if installed!=version:
    raise SystemExit(f'Install the matching tool first: cargo install wasm-bindgen-cli --version {version} --locked')
subprocess.run(['cargo','build','-p','cassette-ui','--target','wasm32-unknown-unknown','--release','--locked'],cwd=root,check=True)
metadata=__import__('json').loads(subprocess.check_output(['cargo','metadata','--no-deps','--format-version','1'],cwd=root))
wasm=pathlib.Path(metadata['target_directory'])/'wasm32-unknown-unknown/release/cassette_ui.wasm'
subprocess.run(['wasm-bindgen','--target','web','--out-dir',str(root/'ui/web/pkg'),'--no-typescript',str(wasm)],cwd=root,check=True)
print('Rust browser UI generated. Rebuild cassette-studio to embed it.')
