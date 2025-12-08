# utils.py

from . import config
from .api import basic_trans


class PyUtils:
  def __init__(self) -> None:
    pass

  def py_utils(self) -> None:
    backend_config = config.BackendConfig()
    backend_config.config()
    basic_tran = basic_trans.BasicTrans()
    basic_tran.trans("你好")
