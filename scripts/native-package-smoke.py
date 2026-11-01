"""Smoke a production release in an isolated XDG profile without scan or cleanup.

Uses target/compat/release/inodune-desktop by default. No fixture feature,
backend consent bypass, Vite server or real user data is required.
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
standalone=True
out=root/'docs/evidence/f4/production-native'
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
            def request(method,path,payload=None,timeout=35):
                body=None if payload is None else json.dumps(payload).encode()
                try:
                    with urllib.request.urlopen(urllib.request.Request(base+path,data=body,headers={'Content-Type':'application/json'},method=method),timeout=timeout) as response:result=json.load(response)
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
            value=request('POST','/session',{'capabilities':{'alwaysMatch':{'tauri:options':{'application':str(Path(os.environ.get('INODUNE_SMOKE_BINARY', str(root/'target/compat/release/inodune-desktop'))).resolve())}}}})
            session=value['sessionId'];prefix='/session/'+session
            def script(source,*args):return request('POST',prefix+'/execute/sync',{'script':source,'args':args})
            def click(text):
                element=request('POST',prefix+'/element',{'using':'xpath','value':f"//button[normalize-space(.)='{text}']"})
                element_id=next(iter(element.values()));request('POST',prefix+'/element/'+element_id+'/click',{})
            def screenshot(name):
                request('POST',prefix+'/execute/async',{'script':'const done=arguments[arguments.length-1];requestAnimationFrame(()=>requestAnimationFrame(()=>done(true)));','args':[]})
                data=request('GET',prefix+'/screenshot');(out/name).write_bytes(base64.b64decode(data))
            until(lambda:script("return document.querySelector('dialog[open]')!==null"))
            assert script('return window.isTauri') is True
            assert script("return Array.from(document.images).every(img=>img.complete&&img.naturalWidth>0)"),'Brand assets must load under native CSP'
            if standalone:assert not script('return location.href').startswith('http://localhost:1420')
            click('Explorar sem analisar')
            until(lambda:'Meu computador' in script('return document.body.innerText'))
            assert '512 GiB' not in script('return document.body.innerText')
            screenshot('local-empty.png')
            def invoke(command,args,timeout=35):return request('POST',prefix+'/execute/async',{'script':"const done=arguments[arguments.length-1];window.__TAURI_INTERNALS__.invoke(arguments[0],arguments[1]).then(v=>done({ok:true,value:v})).catch(e=>done({ok:false,error:String(e)}));",'args':[command,args]},timeout)
            assert invoke('list_scopes',{})=={'ok':True,'value':[]}
            assert invoke('list_history',{})=={'ok':True,'value':[]}
            installed=invoke('list_installed_apps',{},60)
            assert installed.get('ok') is True, installed
            assert 'not allowed by ACL' not in json.dumps(installed), installed
            assert installed['value'].get('read_only') is True, installed
            denied=invoke('begin_scan',{'scopeIds':[]})
            assert not denied['ok'],denied
            click('Histórico')
            until(lambda:'Nenhum relatório salvo' in script('return document.body.innerText'))
            screenshot('production-history-empty.png')
            css='.sidebar-bottom > button'
            element=request('POST',prefix+'/element',{'using':'css selector','value':css})
            request('POST',prefix+'/element/'+next(iter(element.values()))+'/click',{})
            script("const select=document.querySelector('select[aria-label=Idioma]');select.value='en-US';select.dispatchEvent(new Event('change',{bubbles:true}));")
            until(lambda:script("return document.documentElement.lang==='en-US'"))
            screenshot('production-settings-english.png')
            (out/'result.json').write_text(json.dumps({'production_release':True,'standalone_without_vite':True,'empty_approved_scopes':True,'empty_history':True,'installed_apps_acl':True,'unapproved_scan_rejected':True,'english_preferences':True,'graphics':'Xvfb software compositing fallback','no_data_scan_or_cleanup':True,'result':'passed'},indent=2)+'\n')
            print('PASS: production release opens standalone, loads canonical assets/CSP, has no approved scope, rejects unapproved scan and shows local empty history/English settings.')
        finally:
            if session:
                try:request('DELETE','/session/'+session)
                except Exception:pass
            for process in reversed(processes):stop(process)
