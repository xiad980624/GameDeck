import { useState } from 'react';
import { FolderOpen, FilePlus2 } from 'lucide-react';
import { backend, type ManualCandidate } from '../lib/backend';
import { useDeck } from '../lib/store';
import { DeckDialog } from './DeckDialog';

export function LocalImport({hidden,onClose}:{hidden:boolean;onClose:()=>void}) {
  const [candidates,setCandidates]=useState<ManualCandidate[]>([]);
  const [selected,setSelected]=useState('');
  const [name,setName]=useState('');
  const [busy,setBusy]=useState(false);
  const [error,setError]=useState('');
  const [warnings,setWarnings]=useState<string[]>([]);
  const choose=async(folder:boolean)=>{
    setBusy(true);setError('');
    try {
      const path=await backend.chooseManual(folder);if(!path)return;
      setCandidates([]);setSelected('');setWarnings([]);
      if(folder){
        const scan=await backend.scanManualFolder(path);
        setCandidates(scan.candidates);setName(scan.name);setWarnings(scan.warnings);
        setSelected(scan.candidates.length===1?scan.candidates[0].path:'');
        if(!scan.candidates.length)setError('未找到可用的游戏 EXE。请选择更具体的游戏文件夹，或直接选择 EXE。');
      } else {
        setCandidates([{path,relative_path:path.split(/[\\/]/).pop()||path,size:0,score:0}]);setSelected(path);
        setName((path.split(/[\\/]/).pop()||'本地游戏').replace(/\.exe$/i,''));
      }
    }catch(error){setError(String(error));}finally{setBusy(false);}
  };
  const save=async()=>{
    if(!selected||!name.trim()||busy)return;
    setBusy(true);setError('');
    try{
      const id=await backend.importManual(selected,name.trim(),hidden);
      const state=useDeck.getState();state.setGames(await backend.listGames());
      state.setQuery('');state.setFilter('all');state.setPlatformFilter('all');state.setFocus(id);
      state.notify(hidden?'游戏已导入隐藏空间':'本地游戏已导入');onClose();
    }catch(error){setError(String(error));}finally{setBusy(false);}
  };
  return <DeckDialog title={hidden?'导入到隐藏空间':'导入本地游戏'} onClose={onClose} busy={busy}>
    <p className="dialog-note">选择一款游戏的文件夹，确认启动程序后导入。游戏文件保留在原位置。</p>
    <div className="dialog-actions"><button className="btn btn-primary" disabled={busy} onClick={()=>void choose(true)}><FolderOpen size={18}/>选择游戏文件夹</button><button className="btn btn-outline" disabled={busy} onClick={()=>void choose(false)}><FilePlus2 size={18}/>直接选择 EXE</button></div>
    {busy&&<p role="status" className="dialog-note">正在处理…</p>}
    {candidates.length>0&&<form onSubmit={event=>{event.preventDefault();void save();}}>
      <label className="dialog-field">游戏名称<input value={name} maxLength={120} disabled={busy} onChange={event=>setName(event.target.value)} required/></label>
      <fieldset className="launch-candidates" disabled={busy}><legend>{candidates.length>1?'找到多个程序，请选择游戏启动程序':'启动程序'}</legend>
        <div className="launch-candidate-list">{candidates.map(candidate=><label key={candidate.path} className={selected===candidate.path?'candidate-picked':''}>
          <input type="radio" name="launch-exe" checked={selected===candidate.path} onChange={()=>setSelected(candidate.path)}/><span title={candidate.path}>{candidate.relative_path}{candidate.size>0&&<small>{(candidate.size/1024/1024).toFixed(1)} MB</small>}</span>
        </label>)}</div>
      </fieldset>
      <div className="dialog-actions"><button className="btn btn-primary" type="submit" disabled={busy||!selected||!name.trim()}>确认导入</button><button className="btn btn-text" type="button" disabled={busy} onClick={onClose}>取消</button></div>
    </form>}
    {warnings.map(warning=><p key={warning} className="dialog-note">{warning}</p>)}
    {error&&<p className="dialog-error" role="alert">{error}</p>}
  </DeckDialog>;
}
