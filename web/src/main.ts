import { Chessground } from '@lichess-org/chessground';
import type { Api } from '@lichess-org/chessground/api';
import type { Color, Dests, Key, Role } from '@lichess-org/chessground/types';
import { Chess } from 'chessops/chess';
import { parsePgn, startingPosition } from 'chessops/pgn';
import { parseSan } from 'chessops/san';
import { makeFen } from 'chessops/fen';
import { makeUci } from 'chessops/util';
import './style.css';

interface Options { variant: string; side: Color | 'random'; level: number; minutes: number; increment: number; rated: boolean; fen: string; player: string; chess960: number }
interface Move { uci: string; san: string; fen: string }
interface Legal { uci: string; from: string; to: string; promotion: string; role: string }
interface Summary { id: string; date: string; variant: string; level: number; result: string; rated: boolean; player: string; timeControl: string; plies: number }
interface Rating { value: number; deviation: number; games: number; wins: number; draws: number; losses: number }
interface Progress { pool: string; rating: Rating; levels: {level: number; wins: number; draws: number; losses: number}[]; trend: number[]; calibrated: boolean }
interface Snapshot { revision: string | number; id: string; options?: Options; fen: string; initialFen: string; turn: Color; legal: Legal[]; moves: Move[]; status: string; result: string; reason: string; check: boolean; canClaim: boolean; whiteMs: number; blackMs: number; pgn: string; progress?: Progress; history: Summary[]; preferences: Record<string,string>; players: string[]; analysis: string }
interface Response { kind?: string; requestId: string; ok: boolean; error: string; snapshot?: Snapshot }
interface Catalog { revision: string; pieces: string[]; pieces3d: string[]; boards: {id:string;file:string}[]; boards3d: {id:string;file:string}[] }
interface Host { postMessage(value: unknown): void; addEventListener(name: string, fn: (event: MessageEvent) => void): void }
declare global { interface Window { chrome?: { webview?: Host } } }
const el = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const input = (id: string) => el<HTMLInputElement>(id);
const select = (id: string) => el<HTMLSelectElement>(id);
const text = (id: string, value: string) => { el(id).textContent = value; };
const button = (label: string, click: () => void) => { const b = document.createElement('button'); b.type = 'button'; b.textContent = label; b.onclick = click; return b; };
let state: Snapshot;
let ground: Api;
let catalog: Catalog;
let busy = false;
let orientation: Color = 'white';
let reviewPly = -1;
let lastRendered = '';
let lastGame = '';
let failedRevision = '';
let requestCounter = 0;
let selectedDrop: Role | undefined;
let textSelection = '';
let importedPgn = '';
const pending = new Map<string, { resolve: (response: Response) => void; reject: (error: Error) => void }>();
const roles: Record<string,Role> = {p:'pawn',n:'knight',b:'bishop',r:'rook',q:'queen',k:'king'};
const chars: Record<string,string> = {pawn:'p',knight:'n',bishop:'b',rook:'r',queen:'q',king:'k'};
const names: Record<string,string> = {chess:'Standard chess',chess960:'Chess960',crazyhouse:'Crazyhouse',kingofthehill:'King of the Hill','3check':'Three-check',antichess:'Antichess',atomic:'Atomic',horde:'Horde',racingkings:'Racing Kings'};
const budgets = [100,300,1000,3000,10000,30000,100000,300000];
const depths = [1,2,3,5,7,10,14,18];
const customBoards: Record<string,[string,string]> = {
 'Amber':['#efe1c4','#a47a48'], 'Slate':['#d6dfe4','#6c8290'], 'Sage':['#e5e8d4','#8b9c77'], 'Pine':['#dfebdf','#648978'], 'Clay':['#eee0d4','#b28a79'], 'Lilac':['#e9dfeb','#9b87a8'], 'Plum':['#e9dce6','#98768c'], 'Sand':['#f0e5cf','#b9a277'], 'Fog':['#e3e5e3','#929b99'], 'Rose':['#f0dfe0','#b98d94'], 'Ink':['#d7dce5','#64728d'], 'Warm grey':['#e9e5dd','#a29a8b'], 'Custom':['#f0d9b5','#b58863'],
 'Fine wood':['#e8d4b5','#a7825b'], 'Quiet stone':['#e0e2dd','#8c9690'],
};

