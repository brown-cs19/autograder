#!/usr/bin/env python3

import os
from prehook_lib import ImportReplacer
IMPL = os.environ['IMPL']
TEST = os.environ['TEST']
COMMON = TEST.replace('-tests', '-common')

replacer = ImportReplacer('tests.arr', assignment_dir="../..")
replacer.replace_code_import(os.path.dirname(IMPL), os.path.basename(IMPL))
replacer.replace_common_import(os.path.dirname(COMMON))
replacer.finalize()
