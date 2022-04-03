from abc import ABC, abstractmethod

class SocialNetworkAPI(ABC):
    @abstractmethod
    def user_info(self, user_id: str) -> dict:
        pass

    @abstractmethod
    def content_types(self):
        pass

    @abstractmethod
    def content(self, user_id: str, content_type, count: int) -> dict:
        pass

    @abstractmethod
    def content_by_id(self, content_id: str) -> dict:
        pass

    @abstractmethod
    def status(self) -> bool:
        pass