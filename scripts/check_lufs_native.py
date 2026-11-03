"""Shared LUFS endpoint checks for packaged native applications."""
import json,urllib.request,urllib.error,io,wave,struct

def check_lufs(base,track_id,collection_id):
    def post(route,body):
        request=urllib.request.Request(base+route,data=json.dumps(body).encode(),headers={'Content-Type':'application/json'})
        try:
            return json.load(urllib.request.urlopen(request,timeout=120))
        except urllib.error.HTTPError as error:
            raise AssertionError(f'{route}: {error.read().decode()}') from error
    effect_request=urllib.request.Request(base+'/api/tracks/'+track_id+'/pseudo-stereo',data=json.dumps({'strength':2}).encode(),method='PUT',headers={'Content-Type':'application/json'})
    effected=json.load(urllib.request.urlopen(effect_request,timeout=120))
    assert effected['mono'] is True and effected['pseudo_stereo']==2
    assert effected['pseudo_analysis']['strength']==2
    preview=post('/api/preview/'+track_id,{'normalize':True,'remove_dc':True})
    with wave.open(io.BytesIO(urllib.request.urlopen(base+preview['url']).read()),'rb') as audio:
        assert audio.getnchannels()==2
        samples=list(struct.iter_unpack('<hh',audio.readframes(audio.getnframes())))
        assert any(abs(left-right)>2 for left,right in samples),'Pseudo stereo did not create channel differences'
    graph=post('/api/tracks/'+track_id+'/loudness-graph',{'settings':{'normalize':True,'remove_dc':True}})
    assert graph['duration']>0 and graph['original']['points']
    assert any(p['momentary'] is not None for p in graph['processed']['points'])
    assert graph['processed']['true_peak_db'] is not None
    for side in [0,1]:
        response=post('/api/collections/'+collection_id+'/loudness-graph',{'side':side})
        graph=response['graph']
        assert graph['duration']>0 and graph['processed']['points']
        assert graph['markers'] and graph['markers'][0]['time']==.5
        assert graph['processed']['integrated_lufs'] is not None
        assert response==post('/api/collections/'+collection_id+'/loudness-graph',{'side':side})
    assert b'lufs-layout' in urllib.request.urlopen(base).read()
    assert b'loadSide' in urllib.request.urlopen(base+'/js/audio/loudness-graph.js').read()
    effect_request=urllib.request.Request(base+'/api/tracks/'+track_id+'/pseudo-stereo',data=json.dumps({'strength':0}).encode(),method='PUT',headers={'Content-Type':'application/json'})
    assert json.load(urllib.request.urlopen(effect_request,timeout=120))['pseudo_stereo']==0
    print('PASS: mono detection and pseudo stereo preview; packaged track/A/B LUFS analysis, true peak, track starts, cache and embedded graph UI')
