from api.social_network_api import SocialNetworkAPI
import requests
from requests.structures import CaseInsensitiveDict
import os

class TwitterAPI(SocialNetworkAPI):
    TWITTER_TWEET_FIELDS = "id,text,attachments,author_id,in_reply_to_user_id,referenced_tweets,source";
    TWITTER_EXPANSIONS = "author_id,attachments.media_keys"
    TWITTER_MEDIA_FIELDS = "preview_image_url,url,media_key"
    TWITTER_URL="https://api.twitter.com/2"
    
    def __init__(self) -> None:
        self.params = {"tweet.fields": TwitterAPI.TWITTER_TWEET_FIELDS, 
                                     "expansions": TwitterAPI.TWITTER_EXPANSIONS, 
                                     "media.fields":TwitterAPI.TWITTER_MEDIA_FIELDS}
        self.last_call_was_succeess = True
        headers = CaseInsensitiveDict()
        headers["Authorization"] = f"Bearer {os.environ.get('TWITTER_API_BEARER_SECRET')}"
        self.headers = headers
        
    def make_request(self, uri: str, params: dict = None) -> dict:
        try:
            data = requests.get(f"{TwitterAPI.TWITTER_URL}/{uri}", params, headers=self.headers).json()
            if data.get("errors"): # Not found
                self.last_call_was_succeess = False
                return None
            self.last_call_was_succeess = True
            return data
        except:
            self.last_call_was_succeess = False
            return None

    def user_info(self, user_id: str) -> dict:
        return self.make_request(f"users/by/username/{user_id}")

    def content_types(self):
        return ["likes", "posts"]

    def content(self, user_id: str, content_type, count: int) -> dict:
        type = "liked_tweets" if content_type == "likes" else "tweets"
        return self.make_request(f"users/{user_id}/{type}?max_results={count}", self.params)

    def content_by_id(self, content_id: str) -> dict:
        return self.make_request(f"tweets/{content_id}", self.params)

    def status(self) -> bool:
        return self.last_call_was_succeess


