#!/bin/bash

#:>stdout.log
#:>stderr.log

# 1 or $PREHOOK     (optional) script to run after copying each $TEST to $OUTPUT

job_name="pw${RANDOM}"

PREHOOK="$(realpath "${1:-$PREHOOK}" || echo "")"
PYRET="$(realpath "$(dirname "${BASH_SOURCE[0]}")/pyret-lang")"
RUNNER="$(dirname "${BASH_SOURCE[0]}")/runner.js"
#RUNNER="/gpfs/main/home/jswrenn/projects/powder-monkey/evaluate/pyret-lang/src/js/base/handalone.js"
# Delete queued jobs if script exits
function qkill(){ qdel -u $USER ; }
trap "qkill" EXIT ERR

function queue-test(){
  IMPL="$1"
  TEST="$2"
  OUTPUT="$3"

  qsub -terse                   \
       -cwd                     \
       -notify                  \
       -l h_rt=00:10:00         \
       -o '/dev/null'           \
       -e '/dev/null'           \
       -N "$job_name"           \
       -v IMPL="$IMPL"          \
       -v TEST="$TEST"          \
       -v OUTPUT="$OUTPUT"      \
       -v PYRET="$PYRET"        \
       -v RUNNER="$RUNNER"      \
       -v PREHOOK="$PREHOOK"    \
       -v UTILS_DIR="$UTILS_DIR"\
       "$(dirname "${BASH_SOURCE[0]}")/evaluate.sh" >/dev/null
}

function jobs-remaining(){
  qstat | tail -n+4 | wc -l
}

function overwrite() { echo -e "\r\033[1A\033[0K$@"; }

function join(){
  stage="$1"
  job_list="$2"
  qsub -cwd                  \
       -sync y               \
       -l test               \
       -hold_jid "$job_list" \
       -o '/dev/null'        \
       -e '/dev/null'        \
       -N "join"             \
       "$(dirname "${BASH_SOURCE[0]}")/join.sh" >stdout.txt &
  # Show progress indicator
  pid=$! ; trap "kill $pid 2> /dev/null; qkill" EXIT ERR ; sleep 1;
  while kill -0 $pid 2> /dev/null; do
    overwrite "[$stage] $(jobs-remaining) jobs remaining"
  done; trap 'qkill' EXIT ERR
}

job_counter=0;
while read -r IMPL TEST OUTPUT ; do
    let job_counter++ ;
    queue-test "$IMPL" "$TEST" "$OUTPUT"
    overwrite "[EVALUATING] Queueing job $job_counter"
done; join "EVALUATING" "$job_name"

