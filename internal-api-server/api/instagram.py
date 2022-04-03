import json
from instagrapi import Client
from instagrapi.exceptions import UserNotFound
import os

from api.social_network_api import SocialNetworkAPI

class InstagramAPI(SocialNetworkAPI):
    def __init__(self) -> None:
        self.instagram = Client()
        self.instagram.login(os.environ["INSTAGRAM_LOGIN"],
                os.environ["INSTAGRAM_PASSWORD"])
        # instagram.set_proxy("socks5://localhost:9050")

    def user_info(self, user_id: str) -> dict:
        try:
            return self.instagram.user_info_by_username(user_id).dict()
        except:
            return None

    def content_types(self):
        return ["stories", "posts"]

    def content(self, user_id: str, content_type, count: int) -> dict:
        try:
            result = self.instagram.user_stories(user_id, count) if content_type == "stories" else self.instagram.user_medias(user_id, count)
            return json.dumps([x.dict() for x in result], default=str)
        except:
            return None

    def content_by_id(self, content_id: str) -> dict:
        try:
            return self.instagram.media_info(self.instagram.media_pk_from_code(content_id)).dict()
        except:
            return None

    def status(self) -> bool:
        return True