function showError(error: unknown) { text('error', error instanceof Error ? error.message : String(error)); el('error').hidden = false; }
el('error').onclick = () => { el('error').hidden = true; };
function raw(action: string, data: Record<string,unknown> = {}): Promise<Response> {
 const host = window.chrome?.webview;
 if (!host) return Promise.reject(new Error('Open Lichess Local from the Windows Start menu. The desktop game service is unavailable.'));
 const requestId = String(++requestCounter);
 return new Promise((resolve,reject) => {
  pending.set(requestId,{resolve,reject});
  host.postMessage({requestId,action,revision:String(state?.revision ?? 0),...data});
 });
}
async function call(action: string, data: Record<string,unknown> = {}, quiet = false) {
 if (!quiet) { el('error').hidden = true; busy = true; renderBoard(); updateControls(); }
 try {
  const response = await raw(action,data);
  return response;
 } catch (e) { if (!quiet) showError(e); throw e; }
 finally { if (!quiet) { busy=false; if(state)render(); renderBoard(); updateControls(); } }
}
function receive(event: MessageEvent) {
 const message = event.data;
 if (message.kind === 'theme') {
  for (const [key,value] of Object.entries(message.values)) document.documentElement.style.setProperty(`--${key}`,String(value));
  document.documentElement.style.colorScheme = message.dark ? 'dark' : 'light';
  document.body.classList.add('ready'); return;
 }
 if (message.kind === 'importPgn') { void importPgn(message.text); return; }
 if (message.kind === 'menu') { el<HTMLButtonElement>(message.id)?.click(); return; }
 if (message.kind === 'fatal') { document.body.classList.add('ready'); showError(message.error); return; }
 if (message.snapshot) {
  const s = message.snapshot as Snapshot;
  s.moves ??= []; s.legal ??= []; s.history ??= []; s.preferences ??= {}; s.players ??= [];
  if (!state || Number(s.revision) >= Number(state.revision)) {
   const playSound=state?.id===s.id&&s.moves.length>state.moves.length&&s.preferences.sound==='true';
   state = s; render();
   if(playSound){const last=s.moves.at(-1);void new Audio(`assets/sound/standard/${last?.san.includes('x')?'Capture':'Move'}.ogg`).play().catch(()=>{});}
  }
 }
 const waiter = pending.get(String(message.requestId));
 if (waiter) { pending.delete(String(message.requestId)); if (message.ok) waiter.resolve(message); else waiter.reject(new Error(message.error || 'The action could not be completed.')); }
}
window.chrome?.webview?.addEventListener('message',receive);

async function confirmAction(title: string, message: string, action: string): Promise<boolean> {
 const dialog = el<HTMLDialogElement>('confirm-dialog'); text('confirm-title',title); text('confirm-text',message); text('confirm-ok',action);
 return new Promise(resolve => { dialog.onclose=()=>resolve(dialog.returnValue==='ok'); dialog.showModal(); });
}
function options(): Options {
 return {variant:select('variant').value,side:select('side').value as Options['side'],level:Number(select('level').value),minutes:Number(select('minutes').value),increment:Number(select('increment').value),rated:input('rated').checked&&!input('start-fen').value.trim(),fen:input('start-fen').value.trim(),player:input('player').value,chess960:Number(input('chess960').value)};
}
async function finishBeforeReplace(): Promise<boolean> {
 if (state?.status !== 'playing' || !state.moves.length) return true;
 if (!await confirmAction('Finish the current game?', 'Starting another game will resign this one. A scored game will count as a loss.','Resign and continue')) return false;
 await call('resign'); return true;
}
el<HTMLFormElement>('setup').onsubmit = event => { event.preventDefault(); void (async()=>{
 if (!await finishBeforeReplace()) return;
 const opts = options(); orientation = opts.side === 'black' ? 'black' : 'white'; reviewPly=-1; failedRevision='';
 await call('new',{options:opts}); orientation = state.options!.side as Color; renderBoard(); await autoComputer();
 })().catch(showError);
};
select('variant').onchange = () => { el('position960').hidden = select('variant').value!=='chess960'; };
for (let level=1;level<=8;level++) select('level').add(new Option(`Level ${level}`,String(level)));
select('level').value='3';
function difficultyDetail() { const level=Number(select('level').value); text('level-detail',`Local level ${level} of 8. Higher levels search more deeply. Your results by level help you choose a difficulty.`); }
select('level').onchange=difficultyDetail; difficultyDetail();

