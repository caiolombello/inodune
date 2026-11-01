"""Launch only the task's binary and isolated X server; never scan user data."""
import os
import subprocess
import tempfile
import time
from pathlib import Path

root = Path(__file__).resolve().parents[1]
out = root / 'docs/evidence/native'
out.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix='inodune-native-') as temp:
    env = dict(os.environ)
    for key in ['XDG_CONFIG_HOME','XDG_CACHE_HOME','XDG_DATA_HOME','XDG_STATE_HOME']:
        env[key] = str(Path(temp, key.lower()))
        Path(env[key]).mkdir(mode=0o700)
    env['GDK_BACKEND'] = 'x11'
    read_fd, write_fd = os.pipe()
    with (out/'xvfb.log').open('wb') as xlog, (out/'app.log').open('wb') as app_log:
        display = subprocess.Popen(['Xvfb','-displayfd',str(write_fd),'-screen','0','1440x1000x24','-nolisten','tcp'],pass_fds=(write_fd,),stdout=xlog,stderr=xlog)
        os.close(write_fd)
        app = None
        try:
            with os.fdopen(read_fd) as pipe:
                number=pipe.readline().strip()
            if not number: raise RuntimeError('Xvfb did not provide a display')
            env['DISPLAY']=':'+number
            app=subprocess.Popen([str(root/'target/debug/inodune-desktop')],env=env,stdout=app_log,stderr=app_log)
            time.sleep(4)
            if app.poll() is not None: raise RuntimeError(f'Application exited: {app.returncode}')
            subprocess.run(['import','-display',env['DISPLAY'],'-window','root',str(out/'desktop.png')],check=True)
            print('Native process remained alive; screenshot captured for visual inspection. Not a scanner/executor test.')
        finally:
            if app is not None and app.poll() is None:
                app.terminate()
                try: app.wait(timeout=10)
                except subprocess.TimeoutExpired: app.kill();app.wait()
            display.terminate();display.wait(timeout=10)
