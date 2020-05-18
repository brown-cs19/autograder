import os
import re


class ImportReplacer:
    def __init__(self, path, assignment_dir=os.curdir):
        self.dir = os.path.dirname(path)
        self.path = path
        self.assignment_dir = assignment_dir
        with open(path, 'r') as f:
            self.content = f.read()

    def replace_code_import(self, code_dir, code_file=None):
        code_dir = os.path.relpath(code_dir, start=self.dir)
        self.content = re.sub(
            r'my-gdrive\(["\'](.*-code\.arr)["\']\)',
            r'file("{}/{}")'.format(code_dir, code_file)
            if code_file else r'file("{}/\1")'.format(code_dir), self.content)

    def replace_common_import(self, common_dir):
        self.content = re.sub(
            r'my-gdrive\(["\'](.*-common\.arr)["\']\)',
            r'file("{}/\1")'.format(os.path.relpath(common_dir,
                                                    start=self.dir)),
            self.content)

    def finalize(self):
        self.content = re.sub(
            r'shared-gdrive\(["\'](.*?)["\'].*?\n?.*?\)',
            r'file("{}/\1")'.format(
                os.path.relpath(self.assignment_dir,
                                start=self.dir)), self.content, re.M)
        with open(self.path, 'w') as f:
            f.write(self.content)
