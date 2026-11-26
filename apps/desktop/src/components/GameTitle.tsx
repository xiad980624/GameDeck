import {useState} from 'react';
import {convertFileSrc} from '@tauri-apps/api/core';
import {desktopMode} from '../lib/backend';
import type {Game} from '../lib/types';

/** Keep original title artwork separate from the background. Never redraw or translate it. */
export function GameTitle({game,className='',preparedLogo}:{game:Game;className?:string;preparedLogo?:string|null}){
 const [failed,setFailed]=useState<string|null>(null);
 const logo=(preparedLogo===undefined?game.logo:preparedLogo);
 const available=logo&&failed!==logo?logo:null;
 if(!available)return <h1 className={className}>{game.name}</h1>;
 const src=desktopMode&&/^[A-Za-z]:[\\/]/.test(available)?convertFileSrc(available):available;
 return <h1 className={`${className} game-title-art`}><img src={src} alt={game.name} draggable={false} decoding="async" fetchPriority="high" key={src} onError={()=>setFailed(available)}/></h1>;
}
