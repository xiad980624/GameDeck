import {useEffect,useState} from 'react';
import {RefreshCw} from 'lucide-react';
import {backend,desktopMode,type EpicArtworkStatus} from '../lib/backend';
import {useDeck} from '../lib/store';
const names:Record<string,string>={poster:'封面',hero:'背景',landscape:'横卡',logo:'Logo'};
export function EpicArtwork(){
 const [status,setStatus]=useState<EpicArtworkStatus>({running:false,report:null});const [error,setError]=useState('');
 const update=()=>backend.epicArtworkStatus().then(setStatus).catch(e=>setError(String(e)));
 useEffect(()=>{let active=true;let unsub:(()=>void)|undefined;const refresh=()=>{if(active)void update();};refresh();void backend.subscribeEpicArtwork(refresh).then(fn=>{if(active)unsub=fn;else fn();});return()=>{active=false;unsub?.();};},[]);
 const start=async()=>{setError('');try{await backend.repairEpicArtwork(true);await update();}catch(e){setError(String(e));}};
 const report=status.report;const remaining=Object.values(report?.games??{}).filter(game=>game.missing.length>0);
 const busy=useDeck(s=>s.busy);
 return <section className="epic-artwork" aria-label="Epic 素材补全">
  <div className="setting-row"><div><h3>Epic 素材补全</h3><p>账号库同步后自动补齐封面、背景和 Logo；已有素材继续保留。</p>{report&&<p role="status">{status.running?`补全中 ${report.processed}/${report.total}`:`上次补全 ${report.saved} 张素材`}{remaining.length?` · ${remaining.length} 款仍有缺项`:''}</p>}</div><button className="btn btn-outline" disabled={!desktopMode||busy||status.running} onClick={()=>void start()}><RefreshCw size={17} className={status.running?'spin':''}/>{status.running?'补全中…':'补全缺失素材'}</button></div>
  {remaining.length>0&&<details className="scrape-status"><summary>查看未完成项</summary><ul>{remaining.map(game=><li key={game.game}>{game.game}：缺少 {game.missing.map(k=>names[k]||k).join('、')}{game.errors.length?`；${game.errors[0]}`:'；尚未找到可靠匹配，可在单款游戏中确认候选'}</li>)}</ul></details>}
  {(error||status.error||report?.errors?.[0])&&<p className="launch-error" role="alert">{error||status.error||report?.errors?.[0]}</p>}
 </section>;
}
