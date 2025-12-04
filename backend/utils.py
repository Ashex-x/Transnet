# utils.py

import config
import api.basic_trans


class PyUtils:
  def __init__(self) -> None:
    pass

  def py_utils(self) -> None:
    basic_trans = api.basic_trans.BasicTrans()
    basic_trans.trans()
