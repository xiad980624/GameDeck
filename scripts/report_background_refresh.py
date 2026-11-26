"""Report a completed native refresh; never select artwork or modify the database."""
import collections
import json
from pathlib import Path


def main():
    project = Path(__file__).resolve().parents[1]
    artwork = project / 'release/data/artwork'
    reports = artwork / 'reports'
    result_path = reports / 'background-refresh-latest.json'
    result = json.loads(result_path.read_text(encoding='utf-8'))
    if result['processed'] != result['total'] or result['errors']:
        raise SystemExit('Native refresh has not successfully completed every library entry')
    verification = json.loads((reports / 'background-verification.json').read_text(encoding='utf-8'))
    if len(verification) != result['total'] or not all(x['verified'] for x in verification):
        raise SystemExit('Persisted selections/display caches have not passed native verification')
    snapshots = sorted(reports.glob('background-before-*.json'))
    games = json.loads(snapshots[-1].read_text(encoding='utf-8'))
    names = {game['id']: game['name'] for game in games}
    platforms = {game['id']: {'steam': 'Steam', 'epic': 'Epic', 'battlenet': 'Battle.net', 'manual': '本地'}.get(game['platform'], game['platform']) for game in games}
    counts = collections.Counter()
    missing = []
    rows = []
    for entry in result['entries']:
        if entry['status'] != 'manual-preserved':
            audit = json.loads((artwork / 'hero' / f"{entry['id']}-promo-report.json").read_text(encoding='utf-8'))
            if audit['policy'] != result['policy']:
                raise SystemExit(f"Outdated per-game audit: {entry['game']}")
            entry['audit'] = audit
        else:
            audit = {}
        name = names.get(entry['id'], entry['game'])
        selection = audit.get('selected') or {}
        grade = selection.get('tier', '保留手动图' if entry['status'] == 'manual-preserved' else '本轮未取得')
        counts[grade] += 1
        dimensions = f"{entry['width']}×{entry['height']}" if 'width' in entry else '—'
        reasons = set(audit.get('issues', []))
        for candidate in audit.get('candidates', []):
            if candidate.get('decision') == 'rejected' and candidate.get('reason'):
                reason = candidate['reason']
                width, height = candidate.get('width'), candidate.get('height')
                if isinstance(width, int) and isinstance(height, int):
                    reason += f'（{width}×{height}）'
                if '跨游戏' in reason and candidate.get('title'):
                    reason = f"《{candidate['title']}》：{reason}"
                reasons.add(reason)
        details = '；'.join(sorted(reasons)) or entry.get('reason', '—')
        if grade == '本轮未取得':
            missing.append({'game': name, 'reasons': sorted(reasons)})
        sources = {'background-alphacoders': 'Alpha Coders 缓存原图', 'alphacoders-promo': 'Alpha Coders',
                   'steam-local': 'Steam 本地缓存', 'steam-library': 'Steam 官方库背景',
                   'steam-library-2x': 'Steam 官方 2× 库背景', 'steam-store-keyart': 'Steam 官方宣传封面',
                   'publisher-og': '发行商官网', 'publisher-press': '发行商媒体库', 'sgdb-promo': 'SteamGridDB'}
        source = selection.get('source', '—')
        rows.append([name, platforms.get(entry['id'], '—'), dimensions, grade, sources.get(source, source), selection.get('proof', '—'), details])
    lines = [f"# 整库宣传背景核验（规则 {result['policy']}）", '',
             f"程序已核验 {result['processed']}/{result['total']} 款；数据库与显示缓存回读校验全部通过。", '',
             '；'.join(f'{key}：{value} 款' for key, value in counts.items()), '',
             '以下为本轮程序检索结果，不将请求失败或缺少证据等同于互联网上不存在素材。', '',
             '| 游戏 | 平台 | 采用尺寸 | 档位 | 素材来源 | 采用依据 | 其他候选未采用的原因 |',
             '|---|---|---|---|---|---|---|']
    for row in sorted(rows, key=lambda row: row[0].casefold()):
        lines.append('| ' + ' | '.join(str(cell).replace('|', '/').replace('\n', ' ') for cell in row) + ' |')
    text = '\n'.join(lines) + '\n'
    (reports / 'background-refresh-latest.md').write_text(text, encoding='utf-8')
    (project / 'docs/PROMO_BACKGROUND_RESULTS.md').write_text(text, encoding='utf-8')
    result_path.write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
    (reports / 'background-refresh-summary.json').write_text(json.dumps({'total': result['total'], 'counts': counts, 'missing': missing}, ensure_ascii=False, indent=2), encoding='utf-8')
    print(json.dumps({'verified': len(verification), 'counts': counts, 'missing': missing}, ensure_ascii=False))


if __name__ == '__main__':
    main()
