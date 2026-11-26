export type AcceleratorId = 'uu'|'xunyou';
const key='gamedeck-game-accelerators';
export function gameAccelerator(id:string):AcceleratorId|'' { try {const value=JSON.parse(localStorage.getItem(key)||'{}')[id];return value==='uu'||value==='xunyou'?value:'';}catch{return '';} }
export function saveGameAccelerator(id:string,provider:AcceleratorId|''){let values:Record<string,string>={};try{values=JSON.parse(localStorage.getItem(key)||'{}');}catch{}if(provider)values[id]=provider;else delete values[id];localStorage.setItem(key,JSON.stringify(values));}
