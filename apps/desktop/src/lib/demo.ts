import type { Game } from './types';

const defs: [string,string,string,string][] = [
  [
    "Cyberpunk 2077",
    "1091500",
    "角色扮演 · 开放世界",
    "#dbdf49"
  ],
  [
    "ELDEN RING",
    "1245620",
    "动作角色扮演 · 奇幻",
    "#c9b48d"
  ],
  [
    "Black Myth: Wukong",
    "2358720",
    "动作 · 神话冒险",
    "#e9b982"
  ],
  [
    "Red Dead Redemption 2",
    "1174180",
    "开放世界 · 西部",
    "#de614c"
  ],
  [
    "Hades",
    "1145360",
    "动作 · Roguelike",
    "#f75e56"
  ],
  [
    "Baldur’s Gate 3",
    "1086940",
    "角色扮演 · 策略",
    "#d9a876"
  ],
  [
    "Hollow Knight",
    "367520",
    "银河恶魔城 · 探索",
    "#b8c9dd"
  ],
  [
    "Forza Horizon 5",
    "1551360",
    "竞速 · 开放世界",
    "#df9a67"
  ],
  [
    "Stardew Valley",
    "413150",
    "模拟经营 · 休闲",
    "#b6c879"
  ],
  [
    "The Witcher 3",
    "292030",
    "角色扮演 · 冒险",
    "#9ba8b8"
  ],
  [
    "Sekiro: Shadows Die Twice",
    "814380",
    "动作 · 忍者",
    "#cba58a"
  ],
  [
    "God of War",
    "1593500",
    "动作 · 剧情",
    "#9dbbc4"
  ],
  [
    "Control",
    "870780",
    "动作 · 超自然",
    "#c78088"
  ],
  [
    "Slay the Spire",
    "646570",
    "卡牌 · Roguelike",
    "#caa2d4"
  ]
];

export const DEMO_GAMES: Game[] = defs.map(([name, appId, genre, accent], i) => ({
  id: 'demo-' + appId, name, description: '', genre, platform: 'steam', external_id: appId,
  launch_installation_id: null, installed: false, favorite: i < 4,
  total_minutes: 0, last_played_at: null,
  added_at: new Date(Date.now() - i * 86_400_000).toISOString(),
  poster: null, hero: null, accent,
}));

export function getPoster(game: Game): string {
  if (game.poster) return game.poster;
  if (game.external_id && game.platform === 'steam' && /^\d+$/.test(game.external_id)) {
    return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.external_id}/library_600x900.jpg`;
  }
  return '/art/fallback-poster.svg';
}
export function getHero(game: Game): string {
  if (game.hero) return game.hero;
  if (game.external_id && game.platform === 'steam' && /^\d+$/.test(game.external_id)) {
    return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.external_id}/library_hero.jpg`;
  }
  return '/art/fallback-hero.svg';
}
export function getLandscape(game: Game): string {
  return game.landscape || getHero(game);
}
export function getFallbackPoster(game: Game): string {
  const n = Math.abs([...game.name].reduce((s, c) => s + c.charCodeAt(0), 0) % 6);
  return `/art/abstract-${n + 1}.svg`;
}
