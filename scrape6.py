import json
import os
import sys
import requests

mee6_auth = os.getenv("MEE6_AUTH")
guild = sys.argv[1]
last_len = 1
page = 0

headers = {
    "Authorization": mee6_auth
}

players = []

while last_len != 0:
    r = requests.get(f"https://mee6.xyz/api/plugins/levels/leaderboard/{guild}?page={page}", headers=headers).json()
    last_len = len(r["players"])
    page += 1
    for item in r["players"]:
        id = item["id"]
        xp = item["xp"]
        players.append({
            "id": id,
            "xp": xp
        })

json.dump(players, sys.stdout)
