"""W3C WebDriver journey through real Tauri IPC, using fixture-tests binary only.

Build with cargo build -p inodune-desktop --features fixture-tests first.
The native backend creates its own temporary roots. No path from the UI is trusted.
"""
import base64
import json
import os
import signal
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

root=Path(__file__).resolve().parents[1]
standalone=os.environ.get('INODUNE_NATIVE_STANDALONE')=='1'
out=root/'docs/evidence/f2/native'
out.mkdir(parents=True,exist_ok=True)
def port():
    with socket.socket() as s:s.bind(('127.0.0.1',0));return s.getsockname()[1]

def stop(process):
    if process is not None and process.poll() is None:
        os.killpg(process.pid,signal.SIGTERM)
        try:process.wait(timeout=10)
        except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait()

with tempfile.TemporaryDirectory(prefix='inodune-driver-') as temp:
    env=dict(os.environ)
    for key in ['XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_DATA_HOME','XDG_STATE_HOME']:
        env[key]=str(Path(temp,key.lower()));Path(env[key]).mkdir(mode=0o700)
    env['GDK_BACKEND']='x11'
    env['INODUNE_FIXTURE_CONFIRM']='1'
    read_fd,write_fd=os.pipe()
    processes=[];session=None
    with (out/'driver.log').open('wb') as log:
        try:
            display=subprocess.Popen(['Xvfb','-displayfd',str(write_fd),'-screen','0','1440x1000x24','-nolisten','tcp'],pass_fds=(write_fd,),stdout=log,stderr=log,start_new_session=True);processes.append(display)
            os.close(write_fd)
            with os.fdopen(read_fd) as pipe:env['DISPLAY']=':'+pipe.readline().strip()
            if not standalone:
                server=subprocess.Popen(['npm','run','preview','-w','@inodune/desktop'],cwd=root,stdout=log,stderr=log,start_new_session=True);processes.append(server)
            proxy_port=port();native_port=port()
            driver=subprocess.Popen([str(root/'.tools/tauri-driver/bin/tauri-driver'),'--port',str(proxy_port),'--native-port',str(native_port),'--native-driver',str(root/'.tools/webdriver/usr/bin/WebKitWebDriver')],env=env,stdout=log,stderr=log,start_new_session=True);processes.append(driver)
            base=f'http://127.0.0.1:{proxy_port}'
            def request(method,path,payload=None):
                body=None if payload is None else json.dumps(payload).encode()
                try:
                    with urllib.request.urlopen(urllib.request.Request(base+path,data=body,headers={'Content-Type':'application/json'},method=method),timeout=35) as response:result=json.load(response)
                except urllib.error.HTTPError as e:raise RuntimeError(e.read().decode()) from e
                value=result.get('value')
                if isinstance(value,dict) and 'error' in value and 'ok' not in value:raise RuntimeError(str(value))
                return value
            def until(fn,timeout=30):
                end=time.monotonic()+timeout;last=None
                while time.monotonic()<end:
                    try:
                        value=fn()
                        if value:return value
                    except Exception as e:last=e
                    time.sleep(.1)
                raise RuntimeError(f'Condition timed out: {last}')
            until(lambda:request('GET','/status'))
            if not standalone:until(lambda:urllib.request.urlopen('http://127.0.0.1:1420',timeout=2).status==200)
            value=request('POST','/session',{'capabilities':{'alwaysMatch':{'tauri:options':{'application':str(root/'target/debug/inodune-desktop')}}}})
            session=value['sessionId'];prefix='/session/'+session
            def script(source,*args):return request('POST',prefix+'/execute/sync',{'script':source,'args':args})
            def click(text):
                element=request('POST',prefix+'/element',{'using':'xpath','value':f"//button[normalize-space(.)='{text}']"})
                element_id=next(iter(element.values()));request('POST',prefix+'/element/'+element_id+'/click',{})
            def screenshot(name):
                data=request('GET',prefix+'/screenshot');(out/name).write_bytes(base64.b64decode(data))
            until(lambda:script("return document.querySelector('dialog[open]')!==null"))
            assert script('return window.isTauri') is True
            assert script("return Array.from(document.images).every(img=>img.complete&&img.naturalWidth>0)"),'Brand assets must load under native CSP'
            if standalone:assert not script('return location.href').startswith('http://localhost:1420')
            click('Explorar sem analisar')
            until(lambda:'Meu computador' in script('return document.body.innerText'))
            assert '512 GiB' not in script('return document.body.innerText')
            screenshot('local-empty.png')
            # Fixture scopes are pre-approved by the feature-gated native harness, never by JS paths.
            click('Analisar pastas')
            until(lambda:script("return document.body.innerText.includes('candidatos ·')&&!document.body.innerText.includes('Iniciando análise')"))
            until(lambda:script("return Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()==='Revisar candidatos'&&!b.disabled)"))
            screenshot('local-results.png')
            click('Revisar candidatos')
            until(lambda:'Miniaturas XDG' in script('return document.body.innerText'))
            text=script('return document.body.innerText')
            assert 'Estado / configuração protegida' in text and 'Sessões e memória de agentes' in text
            (out/'review-debug.json').write_text(json.dumps({'text':text,'checkboxes':script("return Array.from(document.querySelectorAll('input[type=checkbox]')).map(b=>({label:b.getAttribute('aria-label'),checked:b.checked}))")},indent=2))
            assert script("return document.querySelectorAll('input[type=checkbox]:not(:disabled)').length") == 1
            screenshot('local-review.png')
            click('Revisar impacto')
            until(lambda:'Preparar limpeza segura' in script('return document.body.innerText'))
            assert '0 B na mesma unidade' in script('return document.body.innerText')
            screenshot('impact-trash.png')
            click('Preparar limpeza segura')
            until(lambda:script("return document.querySelector('dialog[open]')!==null"))
            assert 'Plano local preparado' in script('return document.body.innerText')
            assert script("return Array.from(document.querySelectorAll('dialog button')).find(b=>b.textContent.includes('Prosseguir')).disabled") is True
            screenshot('confirmation.png')
            # Read the plan through a fresh isolated IPC scan, and exercise rejection before UI execution.
            def invoke(command,args):return request('POST',prefix+'/execute/async',{'script':"const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(v=>done({ok:true,value:v})).catch(e=>done({ok:false,error:String(e)}));",'args':[command,args]})
            # Forged ID never consumes an authorized plan or receives path authority.
            forged=invoke('begin_cleanup',{'planId':'/etc','hash':'forged'})
            assert forged['ok'] is False and 'desconhecido' in forged['error']
            # Its error occupies no active execution slot; the actual prepared UI plan remains valid.
            checkbox=request('POST',prefix+'/element',{'using':'css selector','value':'dialog input[type=checkbox]'})
            request('POST',prefix+'/element/'+next(iter(checkbox.values()))+'/click',{})
            click('Prosseguir para confirmação nativa')
            until(lambda:'Limpeza concluída.' in script('return document.body.innerText'))
            body=script('return document.body.innerText')
            assert 'Enviado à Lixeira' in body and 'Não atribuídos' in body and 'continuam ocupando' in body
            assert '68 B' in body
            screenshot('report-trash.png')
            # The runtime returns the real outcome; retained authority cannot be replayed.
            report=invoke('poll_cleanup',{'jobId':'cleanup-1'})
            assert report['ok'],report
            payload=report['value']['report'];assert payload['complete'] and payload['attributed_freed_bytes'] is None
            assert payload['items'][0]['files'][0]['status']=='trashed'
            preserved=Path(payload['items'][0]['files'][0]['preserved_at']);assert preserved.is_file() and preserved.stat().st_size==68
            assert str(preserved).startswith('/tmp/')
            assert not Path(payload['items'][0]['files'][0]['display_path']).exists()
            assert Path(payload['history_path']).is_file() and Path(payload['journal_path']).is_file()
            replay=invoke('begin_cleanup',{'planId':payload['plan_id'],'hash':'forged'})
            assert not replay['ok'] and 'consumido' in replay['error']
            (out/'execution-report.json').write_text(json.dumps(payload,indent=2)+'\n')
            (out/'result.json').write_text(json.dumps({'standalone_without_vite':standalone,'native':True,'backend':'Rust IPC','scopes':'application-owned temporary fixtures','cleanup':'real same-filesystem Trash move of 68-byte app-owned PNG fixture only','native_confirmation':'fixture-only consent; production GTK dialog not automated','opaque_forgery_rejected':True,'replay_rejected':True,'result':'passed'},indent=2)+'\n')
            print('PASS: standalone WebKitGTK UI → Rust plan → fixture-only consent → actual temporary Trash move → real report; forged and replayed IDs refused.')
        finally:
            if session:
                try:request('DELETE','/session/'+session)
                except Exception:pass
            for process in reversed(processes):stop(process)
