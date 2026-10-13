from .compat import urlparse
from .utils import to_key_val_list


class Session:
    def get(self, url):
        return urlparse(url)
