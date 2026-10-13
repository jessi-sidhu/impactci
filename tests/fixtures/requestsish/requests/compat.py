import importlib

from urllib.parse import urlparse

for lib in ("chardet", "charset_normalizer"):
    try:
        chardet = importlib.import_module(lib)
        break
    except ImportError:
        pass

str = str
