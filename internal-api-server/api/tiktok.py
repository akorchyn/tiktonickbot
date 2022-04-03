from TikTokApi import TikTokApi
from TikTokApi.exceptions import TikTokNotFoundError
import json

from api.social_network_api import SocialNetworkAPI

class TikTokAPI(SocialNetworkAPI):
    def __init__(self) -> None:
        self.tiktok = TikTokApi.get_instance(use_test_endgpoints=True, proxy="socks5://localhost:9050")
        self.last_call_was_succeess = True
        
    def user_info(self, user_id: str) -> dict:
        try:
            self.last_call_was_succeess = True
            return self.tiktok.get_user_object(user_id)
        except:
            self.last_call_was_succeess = False
            return None

    def content_types(self):
        return ["videos", "likes"]

    def content(self, user_id: str, content_type, count: int) -> dict:
        try:
            result = self.tiktok.user_liked_by_username(user_id, count) if content_type == "likes" else self.tiktok.by_username(user_id, count)
            self.last_call_was_succeess = True
            return result
        except:
            self.last_call_was_succeess = False
            return None

    def content_by_id(self, content_id: str) -> dict:
        try:
            self.last_call_was_succeess = True
            return json.dumps([self.tiktok.get_tiktok_by_id(id=content_id).get('itemInfo').get('itemStruct')])
        except:
            self.last_call_was_succeess = False
            return None

    def status(self) -> bool:
        return self.last_call_was_succeess