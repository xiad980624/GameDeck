import type {SessionInfo} from './types';
export const launchPending=(session?:SessionInfo)=>session?.state==='preparing'||session?.state==='awaiting_game';
export function nextSession(previous:SessionInfo|undefined,next:SessionInfo):SessionInfo|undefined{
  if (!previous || previous.session_id !== next.session_id) return previous;
  if (previous.state === 'exited' || previous.state === 'failed') return previous;
  if (previous.state === 'running' && launchPending(next)) return previous;
  if (previous.state === next.state && previous.message === next.message) return previous;
  return next;
}
