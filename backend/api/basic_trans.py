# test_api.py

import pandas as pd
import os
import typing

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

    # Init database: cedict.csv
    current_dir = os.path.dirname(os.path.abspath(__file__))
    path = os.path.join(current_dir, "../../database/processed_data/cedict.csv")

    try:
      self.basic_db_cedict = pd.read_csv(path)
      self.logger.debug("loads csv successfully")
    except FileNotFoundError as e:
      self.logger.error(f"{e}\nDid you process raw data and save it in the"
                        f"right directory?")
    except Exception as e:
      self.logger.error(f"{e}")

  def trans(self, input: str) -> typing.Optional[dict]:
    if isinstance(self.basic_db_cedict, pd.DataFrame):
      resuld = self.basic_db_cedict[
        self.basic_db_cedict['simplified'] == input]

      output_dict = {
        "tranditional": resuld.iloc[0, 0],
        "simplified": resuld.iloc[0, 1],
        "pinyin": resuld.iloc[0, 2],
        "translation": resuld.iloc[0, 3]
        }

      self.logger.debug(f"Translation successful: \n{output_dict}")

      return output_dict
