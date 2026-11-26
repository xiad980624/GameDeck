import sys, json, urllib.request, urllib.parse
key = sys.stdin.readline().strip()
try:
    query = urllib.parse.urlencode(dict(auth=key, content_type='wallpaper', term='Diablo IV', width=3840, height=2160, operator='min'))
    request = urllib.request.Request('https://api.alphacoders.com/4.0/search?' + query, headers={'User-Agent':'GameDeck/1.0'})
    result = json.load(urllib.request.urlopen(request, timeout=30))
    print(json.dumps(result, ensure_ascii=True)[:18000])
except Exception as error:
    print(type(error).__name__, getattr(error, 'code', 'connection_failed'))
