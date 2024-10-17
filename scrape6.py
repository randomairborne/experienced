import json
import os
import sys

import urllib.request

mee6_auth = os.getenv("MEE6_AUTH")
if mee6_auth is None:
    print("Please set MEE6_AUTH environment variable to the Authorization token from mee6.xyz", file=sys.stderr)
    sys.exit(1)

if len(sys.argv) != 2:
    print("Usage: python3 scrape6.py <url> > export.json", file=sys.stderr)
    sys.exit(1)
guild = sys.argv[1]

last_len = 1
page = 0

headers = {
    "Authorization": mee6_auth,
    "Referer": f"https://mee6.xyz/en/leaderboard/{guild}",
    "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:131.0) Gecko/20100101 Firefox/131.0"
}

players = []

while last_len != 0:
    url = f"https://mee6.xyz/api/plugins/levels/leaderboard/{guild}?page={page}&limit=1000"
    req = urllib.request.Request(url, headers=headers)
    resp = urllib.request.urlopen(req)
    apiresp = json.loads(resp.read())
    last_len = len(apiresp["players"])
    page += 1
    for item in apiresp["players"]:
        id = item["id"]
        xp = item["xp"]
        players.append({
            "id": id,
            "xp": xp
        })

json.dump(players, sys.stdout)
