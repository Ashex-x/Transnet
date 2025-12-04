# app.py

import flask
import time

import backend.utils

import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

app = flask.Flask(__name__)


class Bridge:
  def __init__(self) -> None:
    self.utils = backend.utils.PyUtils()

  def main(self) -> None:
    pass


if __name__ == "__main__":
  bridge = Bridge()
