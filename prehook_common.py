import sys
from prehook_lib import ImportReplacer

paths = sys.argv[1:]
for path in paths:
    print("Fixing imports for common file '{}'".format(path))
    replacer = ImportReplacer(path)
    replacer.finalize()
