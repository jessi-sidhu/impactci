from . import sessions


def get(url):
    return sessions.Session().get(url)
