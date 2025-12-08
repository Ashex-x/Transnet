# runme_test.py

import backend.logger_setup
import backend.utils_test


if __name__ == "__main__":
  logger = backend.logger_setup.setup_logger()
  backend.utils_test.main()
