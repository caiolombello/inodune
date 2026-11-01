"""Real CLI confirmation and execution, exclusively on freshly created TempDirs."""
import json,os,pty,select,signal,subprocess,tempfile,time
from pathlib import Path
root=Path(__file__).resolve().parents[1]
results=[]
for action in ('trash','permanent','refuse'):
    with tempfile.TemporaryDirectory(prefix='inodune-cli-fixture-') as temporary:
        tree=Path(temporary);project=tree/'projects';cache=project/'__pycache__';cache.mkdir(parents=True)
        source=project/'module.py';source.write_text('pass\n')
        pyc=cache/'module.cpython-311.pyc';pyc.write_bytes(bytes([0xa7,0x0d,13,10])+bytes(12)+b'c'+bytes(47))
        env=dict(os.environ)
        for key in ('XDG_CACHE_HOME','XDG_CONFIG_HOME','XDG_DATA_HOME','XDG_STATE_HOME'):
            path=tree/key.lower();path.mkdir(mode=0o700);env[key]=str(path)
        args=[str(root/'target/debug/inodune-cli'),'clean','--root',str(project),'--apply','--json']
        if action=='permanent':args+=['--permanent']
        master,slave=pty.openpty();process=None
        try:
            process=subprocess.Popen(args,stdin=slave,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env=env,start_new_session=True)
            os.close(slave);prompt=b'';deadline=time.monotonic()+20
            while b'Digite exatamente' not in prompt:
                if time.monotonic()>deadline:raise RuntimeError('Confirmation timeout')
                ready,_,_=select.select([process.stderr],[],[],1)
                if ready:
                    chunk=os.read(process.stderr.fileno(),8192)
                    if not chunk:raise RuntimeError(prompt.decode())
                    prompt+=chunk
            assert pyc.exists() and source.exists(),'No mutation before exact confirmation'
            os.write(master,('DELETE 1\n' if action=='permanent' else 'WRONG\n' if action=='refuse' else 'TRASH 1\n').encode())
            stdout,stderr=process.communicate(timeout=20)
            if action=='refuse':assert process.returncode==2 and pyc.exists() and source.exists()
            else:
                assert process.returncode==0,(stdout,stderr)
                report=json.loads(stdout);assert report['complete'] and report['attributed_freed_bytes'] is None
                assert report['items'][0]['files'][0]['status']==('removed' if action=='permanent' else 'trashed')
                assert not pyc.exists() and source.exists() and cache.exists()
                assert Path(report['history_path']).is_file() and Path(report['journal_path']).is_file()
                if action=='trash':assert Path(report['items'][0]['files'][0]['preserved_at']).stat().st_size==64
            results.append({'action':action,'result':'passed','scope':'task-created temporary fixture'})
        finally:
            os.close(master)
            if process and process.poll() is None:os.killpg(process.pid,signal.SIGTERM);process.wait(timeout=10)
out=root/'docs/evidence/f2/cli-pty.json';out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(results,indent=2)+'\n')
print('PASS: real CLI terminal consent, Trash, permanent unlink and refusal; only isolated temporary fixtures.')
