import os
from os.path import basename, dirname
import shutil
import subprocess
import tempfile
import json
from multiprocessing import Pool
from prehook_lib import ImportFixer

NODE_PATH = "node"
JQ = "jq"
AUTOGRADER = "/autograder"
PYRET_PATH = f"{AUTOGRADER}/pyret-lang"
NODE_MODULES_PATH = f"{PYRET_PATH}/node_modules"
RUNNER_PATH = f"{AUTOGRADER}/source/runner.js"
RESULTS = f"{AUTOGRADER}/results"
SUBMISSION = f"{AUTOGRADER}/submission"
STENCIL = f"{AUTOGRADER}/source/stencil"
INSTRUCTOR = f"{AUTOGRADER}/source/instructor"


def fix_imports(path, code_path, common_dir):
    fixer = ImportFixer(path, STENCIL)
    fixer.fix_import("code", dirname(code_path), basename(code_path))
    fixer.fix_import("common", common_dir)
    fixer.finalize()


def nonempty(path):
    return os.path.exists(path) and os.path.getsize(path)


def run(job):
    """ Run a single job"""

    code_path, test_path, common_dir = job
    cache_dir = tempfile.mkdtemp(dir="/tmp")

    # Make a directory for the job
    job_name = f"{basename(code_path)};{basename(test_path)}"
    job_path = f"{RESULTS}/{job_name}"
    os.mkdir(job_path)

    # Copy tests into the job directory
    copied_test_path = f"{job_path}/tests.arr"
    shutil.copy(test_path, copied_test_path)
    test_path = copied_test_path

    # Fix test imports for this job
    fix_imports(test_path, code_path, common_dir)

    error_output = f"{job_path}/error.txt"
    with open(error_output, "a") as error:
        # Compile test file
        os.chdir(PYRET_PATH)
        compiled_tests_path = f"{os.path.relpath(job_path)}/tests.js"
        args = [
            NODE_PATH,
            "build/phaseA/pyret.jarr",
            "-no-display-progress",
            "--build-runnable",
            os.path.relpath(test_path),
            "--outfile",
            compiled_tests_path,
            "--standalone-file",
            RUNNER_PATH,
            "--builtin-js-dir",
            "src/js/trove/",
            "--builtin-arr-dir",
            "src/arr/trove",
            "--compiled-dir",
            cache_dir,
            "--require-config",
            "src/scripts/standalone-configA.json",
        ]
        env = {"NODE_PATH": NODE_MODULES_PATH}
        subprocess.run(args, check=True, stderr=error, env=env)

        def report_error(error):
            with open(f"{job_path}/results.json", "a") as output:
                error = {
                    "code": code_path,
                    "tests": test_path,
                    "result": {
                        "Err": error
                    }
                }
                output.write(json.dumps(error))

        # Check for compile error
        if not nonempty(compiled_tests_path):
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
        with open(f"{job_path}/results.json", "a") as output:
            with open(error_output, "a") as error:
                subprocess.run(args, check=True, stdout=output, stderr=error)

    if not nonempty(error_output):
        os.remove(error_output)
        os.remove(compiled_tests_path)


if __name__ == '__main__':
    os.chdir(AUTOGRADER)
    if os.path.exists(RESULTS):
        shutil.rmtree(RESULTS)
    os.mkdir(RESULTS)

    student_common_path = ""
    student_code_path = ""
    student_test_path = ""
    for root, _, files in os.walk(SUBMISSION):
        for f in files:
            if "common" in f:
                student_common_path = os.path.join(root, f)
            if "code" in f:
                student_code_path = os.path.join(root, f)
            if "tests" in f:
                student_test_path = os.path.join(root, f)
    assert student_common_path and student_code_path and student_test_path
    student_common_dir = dirname(student_common_path)

    os.chdir("source")

    jobs = []

    # Fix import statements in student's common file
    fix_imports(student_common_path, student_code_path, SUBMISSION)

    # Fix import statements in student's code file
    fix_imports(student_code_path, student_code_path, SUBMISSION)

    # Run tests against student code
    for root, _, files in os.walk(f"{INSTRUCTOR}/tests"):
        for f in files:
            if f != "README":
                test = os.path.join(root, f)
                jobs.append((student_code_path, test, student_common_dir))

    # Run wheats against student tests
    for root, _, files in os.walk(f"{INSTRUCTOR}/impls/wheat"):
        for f in files:
            if f != "README":
                wheat = os.path.join(root, f)
                fix_imports(wheat, wheat, dirname(wheat))
                jobs.append((wheat, student_test_path, student_common_dir))

    # Run chaffs against student tests
    for root, _, files in os.walk(f"{INSTRUCTOR}/impls/chaff"):
        for f in files:
            if f != "README":
                chaff = os.path.join(root, f)
                fix_imports(chaff, chaff, dirname(chaff))
                jobs.append((chaff, student_test_path, student_common_dir))

    # Run all jobs
    with Pool() as pool:
        pool.map(run, jobs)
