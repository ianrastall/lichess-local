import {test,expect} from '@playwright/test';
import {spawn,ChildProcessWithoutNullStreams} from 'node:child_process';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';
import readline from 'node:readline';
let process:ChildProcessWithoutNullStreams,dir:string;
let pending:((response:unknown)=>void)[]=[];
const root=path.resolve(import.meta.dirname,'../..');
test.beforeEach(async({page})=>{
 dir=await mkdtemp(path.join(tmpdir(),'lichess-local-browser-'));
 process=spawn(path.join(root,'artifacts/rust/release/merci-cli.exe'),[dir,path.join(root,'artifacts/engine/fairy-stockfish.exe')]);
 readline.createInterface({input:process.stdout}).on('line',line=>pending.shift()?.(JSON.parse(line)));
 await page.exposeFunction('sendNative',(request:any)=>new Promise(resolve=>{pending.push(resolve);process.stdin.write(JSON.stringify(request)+'\n');}));
 const source=await readFile(path.resolve(root,'../jupiter-ui/src/Jupiter.UI/Themes/Jupiter.xaml'),'utf8');
 const token=(name:string)=>new RegExp(`<Color x:Key="Jupiter${name}Color">([^<]+)</Color>`).exec(source)![1];
 const theme={window:token('Window'),surface:token('Surface'),raised:token('Raised'),input:token('Input'),separator:token('Separator'),boundary:token('ControlBoundary'),text:token('PrimaryText'),secondary:token('SecondaryText'),hover:token('ControlFillHover'),selection:token('Selected'),error:token('Error'),accent:'#C8A96B','accent-text':token('Window')};
 await page.addInitScript(({theme})=>{let listener:any;Object.defineProperty(window,'chrome',{value:{webview:{addEventListener:(_name:string,fn:any)=>{listener=fn;queueMicrotask(()=>listener({data:{kind:'theme',values:theme,dark:true}}));},postMessage:(request:any)=>{(window as any).sendNative({...request,requestId:Number(request.requestId),revision:Number(request.revision)}).then((response:any)=>listener({data:response}));}}},configurable:true});}, {theme});
 await page.goto('/');await expect(page.locator('body')).toHaveClass('ready');await expect(page.locator('body')).toHaveAttribute('data-core-ready','true');
});
test.afterEach(async()=>{process.stdin.end();process.kill();pending=[];await rm(dir,{recursive:true,force:true});});
test('play, review, resign, local score and saved history',async({page})=>{
 await page.getByRole('button',{name:'New game',exact:true}).click();
 await page.locator('#move-input').fill('e4');await page.getByRole('button',{name:'Play move',exact:true}).click();
 await expect(page.locator('#moves button')).toHaveCount(2);await expect(page.locator('#turn-status')).toContainText('Your turn');
 await page.locator('#first').click();await expect(page.locator('#move-input')).toBeDisabled();await page.locator('#last').click();
 await page.getByRole('button',{name:'Resign',exact:true}).click();await page.locator('#confirm-ok').click();
 await expect(page.locator('#result')).toContainText('0-1');await expect(page.locator('#score-details')).toContainText('1 scored games');
 await page.getByRole('button',{name:'Analyse position'}).click();await expect(page.locator('#analysis')).toBeVisible();
 await expect(page.locator('#turn-status')).toHaveText('Resignation');
 await page.getByRole('button',{name:'Saved games',exact:true}).click();await expect(page.locator('#history-list button')).toHaveCount(1);await page.locator('#close-history').click();
 await page.screenshot({path:path.join(root,'artifacts/screenshots/browser-game.png'),fullPage:true});
 await page.setViewportSize({width:750,height:740});await page.reload();await expect(page.locator('body')).toHaveAttribute('data-core-ready','true');
 expect(await page.evaluate(()=>window.scrollY)).toBe(0);
});
test('all variants start and computer moves as White',async({page})=>{
 for(const v of ['chess','chess960','crazyhouse','kingofthehill','3check','antichess','atomic','horde','racingkings']){
  await page.locator('#variant').selectOption(v);await page.locator('#side').selectOption('black');await page.locator('#level').selectOption('1');await page.locator('#rated').uncheck();await page.getByRole('button',{name:'New game',exact:true}).click();
  await expect(page.locator('#moves button')).toHaveCount(1);await expect(page.locator('#turn-status')).toContainText('Your turn');
  await page.getByRole('button',{name:'Resign',exact:true}).click();await page.locator('#confirm-ok').click();await expect(page.locator('#result')).toBeVisible();
 }
 await expect(page.locator('#error')).toBeHidden();
});
test('all piece sets and boards load, custom textures and 3D',async({page})=>{
 const failures:string[]=[];page.on('response',r=>{if(r.status()>=400)failures.push(r.url());});
 await page.getByRole('button',{name:'New game',exact:true}).click();await page.locator('#appearance summary').click();
 for(const option of await page.locator('#pieces option').evaluateAll(options=>options.map(o=>(o as HTMLOptionElement).value))){await page.locator('#pieces').selectOption(option);await page.waitForTimeout(30);}
 for(const option of await page.locator('#board-theme option').evaluateAll(options=>options.map(o=>(o as HTMLOptionElement).value))){await page.locator('#board-theme').selectOption(option);await page.waitForTimeout(30);}
 await page.locator('#view-mode').selectOption('3d');
 await expect(page.locator('#pieces option')).toHaveCount(11);
 for(const option of await page.locator('#pieces option').evaluateAll(options=>options.map(o=>(o as HTMLOptionElement).value))){await page.locator('#pieces').selectOption(option);await page.waitForTimeout(30);}
 await page.locator('#flip').click();await page.waitForTimeout(100);
 for(const option of await page.locator('#board-theme option').evaluateAll(options=>options.map(o=>(o as HTMLOptionElement).value))){await page.locator('#board-theme').selectOption(option);await page.waitForTimeout(30);}
 expect(failures).toEqual([]);await page.screenshot({path:path.join(root,'artifacts/screenshots/browser-3d.png'),fullPage:true});
});
test('practice takeback, keyboard moves and narrow layout',async({page})=>{
 await page.locator('#rated').uncheck();await page.getByRole('button',{name:'New game',exact:true}).click();await page.locator('#move-input').fill('e2e4');await page.locator('#move-form button').click();await expect(page.locator('#moves button')).toHaveCount(2);
 await page.getByRole('button',{name:'Take back',exact:true}).click();await expect(page.locator('#moves button')).toHaveCount(0);
 await page.setViewportSize({width:1000,height:740});await expect(page.locator('cg-board')).toBeVisible();
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBeTruthy();
 await page.screenshot({path:path.join(root,'artifacts/screenshots/browser-narrow.png'),fullPage:true});
});
