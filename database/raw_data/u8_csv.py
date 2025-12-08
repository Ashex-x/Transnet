# u8_csv.py

import csv
import re
import os


def convert(u8_file, csv_file):
  with open(u8_file, 'r', encoding='utf-8') as infile, \
    open(csv_file, 'w', encoding='utf-8', newline='') as outfile:

    outfile.write("traditional,simplified,pinyin,translation\n")
    writer = csv.writer(outfile)

    for line in infile:
      line = line.strip()

      # Skill empty line and annotation
      if not line or line.startswith('#'):
        continue

      # Extract chinese until the second space
      parts = line.split(' ', 2)
      if len(parts) < 3:
        continue

      Tw = parts[0]
      Cn = parts[1]
      rest = parts[2]

      # Extract pinyin
      pinyin = re.search(r'\[(.*?)\]', rest)
      if pinyin:
        pinyin = pinyin.group(1)
      else:
        pinyin = ''

      # Extract english
      definitions = re.findall(r'/(.*?)/', rest)
      translation = '/ '.join(definitions)

      # Write CSV
      writer.writerow([Tw, Cn, pinyin, translation])

  print("Finish.")


if __name__ == "__main__":
  # Check file path
  u8_file = 'cedict_ts.u8'
  csv_file = 'cedict.csv'

  if os.path.exists(csv_file):
    os.remove(csv_file)

  if not os.path.exists(u8_file):
    print(f"Error: {u8_file} not find.")
  else:
    convert(u8_file, csv_file)
