# logger_setup.py

import logging
import typing
import sys
from pathlib import Path
from datetime import datetime


def setup_logger(name: typing.Optional[str] = None):
  """
  Setup logger

  Args:
    name: logger name. When 'None', return root logger.

  Returns:
    logging.Logger
  """
  # ==================== Create logger ====================
  # Get logger instance, if name is None, get root logger
  logger = logging.getLogger(name)

  # Avoid duplicate definition
  if logger.handlers:
    return logger

  # ==================== Set log level ====================
  # DEBUG < INFO < WARNING < ERROR < CRITICAL
  logger.setLevel(logging.DEBUG)

  # ==================== Create formatter ====================
  formatter = logging.Formatter(
    # Format String Description：
    # %(asctime)s   : Log time, format specified by datefmt
    # %(name)-15s   :
    # Logger name, -15 represents left aligned width of 15 characters
    # %(levelname)-8s:
    # Log level, -8 represents left aligned width of 8 characters
    # %(filename)s  : File name (excluding path)
    # %(lineno)d    : Line number
    # %(funcName)s  : Function name
    # %(message)s   : Log message
    fmt='%(asctime)s | %(name)-10s | %(levelname)-8s |'
    '%(filename)s:%(lineno)d | %(message)s',

    # Date format
    datefmt='%Y-%m-%d %H:%M:%S'
  )

  # ==================== Create Handlers ====================

  # -------------------- Console Handler --------------------
  console_handler = logging.StreamHandler(sys.stdout)
  console_handler.setLevel(logging.INFO)
  console_handler.setFormatter(formatter)

  # Set name
  console_handler.name = "console_handler"

  # -------------------- File Handler --------------------
  # Create directory
  log_dir = Path("logs")
  log_dir.mkdir(exist_ok=True)

  # Basic file handler
  file_handler = logging.FileHandler(
    filename=log_dir / f"app_{datetime.now().strftime('%Y%m%d')}.log",
    mode='a',  # add pattern ('a' indicate append)
    encoding='utf-8'
  )
  file_handler.setLevel(logging.DEBUG)
  file_handler.setFormatter(formatter)
  file_handler.name = "file_handler"

  # ==================== Add Handlers to logger ====================
  logger.addHandler(console_handler)
  logger.addHandler(file_handler)

  # ==================== Pass down ====================
  # propagate=True: The log will be propagated to the parent logger (default)
  # propagate=False: The log will not propagate to the parent logger
  logger.propagate = False

  return logger
