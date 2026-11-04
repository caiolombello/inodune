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
evidence_set=os.environ.get('INODUNE_EVIDENCE_SET','automatic')
if evidence_set not in {'automatic','launch'}:raise SystemExit('Unknown evidence set')
# INODUNE_EVIDENCE_DIR keeps a verification run out of the versioned evidence.
out=Path(os.environ['INODUNE_EVIDENCE_DIR']) if os.environ.get('INODUNE_EVIDENCE_DIR') else root/'docs/evidence'/evidence_set/'native'
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
    # Official Tauri graphics fallback for isolated Xvfb without accelerated DRI.
    env['WEBKIT_DISABLE_COMPOSITING_MODE']='1'
    env['INODUNE_FIXTURE_CONFIRM']='1'
    env['INODUNE_FIXTURE_REVIEW']='1'
    env['INODUNE_FIXTURE_AUTO_PARTIAL']='1'
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
                # Wait for running enter animations (dialogs fade in for ~300 ms) so a
                # screenshot never captures a half-transparent frame.
                request('POST',prefix+'/execute/async',{'script':'const done=arguments[arguments.length-1];Promise.all(document.getAnimations().map(a=>a.finished.catch(()=>null))).then(()=>requestAnimationFrame(()=>requestAnimationFrame(()=>done(true))));','args':[]})
                data=request('GET',prefix+'/screenshot');(out/name).write_bytes(base64.b64decode(data))
            until(lambda:script("return document.body.innerText.includes('Analisar computador')"))
            assert script('return window.isTauri') is True
            assert script("return Array.from(document.images).every(img=>img.complete&&img.naturalWidth>0)"),'Brand assets must load under native CSP'
            if standalone:assert not script('return location.href').startswith('http://localhost:1420')
            until(lambda:'Meu computador' in script('return document.body.innerText'))
            assert '512 GiB' not in script('return document.body.innerText')
            screenshot('local-empty.png')
            # Fixture scopes are pre-approved by the feature-gated native harness, never by JS paths.
            click('Analisar computador')
            until(lambda:script("return document.body.innerText.includes('candidatos ·')&&!document.body.innerText.includes('Iniciando análise')"))
            until(lambda:script("return Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()==='Abrir a lista completa'&&!b.disabled)"))
            screenshot('local-results.png')
            click('Abrir a lista completa')
            until(lambda:'Miniaturas XDG' in script('return document.body.innerText'))
            text=script('return document.body.innerText')
            assert 'Estado / configuração protegida' in text and 'Sessões e memória de agentes' in text
            (out/'review-debug.json').write_text(json.dumps({'text':text,'checkboxes':script("return Array.from(document.querySelectorAll('input[type=checkbox]')).map(b=>({label:b.getAttribute('aria-label'),checked:b.checked}))")},indent=2))
            # Thumbnails, Terraform provider and two Cargo items (fingerprint JSON and, since
            # "allow reviewed recreatable tool caches", the reviewed target/ cache) are
            # selectable; everything else is protected.
            assert script("return document.querySelectorAll('input[type=checkbox]:not(:disabled)').length") == 4
            screenshot('local-review.png')
            def invoke(command,args):return request('POST',prefix+'/execute/async',{'script':"const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(v=>done({ok:true,value:v})).catch(e=>done({ok:false,error:String(e)}));",'args':[command,args]})
            cleanup_jobs=[0]
            def css_click(selector):
                element=request('POST',prefix+'/element',{'using':'css selector','value':selector});request('POST',prefix+'/element/'+next(iter(element.values()))+'/click',{})
            def run_cleanup(title,permanent,name):
                # Low-risk defaults (thumbnails) start checked; review one item alone.
                if script("return document.querySelectorAll('input[type=checkbox]:checked').length"):click('Desmarcar todos')
                css_click('input[aria-label="Selecionar '+title+'"]')
                assert script("return Array.from(document.querySelectorAll('input[type=checkbox]:checked')).length")==1
                click('Revisar impacto')
                until(lambda:'Preparar limpeza segura' in script('return document.body.innerText'))
                if permanent:css_click('input[type=radio][value=permanent]')
                click('Preparar limpeza segura')
                until(lambda:script("return document.querySelector('dialog[open]')!==null"))
                assert 'Revisão individual obrigatória' in script('return document.body.innerText')
                screenshot(name+'-confirmation.png')
                css_click('dialog input[type=checkbox]')
                click('Prosseguir para confirmação nativa')
                until(lambda:'Limpeza concluída.' in script('return document.body.innerText'))
                job=f'cleanup-{cleanup_jobs[0]}'
                cleanup_jobs[0]+=1
                result=invoke('poll_cleanup',{'jobId':job});assert result['ok'],result
                report=result['value']['report'];assert report['complete'] and report['attributed_freed_bytes'] is None,report
                assert all(f['status']==('removed' if permanent else 'trashed') for item in report['items'] for f in item['files'])
                screenshot(name+'-report.png')
                (out/(name+'-report.json')).write_text(json.dumps(report,indent=2)+'\n')
                return report
            provider=run_cleanup('Provider Terraform',False,'provider')
            assert provider['logical_bytes_processed']==77
            first=Path(provider['items'][0]['files'][0]['display_path']);projects=first.parents[8]
            # Derive only from this process's /tmp fixture path, never from caller-provided real data.
            assert str(projects).startswith('/tmp/')
            project=next(p for p in first.parents if p.name=='infra')
            assert (project/'terraform.tfstate').read_bytes()==b'fixture state'
            assert (project/'.terraform/terraform.tfstate').read_bytes()==b'private backend fixture'
            assert (project/'.terraform/modules/local-script.sh').read_bytes()==b'private module fixture'
            assert (project/'.terraform.lock.hcl').is_file()
            assert sum(Path(f['preserved_at']).stat().st_size for f in provider['items'][0]['files'])==77
            click('Voltar à revisão')
            click('Analisar novamente')
            until(lambda:script("return Array.from(document.querySelectorAll('button')).some(b=>b.textContent.trim()==='Abrir a lista completa'&&!b.disabled)"))
            click('Abrir a lista completa')
            until(lambda:'Metadado de fingerprint Cargo' in script('return document.body.innerText'))
            cargo=run_cleanup('Metadado de fingerprint Cargo',True,'cargo')
            cargo_json=Path(cargo['items'][0]['files'][0]['display_path']);assert not cargo_json.exists()
            assert cargo_json.with_suffix('').read_bytes()==b'0123456789abcdef'
            assert next(p for p in cargo_json.parents if p.name=='orbit').joinpath('target/binary').read_bytes()==b'build fixture'
            denied=invoke('begin_cleanup',{'planId':cargo['plan_id'],'hash':'replayed'})
            assert not denied['ok'] and 'consumido' in denied['error']
            history=invoke('list_history',{});assert history['ok'],history
            assert len(history['value'])==2,history
            for entry in history['value']:
                reread=invoke('get_history_report',{'reportId':entry['report_id']})
                assert reread['ok'] and reread['value']['plan_id']==entry['plan_id'],reread
            invalid=invoke('get_history_report',{'reportId':'../report-not-an-id'})
            assert not invalid['ok'],invalid
            # Exclusions invalidate old snapshots before a second read-only scan.
            scopes=invoke('list_scopes',{})['value']
            project_scope=next(scope for scope in scopes if scope['kind']=='projects')
            changed=invoke('set_scope_exclusions',{'scopeId':project_scope['id'],'relativePaths':[str(project.relative_to(Path(project_scope['display_path'])))]})
            assert changed['ok'] and changed['value']['exclusions']==[str(project.relative_to(Path(project_scope['display_path'])))],changed
            invalid_plan=invoke('prepare_cleanup',{'scanId':'scan-1','candidateIds':['untrusted'],'action':'trash','locale':'en-US'})
            assert not invalid_plan['ok'],invalid_plan
            scan=invoke('begin_scan',{'scopeIds':[scope['id'] for scope in scopes]});assert scan['ok'],scan
            def excluded_result():
                poll=invoke('poll_scan',{'jobId':scan['value']})
                return poll['value'].get('result') if poll['ok'] else None
            after=until(excluded_result)
            assert not any('/infra/' in c['display_path'] for c in after['candidates']),after
            assert any(i['code']=='policy_excluded' for i in after['issues']),after
            # Exercise actual native window reflow, light mode, and manual reduced-motion control.
            css_click('.sidebar-bottom > button')
            css_click('select[aria-label="Tema"] option[value=light]')
            until(lambda:script("return document.documentElement.dataset.theme==='light'"))
            click('Histórico')
            until(lambda:script("return document.querySelectorAll('.entries button').length===2"))
            css_click('.entries button')
            until(lambda:'Detalhes do relatório' in script('return document.body.innerText'))
            screenshot('report-light.png')
            request('POST',prefix+'/window/rect',{'width':420,'height':900})
            until(lambda:script('return innerWidth<600'))
            assert script('return document.documentElement.scrollWidth<=innerWidth')
            screenshot('report-narrow.png')
            request('POST',prefix+'/window/rect',{'width':1280,'height':900})
            css_click('.sidebar-bottom > button')
            css_click('.setting-row input[type=checkbox]')
            until(lambda:script("return document.documentElement.dataset.motion==='reduce'"))
            assert script("return parseFloat(getComputedStyle(document.querySelector('button')).transitionDuration)")<=0.00002
            screenshot('preferences-reduced.png')
            (out/'result.json').write_text(json.dumps({'standalone_without_vite':standalone,'native':True,'backend':'Rust IPC','provider_review':'77 bytes actually moved to temporary Trash; state/backend/lock/modules retained','cargo_review':'only diagnostic JSON actually unlinked; hash/source/artifacts retained','native_confirmation':'fixture-only consent; production GTK dialog not automated','replay_rejected':True,'history_reread':True,'invalid_report_id_rejected':True,'exclusions_invalidate_scan':True,'native_reflow_light_reduced':True,'automatic_discovery':True,'partial_inventory_verified_plans':True,'result':'passed'},indent=2)+'\n')
            print('PASS: automatic partial discovery with no picker, verified-subset plans, history/exclusions plus individual provider/Cargo review → actual temporary Trash/unlink → accurate reports; state preserved; reflow/light/reduced motion verified.')
        finally:
            if session:
                try:request('DELETE','/session/'+session)
                except Exception:pass
            for process in reversed(processes):stop(process)
