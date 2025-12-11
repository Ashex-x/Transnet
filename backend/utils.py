# utils.py

from typing import Optional, Dict, Any

from . import config
from .api import basic_trans


class PyUtils:
  def __init__(self) -> None:
    self.basic_tran = None

  def total_trans(self, input: str) -> Optional[Dict[str, Any]]:
    """
    Use many methods to process the input and return all the result.

    :param input: User's input
    :type input: str
    :return: the final result

    "traditional": xxx,

    "simplified": xxx,

    "pinyin": xxx,

    "translation": xxx

    :rtype: dict[Any, Any] | None
    """
    backend_config = config.BackendConfig()
    backend_config.config()
    self.basic_tran = basic_trans.BasicTrans()

    return self.basic_tran.trans(input=input)
