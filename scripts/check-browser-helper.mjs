import { createServer } from 'node:http';
import { fileURLToPath } from 'node:url';
const { chromium } = await import(process.env.BUDDY_PLAYWRIGHT_MODULE || 'playwright');
const server = createServer((req, response) => response.end('<!doctype html><html><body></body></html>'));
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = 'http://127.0.0.1:' + server.address().port;
import assert from 'node:assert/strict';
const browser = await chromium.launch({executablePath:process.env.BUDDY_BROWSER_EXECUTABLE,headless:true});
try {
 const page=await browser.newPage();
 await page.goto(origin);
 await page.setContent(`<main><h1>Practice quiz</h1><p>What is 2 + 2? Select the correct complexity.</p><form onsubmit="window.submits++;return false"><label>Sum<textarea id="sum"></textarea></label><fieldset><legend>Binary search complexity</legend><label><input type="radio" name="complexity" value="linear">O(n)</label><label><input type="radio" name="complexity" value="log">O(log n)</label></fieldset><label><input type="checkbox" id="agree">I reviewed the answers</label><label>Language<select id="language"><option value="js">JavaScript</option><option value="py">Python</option></select></label><label>Password<input type="password" value="private"></label><input type="hidden" value="hidden"><button>Submit</button></form></main>`);
 await page.evaluate(()=>{window.submits=0;window.inputEvents=0;document.addEventListener('input',()=>window.inputEvents++);window.buddyHandlers=[];window.chrome={runtime:{id:'fixture',onMessage:{addListener:listener=>window.buddyHandlers.push(listener)}}};});
 await page.addScriptTag({path:fileURLToPath(new URL('../browser-helper/page.js', import.meta.url))});
 const send=message=>page.evaluate(message=>new Promise(resolve=>window.buddyHandlers[0](message,{id:'fixture'},resolve)),message);
 const first=await send({type:'capture'}); assert.equal(first.ok,true); assert.equal(first.payload.fields.length,4); assert(!JSON.stringify(first).includes('private'));
 const assignments=first.payload.fields.map(field=>({id:field.id,value:field.kind==='radio'?'1':field.kind==='checkbox'?'true':field.kind==='select'?'py':'4'}));
 const plan={id:'apply-fixture',nonce:first.payload.nonce,fingerprint:first.payload.fingerprint,assignments};
 const result=await send({type:'apply',plan}); assert.equal(result.ok,true); assert.equal(result.payload.changed.length,4); assert.equal(result.payload.error,'');
 assert.equal(await page.locator('#sum').inputValue(),'4'); assert.equal(await page.locator('#language').inputValue(),'py'); assert.equal(await page.locator('[value=log]').isChecked(),true); assert.equal(await page.locator('#agree').isChecked(),true); assert.equal(await page.evaluate(()=>window.submits),0);
 const count=await page.evaluate(()=>window.inputEvents); assert.deepEqual((await send({type:'apply',plan})).payload,result.payload); assert.equal(await page.evaluate(()=>window.inputEvents),count);
 const second=(await send({type:'capture'})).payload; await page.locator('#sum').fill('user edit');
 const conflict=await send({type:'apply',plan:{id:'conflict',nonce:second.nonce,fingerprint:second.fingerprint,assignments:second.fields.map(field=>({id:field.id,value:field.kind==='radio'?'0':field.kind==='checkbox'?'false':field.kind==='select'?'js':'overwrite'}))}}); assert.equal(conflict.ok,false); assert.equal(await page.locator('#sum').inputValue(),'user edit'); assert.equal(await page.locator('[value=log]').isChecked(),true);
 const third=(await send({type:'capture'})).payload; await page.locator('h1').evaluate(element=>element.textContent='New question');
 const stale=await send({type:'apply',plan:{id:'stale',nonce:third.nonce,fingerprint:third.fingerprint,assignments:[{id:third.fields[0].id,value:'99'}]}}); assert.equal(stale.ok,false);
 console.log('Browser control extraction, text/radio/checkbox/select fill, readback, no-submit, idempotency, edited-field and changed-question rejection passed.');
} finally { await browser.close(); server.close(); }
