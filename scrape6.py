import json
import os
import sys
import requests

mee6_auth = os.getenv("MEE6_AUTH")
guild = sys.argv[1]
last_len = 1
page = 0
f = open("dump.sql", 'w+')
headers = {
    "Authorization": mee6_auth
}
while last_len != 0:
    r = requests.get(f"https://mee6.xyz/api/plugins/levels/leaderboard/{guild}?page={page}", headers=headers).json()
    last_len = len(r["players"])
    page += 1
    for item in r["players"]:
        id = item["id"]
        xp = item["xp"]
        f.write(
            f'INSERT INTO levels (id, xp, guild) VALUES ({id}, {xp}, {guild}) ON CONFLICT (id, guild) DO UPDATE SET xp = EXCLUDED.xp + {xp};\n')
f.close()
