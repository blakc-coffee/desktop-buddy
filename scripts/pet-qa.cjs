// Attach to the UI-launched isolated fixture. No model calls or synthesized input.
const {chromium}=require(process.env.BUDDY_PLAYWRIGHT_PATH||'playwright');
const fs=require('node:fs');const path=require('node:path');
const {spawn}=require('node:child_process');
const pause=ms=>new Promise(r=>setTimeout(r,ms));
(async()=>{
 let child;
 if(process.env.BUDDY_PET_QA_LAUNCH==='1') {
  child=spawn(path.resolve('src-tauri/target/release/desktop-buddy.exe'),['--panel'],{env:{...process.env,BUDDY_DATA_DIR:path.resolve('artifacts/pet-live-data-v4'),WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS:'--remote-debugging-port=9225 --remote-debugging-address=127.0.0.1'},windowsHide:true,stdio:['ignore','ignore','pipe']});
  child.stderr.on('data',d=>process.stderr.write(d));
  for(let i=0;i<50;i++){try{if((await fetch('http://127.0.0.1:9225/json/version')).ok)break;}catch{}if(child.exitCode!==null)throw Error('Buddy exited: '+child.exitCode);await pause(300);}
 }
 const browser=await chromium.connectOverCDP('http://127.0.0.1:9225');
 try {
  const page=browser.contexts()[0].pages()[0];
  await page.getByRole('heading',{name:'A little less switching.'}).waitFor();
  const invoke=(cmd,args={})=>page.evaluate(({cmd,args})=>window.__TAURI_INTERNALS__.invoke(cmd,args),{cmd,args});
  const root=path.resolve('artifacts/pet-transfer');
  const files=[path.join(root,'input/handoff.txt'),path.join(root,'input/猫.txt')];
  const originals=files.map(p=>fs.readFileSync(p));
  await invoke('pet_commands');
  let bar;
  for(let i=0;i<30;i++){bar=await invoke('pet_status');if(bar.commandBarVisible)break;await pause(100);}
  if(!bar.commandBarVisible)throw Error('Native command bar did not open');
  await invoke('pet_commands');
  for(let i=0;i<30;i++){bar=await invoke('pet_status');if(!bar.commandBarVisible)break;await pause(100);}
  if(bar.commandBarVisible)throw Error('Native command bar did not close');
  const output=path.join(root,'output-'+Date.now());fs.mkdirSync(output);
  await invoke('pet_hold',{paths:files});
  let held;
  for(let i=0;i<30;i++){held=await invoke('pet_status');if(held.count===2)break;await pause(100);}
  if(held.count!==2||held.secondsLeft<118||held.secondsLeft>120)throw Error('Two-minute native hold failed');
  await invoke('pet_copy');
  const copied=await invoke('pet_status');if(!copied.message.startsWith('Copied file references'))throw Error('Native clipboard copy failed');
  if(copied.count!==0||copied.secondsLeft!==0)throw Error('Successful copy did not clear the hold');
  let consumed=false;try{await invoke('pet_copy');}catch{consumed=true;}
  if(!consumed)throw Error('A consumed batch was copied again');
  await invoke('stage_paths',{paths:files});
  const start=Date.now();
  await invoke('pet_hold',{paths:files});
  for(let i=0;i<30;i++){held=await invoke('pet_status');if(held.count===2)break;await pause(100);}
  if(held.count!==2)throw Error('Could not start a fresh hold after the handoff');
  console.log('READY: bar lifecycle and single-use copy passed. Checking expiry of a fresh batch.');
  while(Date.now()-start<123000){await pause(500);held=await invoke('pet_status');if(held.count===0)break;}
  if(Date.now()-start<119000)throw Error('Fresh hold expired early');
  if(held.count!==0||held.secondsLeft!==0)throw Error('Hold did not expire');
  for(let i=0;i<files.length;i++)if(!fs.readFileSync(files[i]).equals(originals[i]))throw Error('Original content changed');
  let rejected=false;try{await invoke('pet_copy');}catch{rejected=true;}if(!rejected)throw Error('Copy after expiry should fail');
  const result={noModelCalls:true,holdSeconds:120,checks:['native command bar opens and closes','native hold count and deadline','copy-only shell clipboard handoff consumes the batch','second copy is rejected','real two-minute expiry','original files unchanged','copy after expiry rejected'],destinationCopyVerified:files.every((p,i)=>{const destination=path.join(output,path.basename(p));return fs.existsSync(destination)&&fs.readFileSync(destination).equals(originals[i]);}),destination:output,explorerPasteVerified:false,note:'Cross-window drag and Explorer paste require manual acceptance; no Explorer UI action was observed.'};
  fs.writeFileSync('artifacts/pet-qa.json',JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));
 }finally{await browser.close();if(child)child.kill();}
})().catch(e=>{console.error(e);process.exitCode=1;});