function currentPly() { return reviewPly<0 ? state?.moves.length??0 : reviewPly; }
function displayedFen() { const p=currentPly(); return p===0 ? state?.initialFen : state?.moves[p-1]?.fen; }
function makeGround() {
 ground=Chessground(el('board'),{fen:'start',coordinates:true,disableContextMenu:true,animation:{duration:180},movable:{free:false,events:{after:(from,to)=>{ void chooseMove(from,to); },afterNewPiece:(role,key)=>{const m=state.legal.find(m=>m.role===chars[role]&&m.to===key); if(m)void commit(m.uci);else renderBoard();}}},premovable:{enabled:true},events:{select:key=>{ if(selectedDrop){const m=state.legal.find(m=>m.role===chars[selectedDrop!]&&m.to===key);selectedDrop=undefined;if(m)void commit(m.uci);renderPockets();}}}});
}
async function chooseMove(from: string, to: string) {
 const matches = state.legal.filter(m=>m.from===from&&m.to===to);
 if (!matches.length) { renderBoard(); return; }
 let chosen=matches[0];
 if (matches.length>1) {
  const promotion=el<HTMLDialogElement>('promotion'); const choices=el('promotion-choices'); choices.replaceChildren();
  const result=await new Promise<string|undefined>(resolve=>{
   for(const m of matches) choices.append(button(roles[m.promotion]??m.promotion,()=>{promotion.close();resolve(m.uci);}));
   el('cancel-promotion').onclick=()=>{promotion.close();resolve(undefined);};promotion.oncancel=()=>resolve(undefined);promotion.showModal();
  });
  if(!result){renderBoard();return;} chosen=matches.find(m=>m.uci===result)!;
 }
 await commit(chosen.uci);
}
async function commit(uci: string) {
 try { reviewPly=-1; await call('move',{text:uci}); await autoComputer(); } catch(e){showError(e);renderBoard();}
}
async function autoComputer() {
 if(!state?.options||state.status!=='playing'||state.turn===state.options.side||busy||failedRevision===String(state.revision))return;
 try { await call('computer'); if(state.status==='playing'&&state.turn===state.options!.side)ground.playPremove(); }
 catch(e){if((e as Error).message!=='Cancelled'){failedRevision=String(state.revision);showError(e);}}
 updateControls();
}
function renderBoard() {
 if(!ground||!state?.options)return;
 const live=currentPly()===state.moves.length;
 const dests:Dests=new Map();
 if(live&&state.status==='playing'&&state.turn===state.options.side&&!busy)for(const m of state.legal){if(m.from){const a=dests.get(m.from as Key)??[];if(!a.includes(m.to as Key))a.push(m.to as Key);dests.set(m.from as Key,a);}}
 const last=currentPly()>0?state.moves[currentPly()-1].uci:undefined;
 const lastMove=last&&!last.includes('@')?[last.slice(0,2),last.slice(2,4)] as Key[]:last?.includes('@')?[last.slice(2,4) as Key]:undefined;
 ground.set({fen:displayedFen()||state.fen,orientation,turnColor:state.turn,check:live&&state.check?state.turn:false,lastMove:lastMove,viewOnly:!live||state.status!=='playing',coordinates:state.preferences.coordinates!=='false',movable:{color:state.options.side as Color,dests,rookCastle:state.options.variant==='chess960'},premovable:{enabled:state.status==='playing'&&live},animation:{enabled:!matchMedia('(prefers-reduced-motion: reduce)').matches}});
 renderPockets();renderTextBoard();
}
function render() {
 document.body.dataset.coreReady='true';
 if(!ground)makeGround();
 if(state.id!==lastGame){reviewPly=-1;lastGame=state.id;failedRevision='';
  if(state.options){const o=state.options;input('player').value=o.player;select('variant').value=o.variant;select('side').value=o.side;select('level').value=String(o.level);select('minutes').value=String(o.minutes);select('increment').value=String(o.increment);input('rated').checked=o.rated;input('start-fen').value=o.fen;input('chess960').value=String(o.chess960);orientation=o.side as Color;el('position960').hidden=o.variant!=='chess960';difficultyDetail();}
  applyAppearance();
 }
 const renderKey=`${state.id}:${state.revision}:${currentPly()}`;
 if(renderKey!==lastRendered){lastRendered=renderKey;renderBoard();renderMoves();renderProgress();renderOpening();}
 const o=state.options;
 text('opponent',o?`Fairy-Stockfish 14 · Level ${o.level}`:'Fairy-Stockfish 14');
 text('human-name',o?.player??'Player');
 text('game-info',o?`${names[o.variant]} · ${o.minutes?`${o.minutes}+${o.increment}`:'Untimed'} · ${o.rated?'Scored':'Practice'}`:'Choose a variant and difficulty');
 text('turn-status',busy?'Computer thinking…':state.status==='finished'?state.reason:state.status==='review'?'Imported game · review':state.status==='playing'?`${state.turn===o?.side?'Your':'Computer’s'} turn${state.check?' · Check':''}`:'Ready');
 el('result').hidden=state.status!=='finished'&&state.status!=='review';
 text('result',`${state.result??''} · ${state.reason??''}`);
 const humanMs=o?.side==='black'?state.blackMs:state.whiteMs,computerMs=o?.side==='black'?state.whiteMs:state.blackMs;
 text('human-clock',clock(humanMs,o?.minutes??0));text('computer-clock',clock(computerMs,o?.minutes??0));
 el('human-clock').classList.toggle('active',state.status==='playing'&&state.turn===o?.side);el('computer-clock').classList.toggle('active',state.status==='playing'&&state.turn!==o?.side);
 el('human-clock').classList.toggle('low',!!o?.minutes&&humanMs<20000);
 text('analysis',analysisText(state.analysis));el('analysis').hidden=!state.analysis;
 const players=el('players');players.replaceChildren(...state.players.map(p=>{const option=document.createElement('option');option.value=p;return option;}));
 updateControls();
}
function clock(ms:number,minutes:number) { if(!minutes)return '∞';const total=Math.max(0,Math.ceil(ms/1000));return `${Math.floor(total/60)}:${String(total%60).padStart(2,'0')}`; }
function updateControls(){
 const playing=state?.status==='playing';
 el<HTMLButtonElement>('undo').disabled=busy||!playing||!state?.moves.length||!!state?.options?.rated;
 el<HTMLButtonElement>('resign').disabled=busy||!playing;
 el<HTMLButtonElement>('claim').disabled=busy||!playing||!state?.canClaim;
 el<HTMLButtonElement>('analyze').disabled=busy||!state?.options||playing;
 el<HTMLButtonElement>('continue').disabled=busy||!state?.options||playing;
 el<HTMLButtonElement>('export').disabled=!state?.options||busy;
 el<HTMLButtonElement>('move-input').disabled=busy||!playing||state?.turn!==state.options?.side||currentPly()!==state?.moves.length;
 el('retry').hidden=!playing||state?.turn===state.options?.side||!failedRevision;
 el('cancel').hidden=!busy;
}
function renderMoves(){
 const list=el('moves');list.replaceChildren();
 if(!state.options)return;
 const start=state.initialFen.split(' '),firstNumber=Number(start[5]||1),blackFirst=start[1]==='b';
 state.moves.forEach((m,i)=>{
  const index=i+(blackFirst?1:0);
  if(index%2===0||i===0){const n=document.createElement('span');n.className='number';n.textContent=String(firstNumber+Math.floor(index/2));list.append(n);if(i===0&&blackFirst)list.append(document.createElement('span'));}
  const b=button(m.san,()=>review(i+1));b.classList.toggle('current',currentPly()===i+1);b.setAttribute('aria-label',`Move ${firstNumber+Math.floor(index/2)}, ${index%2===0?'White':'Black'}, ${m.san}`);list.append(b);
 });
 const current=list.querySelector<HTMLElement>('.current');
 if(current)list.scrollTop=Math.max(0,current.offsetTop-list.offsetTop-list.clientHeight/2);
}
function review(ply:number){if(!state?.options)return;reviewPly=Math.max(0,Math.min(ply,state.moves.length));lastRendered='';render();}
el('first').onclick=()=>review(0);el('previous').onclick=()=>review(currentPly()-1);el('next').onclick=()=>review(currentPly()+1);el('last').onclick=()=>review(state.moves.length);
el('flip').onclick=()=>{orientation=orientation==='white'?'black':'white';applyPieceStyle();renderBoard();};
el('undo').onclick=()=>{void call('undo').then(()=>autoComputer()).catch(()=>{});};
el('claim').onclick=()=>{void call('claim').catch(()=>{});};
el('resign').onclick=()=>{void(async()=>{if(await confirmAction('Resign this game?','The computer will win this game.','Resign'))await call('resign');})().catch(showError);};
el('cancel').onclick=()=>{window.chrome?.webview?.postMessage({action:'cancel'});};
el('retry').onclick=()=>{failedRevision='';void autoComputer();};
el('analyze').onclick=()=>{void call('analyze',{index:currentPly()}).catch(()=>{});};
el('continue').onclick=()=>{input('start-fen').value=displayedFen();input('rated').checked=false;select('side').value=(displayedFen().split(' ')[1]==='b'?'black':'white');el<HTMLFormElement>('setup').requestSubmit();};
el<HTMLFormElement>('move-form').onsubmit=e=>{e.preventDefault();const value=input('move-input').value.trim();if(value){input('move-input').value='';void commit(value);}};
el('app-settings').onclick=()=>{void raw('settings').catch(showError);};
el('export').onclick=()=>{void raw('savePgn',{text:state.pgn}).catch(showError);};
el('import').onclick=()=>{void(async()=>{if(await finishBeforeReplace())await raw('pickPgn');})().catch(showError);};
el('about').onclick=()=>el<HTMLDialogElement>('about-dialog').showModal();el('close-about').onclick=()=>el<HTMLDialogElement>('about-dialog').close();
el('show-history').onclick=()=>{
 const list=el('history-list');list.replaceChildren();
 if(!state.history.length)list.textContent='Completed games appear here.';
 for(const game of state.history){const b=button('',()=>{void(async()=>{if(!await finishBeforeReplace())return;await call('history',{text:game.id});el<HTMLDialogElement>('history-dialog').close();})().catch(showError);});
  for(const value of [new Date(Number(game.date)).toLocaleDateString(),`${names[game.variant]} · ${game.player}`,`Level ${game.level}`,game.result]){const span=document.createElement('span');span.textContent=value;b.append(span);}list.append(b);
 }el<HTMLDialogElement>('history-dialog').showModal();
};el('close-history').onclick=()=>el<HTMLDialogElement>('history-dialog').close();
function renderProgress(){
 const p=state.progress;if(!p){el('progress').hidden=true;return;}el('progress').hidden=false;
 const r=p.rating;const games=r.games??0;
 text('pool',`${state.options!.player} · ${p.pool}`);text('score',`${Math.round(r.value)}${r.deviation>110?'?':''}`);
 text('score-details',`${games} scored games · ${r.wins??0} wins, ${r.draws??0} draws, ${r.losses??0} losses · uncertainty ±${Math.round(r.deviation*2)}`);
 const svg=el('trend');svg.replaceChildren();
 const points=p.trend?.slice(-40)??[];
 svg.style.display=points.length>1?'block':'none';
 if(points.length>1){const min=Math.min(...points)-40,max=Math.max(...points)+40;const line=document.createElementNS('http://www.w3.org/2000/svg','polyline');line.setAttribute('points',points.map((p,i)=>`${4+i/(points.length-1)*252},${60-(p-min)/(max-min)*56}`).join(' '));svg.append(line);}
 const tbody=el('level-results').querySelector('tbody')!;tbody.replaceChildren();for(const l of p.levels??[]){const tr=document.createElement('tr');tr.classList.toggle('current',l.level===state.options!.level);for(const val of [l.level,l.wins??0,l.draws??0,l.losses??0]){const td=document.createElement('td');td.textContent=String(val);tr.append(td);}tbody.append(tr);}
}
function parseBoard(fen:string){const cells=new Map<string,string>();let rank=8,file=0;for(const c of (fen?.split(' ')[0]??'').split('[')[0]){if(c==='/'){rank--;file=0;}else if(/[1-8]/.test(c))file+=Number(c);else if(c!=='~'){cells.set(`${String.fromCharCode(97+file)}${rank}`,c);file++;}}return cells;}
function renderTextBoard(){
 const container=el('text-board');container.replaceChildren();const table=document.createElement('table');const board=parseBoard(displayedFen());
 for(let r=0;r<8;r++){const tr=document.createElement('tr');for(let f=0;f<8;f++){const file=orientation==='white'?f:7-f,rank=orientation==='white'?8-r:r+1,key=`${String.fromCharCode(97+file)}${rank}`,piece=board.get(key);const td=document.createElement('td');const label=piece?`${piece===piece.toUpperCase()?'White':'Black'} ${roles[piece.toLowerCase()]}`:'empty';const b=button(`${key} ${piece??'·'}`,()=>{if(textSelection){void chooseMove(textSelection,key);textSelection='';}else textSelection=key;});b.setAttribute('aria-label',`${key}, ${label}`);td.append(b);tr.append(td);}table.append(tr);}container.append(table);
}
function renderPockets(){
 for(const [id,color] of [['pocket-top',state?.options?.side==='white'?'black':'white'],['pocket-bottom',state?.options?.side??'white']] as const){const host=el(id);host.hidden=state?.options?.variant!=='crazyhouse';host.replaceChildren();if(host.hidden)continue;const pocket=displayedFen().split('[')[1]?.split(']')[0]??'';
  for(const char of ['p','n','b','r','q']){const c=color==='white'?char.toUpperCase():char,count=[...pocket].filter(x=>x===c).length;const b=button('',()=>{if(color===state.options!.side&&count>0){selectedDrop=roles[char];renderPockets();}});b.disabled=count===0||color!==state.options!.side||state.turn!==color||busy||state.status!=='playing';b.classList.toggle('selected',selectedDrop===roles[char]);b.setAttribute('aria-label',`${color} ${roles[char]}, ${count} in pocket`);const image=document.createElement('span');image.className='piece-preview';image.style.backgroundImage=`url('${pieceUrl(color as Color,roles[char])}')`;b.append(image,document.createTextNode(String(count)));b.onpointerdown=e=>{if(!b.disabled&&e.button===0)ground.dragNewPiece({color:color as Color,role:roles[char]},e);};host.append(b);}
 }
}
let preferenceQueue:Promise<unknown>=Promise.resolve();
function preference(key:string,value:string):Promise<unknown>{const command=preferenceQueue.then(async()=>{if(!state)return;await call('preference',{key,value},true);applyAppearance();});preferenceQueue=command.catch(showError);return command;}
function fillAppearance(){
 const three=select('view-mode').value==='3d';const pieceSelect=select('pieces'),boardSelect=select('board-theme');pieceSelect.replaceChildren();boardSelect.replaceChildren();
 for(const name of three?catalog.pieces3d:catalog.pieces)pieceSelect.add(new Option(name.replaceAll('-',' '),name));
 for(const board of three?catalog.boards3d:catalog.boards)boardSelect.add(new Option(board.id.replaceAll('-',' '),`upstream:${board.id}`));
 if(!three)for(const name of Object.keys(customBoards))boardSelect.add(new Option(name,`local:${name}`));
 const key=three?'pieces3d':'pieces',bkey=three?'board3d':'board';pieceSelect.value=state?.preferences[key]??(three?'Basic':'cburnett');boardSelect.value=state?.preferences[bkey]??(three?'upstream:Woodi':'upstream:brown');
 if(pieceSelect.selectedIndex<0)pieceSelect.selectedIndex=0;if(boardSelect.selectedIndex<0)boardSelect.selectedIndex=0;
}
function pieceUrl(color:Color,role:Role){const three=select('view-mode').value==='3d',set=select('pieces').value||'cburnett';if(three){const flipped=orientation==='black'&&set!=='Staunton'&&(role==='knight'||role==='bishop')?'-Flipped':'';return `assets/staunton/piece/${encodeURIComponent(set)}/${color==='white'?'White':'Black'}-${role[0].toUpperCase()+role.slice(1)}${flipped}.webp`;}
 const prefix=color==='white'?'w':'b';
 return `assets/piece/${encodeURIComponent(set)}/${set==='disguised'?prefix+'.svg':prefix+chars[role].toUpperCase()+(set==='monarchy'?'.webp':'.svg')}`;
}
const pieceStyle=document.createElement('style');document.head.append(pieceStyle);
function applyPieceStyle(){let css='';for(const color of ['white','black'] as Color[])for(const role of Object.values(roles))css+=`#board piece.${color}.${role}{background-image:url("${pieceUrl(color,role)}")}\n`;pieceStyle.textContent=css;}
function noiseTexture(light:string,dark:string,wood:boolean){
 const canvas=document.createElement('canvas');canvas.width=canvas.height=1024;const ctx=canvas.getContext('2d')!;
 const parse=(c:string)=>[parseInt(c.slice(1,3),16),parseInt(c.slice(3,5),16),parseInt(c.slice(5,7),16)];const a=parse(light),b=parse(dark),image=ctx.createImageData(1024,1024);let random=173;
 for(let y=0;y<1024;y++)for(let x=0;x<1024;x++){random=(Math.imul(random,1664525)+1013904223)|0;const grain=wood?(Math.sin(x*.12+Math.sin(y*.005)*3+Math.sin(x*.023+y*.003)*2)*2.4):Math.sin(x*.011+y*.016+Math.sin(y*.018)*2)*2;const shade=grain+((random>>>24)/255-.5)*2;const base=(Math.floor(x/128)+Math.floor(y/128))%2?b:a;const i=(y*1024+x)*4;for(let c=0;c<3;c++)image.data[i+c]=base[c]+shade;image.data[i+3]=255;}ctx.putImageData(image,0,0);return canvas.toDataURL('image/png');
}
const textureCache=new Map<string,string>();
function applyAppearance(){
 if(!catalog)return;
 select('view-mode').value=state?.preferences.view??'2d';fillAppearance();
 input('coordinates').checked=state?.preferences.coordinates!=='false';input('sound').checked=state?.preferences.sound==='true';
 input('light-square').value=state?.preferences.light??'#f0d9b5';input('dark-square').value=state?.preferences.dark??'#b58863';
 applyPieceStyle();const board=el('board');const three=select('view-mode').value==='3d';board.classList.toggle('three-dimensional',three);
 const cg=board.querySelector<HTMLElement>('cg-board');if(!cg)return;
 const value=select('board-theme').value;
 if(value.startsWith('upstream:')){const item=(three?catalog.boards3d:catalog.boards).find(b=>b.id===value.slice(9));cg.style.backgroundImage=item?`url("assets/${three?'staunton/board':'board'}/${item.file}")`:'';cg.style.backgroundSize='100% 100%';}
 else {const name=value.slice(6);let [light,dark]=customBoards[name]??customBoards.Custom;if(name==='Custom'){light=input('light-square').value;dark=input('dark-square').value;}
  board.style.setProperty('--light',light);board.style.setProperty('--dark',dark);
  if(name==='Fine wood'||name==='Quiet stone'){const key=`${name}:${light}:${dark}`;if(!textureCache.has(key))textureCache.set(key,noiseTexture(light,dark,name==='Fine wood'));cg.style.backgroundImage=`url("${textureCache.get(key)}")`;cg.style.backgroundSize='100% 100%';}
  else {cg.style.backgroundImage='';cg.style.backgroundSize='25% 25%';}
 }
 renderBoard();
}
select('view-mode').onchange=()=>{void preference('view',select('view-mode').value);};
select('pieces').onchange=()=>{void preference(select('view-mode').value==='3d'?'pieces3d':'pieces',select('pieces').value);};
select('board-theme').onchange=()=>{void preference(select('view-mode').value==='3d'?'board3d':'board',select('board-theme').value);};
for(const [id,key] of [['coordinates','coordinates'],['sound','sound']])input(id).onchange=()=>{void preference(key,String(input(id).checked));};
for(const [id,key] of [['light-square','light'],['dark-square','dark']])input(id).onchange=()=>{void(async()=>{await preference(key,input(id).value);await preference('board','local:Custom');})();};
const openingNames=new Map<string,string>();
function keyFen(f:string){return f.split(' ').slice(0,4).join(' ');}
async function loadOpenings(){
 const files=await Promise.all(['a','b','c','d','e'].map(c=>fetch(`openings/${c}.tsv`).then(r=>{if(!r.ok)throw new Error('Opening names could not be loaded');return r.text();})));
 for(const file of files)for(const line of file.trim().split('\n').slice(1)){const [eco,name,pgn]=line.trimEnd().split('\t');const pos=Chess.default();let valid=true;for(const token of pgn.replace(/\d+\./g,' ').trim().split(/\s+/)){const m=parseSan(pos,token);if(!m){valid=false;break;}pos.play(m);}if(valid)openingNames.set(keyFen(makeFen(pos.toSetup())),`${eco} · ${name}`);}
 renderOpening();
}
function renderOpening(){text('opening','');if(!state?.options||state.options.variant!=='chess')return;const fens=[state.initialFen,...state.moves.slice(0,currentPly()).map(m=>m.fen)];for(const f of fens.reverse()){const name=openingNames.get(keyFen(f));if(name){text('opening',name);break;}}}
async function importPgn(source:string){
 try{
 const games=parsePgn(source);if(!games.length)throw new Error('No game found in the PGN.');
 let selected=0;if(games.length>1){const choice=await confirmAction('Open the first game?',`This file contains ${games.length} games. The first game will open for review. Use a PGN editor to select another game.`,'Open first game');if(!choice)return;}
 const game=games[selected];const pos=startingPosition(game.headers).unwrap();const initial=makeFen(pos.toSetup());const moves:string[]=[];
 for(const node of game.moves.mainline()){const move=parseSan(pos,node.san);if(!move)throw new Error(`Illegal PGN move: ${node.san}`);moves.push(makeUci(move));pos.play(move);}
 const variants:Record<string,string>={'standard':'chess','chess960':'chess960','crazyhouse':'crazyhouse','king of the hill':'kingofthehill','three-check':'3check','antichess':'antichess','atomic':'atomic','horde':'horde','racing kings':'racingkings'};
 const opts:Options={...options(),variant:variants[(game.headers.get('Variant')??'Standard').toLowerCase()]??'chess',fen:initial,rated:false,side:'white',minutes:0};
 await call('import',{text:JSON.stringify({options:opts,moves,result:game.headers.get('Result')??'*'})});importedPgn=source;
 }catch(e){showError(e);}
}
function analysisText(raw:string){if(!raw)return '';const score=raw.match(/score (cp|mate) (-?\d+)/),pv=raw.match(/ pv (.*)/),depth=raw.match(/depth (\d+)/);let label=score?score[1]==='mate'?`Mate ${score[2]}`:`${Number(score[2])/100>=0?'+':''}${(Number(score[2])/100).toFixed(2)}`:'No score';return `${label} (side to move) · depth ${depth?.[1]??'?'}\n${pv?.[1]??raw.split('\n').at(-1)}`;}
document.addEventListener('keydown',e=>{if(e.target instanceof HTMLInputElement||e.target instanceof HTMLTextAreaElement||e.target instanceof HTMLSelectElement||document.querySelector('dialog[open]'))return;if(e.key==='ArrowLeft'){e.preventDefault();review(currentPly()-1);}if(e.key==='ArrowRight'){e.preventDefault();review(currentPly()+1);}if(e.key==='Home'){e.preventDefault();review(0);}if(e.key==='End'){e.preventDefault();review(state.moves.length);}if(e.key==='f'){el('flip').click();}if(e.key==='Escape'){selectedDrop=undefined;ground.cancelPremove();renderPockets();}});

async function init(){
 try{catalog=await fetch('assets/catalog.json').then(r=>r.json());fillAppearance();await raw('snapshot');applyAppearance();void loadOpenings().catch(showError);await autoComputer();
 setInterval(()=>{if(!busy&&state?.status==='playing')void call('snapshot',{},true).catch(showError);},500);
 }catch(e){document.body.classList.add('ready');showError(e);}
}
void init();
