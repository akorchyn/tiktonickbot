from TikTokApi import TikTokApi
import json
import sys

if  len(sys.argv) != 3:
    exit(0)

api = TikTokApi.get_instance()

print(json.dumps(api.by_username(sys.argv[1], int(sys.argv[2]))))