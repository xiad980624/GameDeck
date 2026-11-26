import {useEffect,useState} from 'react';
import {getHero} from '../lib/demo';
import {peekHero,prepareHero} from '../lib/artwork';
import type {Game} from '../lib/types';
import {MotionImage} from './MotionImage';
export function HeroImage({game,className=''}:{game:Game;className?:string}){
 const key=getHero(game);const ready=peekHero(game);
 const [loaded,setLoaded]=useState(()=>({key,url:ready?.src||''}));
 useEffect(()=>{let active=true;void prepareHero(game).then(img=>{if(active)setLoaded({key,url:img.src});}).catch(()=>undefined);return()=>{active=false;};},[game.id,key]);
 return <MotionImage play={false} className={className} src={ready?.src||(loaded.key===key?loaded.url:'')||'/art/fallback-hero.svg'} alt="" draggable={false} decoding="async" fetchPriority="high"/>;
}
