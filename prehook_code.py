import sys
import os
from prehook_lib import ImportReplacer

paths = sys.argv[1:]
for path in paths:
    print("Fixing imports for code file '{}'".format(path))
    replacer = ImportReplacer(path)
    replacer.replace_common_import(os.path.dirname(path))
    replacer.finalize()
