import {LoaderCircle,MonitorPlay,Play} from 'lucide-react';
import {useDeck} from '../lib/store';
import {launchPending} from '../lib/launchState';
import {primaryAction} from '../lib/platforms';
import type {Game} from '../lib/types';

export function LaunchButton({game,onLaunch}:{game:Game;onLaunch:(game:Game)=>void}){
  const session=useDeck(s=>s.sessions[game.id]);
  const pending=launchPending(session);const running=session?.state==='running';
  const failed=session?.state==='failed';
  return <div className="launch-control" data-launch-state={session?.state??'idle'}>
    <button className={`btn btn-primary ${running?'is-running':''}`} disabled={pending||running} aria-busy={pending} onClick={()=>onLaunch(game)}>
      {pending?<LoaderCircle className="spin" size={19}/>:running?<MonitorPlay size={19}/>:<Play size={19} fill="currentColor"/>}
      <span aria-live="polite">{pending?'启动中…':running?'运行中':failed?'重试启动':primaryAction(game)}</span>
    </button>
    {failed&&session.message&&<span className="launch-error" role="alert">{session.message}</span>}
  </div>;
}
