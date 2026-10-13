from __future__ import annotations

import typing as t

import click

from . import json
from .globals import g
from .sansio.app import App

if t.TYPE_CHECKING:
    from .testing import FlaskClient


class Flask(App):
    pass
