## tiktonickbot

![Build](https://github.com/akorchyn/tiktonickbot/actions/workflows/rust.yml/badge.svg)

Tiktonick is a bot that would help you to follow users on a variety of social media. 
The idea is to aggregate a variety of accounts into one personal feed, so you would be able to subscribe and get one feed.

#### Currently supported media:
* Tiktok
* Twitter (Except videos from Twitter, unfortunately, the current version of the Twitter V2 API doesn't support it)

#### Planned media:
* Instagram
* Telegram channels
* Youtube


### Features
The bot expects that content is publicly available. The bot doesn't use any exploits or hacks. All the data is publicly available.

* User content subscription 
* User likes subscription
* Convertation of the links to posts/videos from Twitter, TikTok to messages with downloaded media
* Fetching last N likes/posts from given user on a command basis  

### Commands
These commands are supported:
* /help - display this text.
* /subscriptions - shows subscriptions for this chat
* /tweet - sends last tweet for given user.
* /tweets - sends last n tweets for given user.
* /ltweet - sends last liked tweet for given user.
* /ltweets - sends last n liked tweets for given user.
* /ltiktok - sends last like for given user.
* /ltiktoks - sends last n likes for given user.
* /tiktok - sends last video for given user.
* /tiktoks - sends last n videos for given user.
* /sub_tiktok_likes - subscribe chat to tiktok user likes feed.
* /sub_tiktok - subscribe chat to tiktok user likes feed.
* /unsub_tiktok_likes - unsubscribe chat from tiktok user video feed.
* /unsub_tiktok - unsubscribe chat from tiktok user video feed.
* /sub_twitter_likes - subscribe chat to tiktok user likes feed.
* /sub_twitter - subscribe chat to tiktok user likes feed.
* /unsub_twitter_likes - unsubscribe chat from tiktok user video feed.
* /unsub_twitter - unsubscribe chat from tiktok user video feed.

#### Command format:
* Subscription/Unsubscription commands. Require one argument. Login account.
Example: /sub_tiktok tiktokaccountname
* Commands that fetch last post. Require one argument. It's account login.
Example: /ltweet twitteraccountname
* Commands that fetch last N posts. Require two arguments. It's account login and amount of posts.
Example: /ltweet twitteraccountname 1 (The same as previous. It will return last post)


### Bottleneck
Currently, as I know, Twitter is the only social media that have a publicly available API. 
All others media don't have them, so we need to use tools that reverse engineer them. 
Unfortunately, Tiktok(and probably many others) is not an easy target  and tries to block data scrapping, so we require a proxy. 
The current implementation uses the TOR network for that, but it requires finding an IP that would work, so data receiving time is not guaranteed.  
