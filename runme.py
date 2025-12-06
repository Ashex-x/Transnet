# runme.py

import logging
import backend.logger_setup
# import app


class Main:
  def __init__(self) -> None:
    # Gets root logger.
    self.logger = backend.logger_setup.setup_logger()
    self.logger.info("Start class: Main")


if __name__ == "__main__":
  run_class = Main()
