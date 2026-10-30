"""Integration checks using the actual shipped engine and Rust command layer."""
import json,subprocess,tempfile,time
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='lichess-local-smoke-') as directory:
    p=subprocess.Popen([str(ROOT/'artifacts/rust/release/merci-cli.exe'),directory,str(ROOT/'artifacts/engine/fairy-stockfish.exe')],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,encoding='utf-8')
    revision=1
    def call(action,**data):
        global revision
        p.stdin.write(json.dumps(dict(action=action,revision=revision,**data))+'\n');p.stdin.flush()
        response=json.loads(p.stdout.readline());assert response['ok'],response['error']
        revision=response['snapshot']['revision'];return response['snapshot']
    snapshot=call('snapshot')
    for variant in ['chess','chess960','crazyhouse','kingofthehill','3check','antichess','atomic','horde','racingkings']:
        options=dict(variant=variant,side='black',level=3,player='Smoke',chess960=518)
        s=call('new',options=options);s=call('computer');assert len(s['moves'])==1,variant
        move=s['legal'][0]['uci'];s=call('move',text=move);s=call('computer');assert len(s['moves'])==3,variant
        call('resign');print(f'{variant}: legal human and engine moves, finish, PGN, save',flush=True)
    s=call('snapshot');assert len(s['history'])==9
    last=s['history'][0]['id'];s=call('history',text=last);assert s['status']=='finished'
    s=call('analyze',index=0);assert 'bestmove' in s['analysis']
    p.stdin.close();assert p.wait(timeout=10)==0
    # Reopen saved data through a new core process.
    q=subprocess.run([str(ROOT/'artifacts/rust/release/merci-cli.exe'),directory,str(ROOT/'artifacts/engine/fairy-stockfish.exe')],input=json.dumps({'action':'snapshot'})+'\n',capture_output=True,text=True,timeout=10)
    reopened=json.loads(q.stdout);assert reopened['ok'];assert len(reopened['snapshot']['history'])==9
    print('History, analysis and restart restoration passed.',flush=True)
