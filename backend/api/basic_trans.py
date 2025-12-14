# test_api.py

from typing import Dict, Any, Optional, Union, List
import pandas as pd
import os

import logging


class BasicTrans:
  """
  Basic translation system for just simple translation.

  This will only use basic data base(cedict.u8 -> cedict.csv).

  Use trans() to get basic translation.
  """
  def __init__(self) -> None:
    self.basic_db_cedict = None
    self.logger = logging.getLogger()
    self.output_dict: dict[str, Union[str, List[str]]] = {}

    # Init database: cedict.csv
    current_dir = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(current_dir, "../../database/processed_data/cedict.csv")

    try:
      self.basic_db_cedict = pd.read_csv(path)  # type: ignore
      self.logger.debug("loads csv successfully")
    except FileNotFoundError as e:
      self.logger.error(f"{e}\nDid you process raw data and save it in the"
                        f"right directory?")
    except Exception as e:
      self.logger.error(f"{e}")

  def trans(self, input: str) -> Optional[Dict[str, Any]]:
    """
    Use basic dictory to translate the input text.

    :param input: the string that you want translate.
    :type input: str
    :return: the translation result
    :rtype: dict.traditional: str
            dict.simplified: str
            dict.pinyin: List
            dict.Translation: List
    """
    if isinstance(self.basic_db_cedict, pd.DataFrame):
      result = self.basic_db_cedict[
        self.basic_db_cedict['simplified'] == input]

      if not result.empty:
        # --- Input is a word ---
        if not result.iloc[1].empty:

          self.logger.warning("There are two or more result from cedict.csv")

        translation_raw = str(result.iloc[0, 3])
        translation = [item for item in
                       translation_raw.strip('/').split('/') if item]

        self.output_dict = {
          "tranditional": str(result.iloc[0, 0]),
          "simplified": str(result.iloc[0, 1]),
          "pinyin": str(result.iloc[0, 2]),
          "translation": translation
          }

        self.logger.debug(f"Translation successful: \n{self.output_dict}")
      elif input:
        # --- Input is a phrase or sentence ---
        pass

      return self.output_dict

    self.logger.critical("TypeError: Check your database: basic cedict")
    return None

  def _words_trans(self) -> Optional[Dict[str, Any]]:
    pass
