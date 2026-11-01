import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve, delimiter } from 'node:path';
import { spawn } from 'node:child_process';
const root=fileURLToPath(new URL('../',import.meta.url));
const env={...process.env};
const localCargo=resolve(root,'.tools/cargo');
if(existsSync(resolve(localCargo,'bin/cargo'))){env.CARGO_HOME=localCargo;env.RUSTUP_HOME=resolve(root,'.tools/rustup');env.PATH=resolve(localCargo,'bin')+delimiter+env.PATH;}
const args=process.argv.slice(2);
const child=spawn('npm',['run','tauri','-w','@inodune/desktop','--',...(args.length?args:['dev'])],{cwd:root,env,stdio:'inherit'});
child.on('error',error=>{console.error(error.message);process.exitCode=1;});
child.on('exit',code=>{process.exitCode=code??1;});
for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>child.kill(signal));
