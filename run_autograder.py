import sys
import os
from os.path import basename, dirname
import shutil
import subprocess
import json
from prehook_lib import ImportFixer

NODE_PATH = "nodejs"
JQ = "jq"
AUTOGRADER = "/autograder"
SOURCE = f"{AUTOGRADER}/source/autograder"
PYRET_PATH = f"{AUTOGRADER}/pyret-lang"
NODE_MODULES_PATH = f"{PYRET_PATH}/node_modules"
RUNNER_PATH = f"{SOURCE}/runner.js"
RESULTS = f"{AUTOGRADER}/results"
SUBMISSION = f"{AUTOGRADER}/submission"
CACHE_DIR = f"{SOURCE}/cache"
STENCIL = f"{SOURCE}/stencil"
INSTRUCTOR = f"{SOURCE}/instructor"
WHEATS = f"{INSTRUCTOR}/impls/wheat"
CHAFFS = f"{INSTRUCTOR}/impls/chaff"
TESTS = f"{INSTRUCTOR}/tests"


class CompileError(Exception):
    pass


def fix_imports(path, code_path, common_dir):
    fixer = ImportFixer(path, STENCIL)
    fixer.fix_import("code", dirname(code_path), basename(code_path))
    fixer.fix_import("common", common_dir)
    fixer.finalize()


def nonempty(path):
    return os.path.exists(path) and os.path.getsize(path)


def compile_tests(test_path, error_file):
    os.chdir(PYRET_PATH)
    rel_test_path = os.path.relpath(test_path)
    compiled_tests_path = f"{dirname(rel_test_path)}/tests.js"
    args = [
        NODE_PATH,
        "--max-old-space-size=4096",
        "build/phaseA/pyret.jarr",
        "-no-display-progress",
        "--build-runnable",
        rel_test_path,
        "--outfile",
        compiled_tests_path,
        "--standalone-file",
        RUNNER_PATH,
        "--builtin-js-dir",
        "src/js/trove/",
        "--builtin-arr-dir",
        "src/arr/trove",
        "--compiled-dir",
        CACHE_DIR,
        "--require-config",
        "src/scripts/standalone-configA.json",
    ]
    env = {"NODE_PATH": NODE_MODULES_PATH}
    try:
        subprocess.run(args, check=True, stderr=error_file, env=env)
    except Exception as e:
        raise CompileError(e)

    # Check for compile error
    if not nonempty(compiled_tests_path):
        raise CompileError("Compile error")

    return compiled_tests_path


def run(code_path, test_path, common_dir):
    # Make sure cache dir exists
    if not os.path.exists(CACHE_DIR):
        os.mkdir(CACHE_DIR)

    # Make a directory for the job
    job_name = f"{basename(code_path)};{basename(test_path)}"
    job_path = f"{RESULTS}/{job_name}"
    os.mkdir(job_path)
    
    # Prints out currently running job name
    print(job_name)

    # Copy tests into the job directory
    copied_test_path = f"{job_path}/tests.arr"
    shutil.copy(test_path, copied_test_path)
    test_path = copied_test_path

    def report_error(error):
        with open(f"{job_path}/results.json", "w") as output:
            error = {
                "code": code_path,
                "tests": test_path,
                "result": {
                    "Err": error
                }
            }
            output.write(json.dumps(error))

    # Fix test imports for this job
    fix_imports(test_path, code_path, common_dir)

    error_output = f"{job_path}/error.txt"
    with open(error_output, "a") as error:
        # Compile test file
        try:
            compiled_tests_path = compile_tests(test_path, error)
        except CompileError:
            print(f"Compilation failed: {code_path} {test_path}")
            report_error("Compilation")
            return

        # Assume a timeout occurs
        report_error("Timeout")

        # Run tests on code
        output_path = f"{job_path}/raw.json"
        with open(output_path, "w") as output:
            args = [NODE_PATH, compiled_tests_path]
            env = {"NODE_PATH": NODE_MODULES_PATH}
            subprocess.run(args,
                           check=True,
                           stdout=output,
                           stderr=error,
                           env=env)

    if nonempty(error_output):
        with open(error_output, "r") as error:
            if "memory" in error.read():
                report_error("OutOfMemory")
            else:
                report_error("Runtime")

    if nonempty(output_path):
        # Write out results
        args = [
            JQ, "--compact-output", "--arg", "code", code_path, "--arg",
            "test", test_path,
            '{ code: $code, tests: $test, result: {Ok: (. |= map(select(.loc | contains("tests.arr"))))} }',
            output_path
        ]
        with open(f"{job_path}/results.json", "w") as output:
            with open(error_output, "a") as error:
                subprocess.run(args, check=True, stdout=output, stderr=error)

    if not nonempty(error_output):
        os.remove(error_output)
        os.remove(compiled_tests_path)


if __name__ == '__main__':
    if len(sys.argv) == 1:
        config_file = None
    elif len(sys.argv) == 2:
        config_file = sys.argv[1]
    else:
        print("Usage: python3 run_autograder.py [config_file.json]", file=sys.stderr)
        sys.exit(1)

    if config_file is not None:
        with open(config_file, "r") as f:
            data = json.loads(f.read())
        
        error = "Config file should take the form {useWheats: Boolean, chaffs: List<String>}"
        assert isinstance(data, dict), error

        assert "useWheats" in data, error
        use_wheats = data["useWheats"]
        assert isinstance(use_wheats, bool), error

        assert "chaffs" in data, error
        chaffs = data["chaffs"]
        assert isinstance(chaffs, list), error
        assert all(map(lambda item: isinstance(item, str), chaffs)), error
    else:
        use_wheats = True
        chaffs = None

    os.chdir(AUTOGRADER)
    if os.path.exists(RESULTS):
        shutil.rmtree(RESULTS)
    os.mkdir(RESULTS)

    student_code_path = ""
    for root, _, files in os.walk(SUBMISSION):
        assert len(files) == 1
        for f in files:
            student_code_path = os.path.join(root, f)
    assert student_code_path
    student_code_dir = dirname(student_code_path)

    os.chdir(SOURCE)  # FIXME: is this needed?

    # Fix import statements in student's code file
    fix_imports(student_code_path, student_code_path, SUBMISSION)

    # Updates imports for stencils
    for root, _, files in os.walk(STENCIL):
        for f in files:
            if f != "README":
                stencil = os.path.join(root, f)
                fix_imports(stencil, stencil, dirname(stencil))

    # Run tests against student code
    for root, _, files in os.walk(TESTS):
        for f in files:
            if f != "README":
                test = os.path.join(root, f)
                run(student_code_path, test, student_code_dir)

    # Run wheats against student tests
    if use_wheats:
        for root, _, files in os.walk(WHEATS):
            for f in files:
                if f != "README":
                    wheat = os.path.join(root, f)
                    fix_imports(wheat, wheat, student_code_dir)
                    run(wheat, student_test_path, student_code_dir)

    # Run chaffs against student tests
    for root, _, files in os.walk(CHAFFS):
        for f in files:
            if f != "README" and (chaffs is None or os.path.basename(f) in chaffs):
                chaff = os.path.join(root, f)
                fix_imports(chaff, chaff, student_code_dir)
                run(chaff, student_test_path, student_code_dir)
