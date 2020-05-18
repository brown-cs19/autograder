import os
import re


class ImportFixer:
    def __init__(self, target_path, stencil_dir):
        self.target_dir = os.path.dirname(target_path)
        self.target_path = target_path
        self.stencil_dir = stencil_dir
        self.rel_stencil_dir = os.path.relpath(self.stencil_dir,
                                               start=self.target_dir)
        with open(target_path, 'r') as f:
            self.content = f.read()

    def fix_import(self, name, location, filename=None):
        rel_loc = os.path.relpath(location, start=self.target_dir)
        self.content = re.sub(
            rf'my-gdrive\(["\'](.*-{name}\.arr)["\']\)',
            rf'file("{rel_loc}/{filename}")'
            if filename else rf'file("{rel_loc}/\1")', self.content)

    def finalize(self):
        self.content = re.sub(r'shared-gdrive\(["\'](.*?)["\'].*?\n?.*?\)',
                              rf'file("{self.rel_stencil_dir}/\1")',
                              self.content, re.M)
        with open(self.target_path, 'w') as f:
            f.write(self.content)
