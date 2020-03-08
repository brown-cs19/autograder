#!/bin/bash

declare -a -x INSTRUCTOR_TESTS=(instructor/tests/*)
declare -a -x INSTRUCTOR_WHEAT=(instructor/impls/wheat/*)
declare -a -x INSTRUCTOR_CHAFF=(instructor/impls/chaff/*)
declare -a -x STUDENT_COMMON=(student/*/*-common.arr)
declare -a -x STUDENT_CODE=(student/*/*-code.arr)

declare -a -x TO_ASSESS=( "$@" )

UTILS_DIR=`realpath $(dirname "$0")`
ASSIGNMENT_DIR=`realpath $PWD`

function to_result_path(){
  IMPL="$(realpath --relative-to=. "$1")"
  TEST="$(realpath --relative-to=. "$2")"
  printf '%s~%s' ${IMPL////\;} ${TEST////\;}
}

function from_result_path(){
  RESULT="$(basename $1)"
  RESULT="${RESULT//;//}"
  echo -e "${RESULT//~/\\n}"
}

echo "Converting imports"

for IMPL in ${INSTRUCTOR_WHEAT[@]} ; do
    python3 $UTILS_DIR/prehook_instructor_impls.py $IMPL
done

for IMPL in ${INSTRUCTOR_CHAFF[@]} ; do
    python3 $UTILS_DIR/prehook_instructor_impls.py $IMPL
done

for COMMON in ${STUDENT_COMMON[@]} ; do
    python3 $UTILS_DIR/prehook_common.py $COMMON
done

for IMPL in ${STUDENT_CODE[@]} ; do
    python3 $UTILS_DIR/prehook_code.py $IMPL
done

time { shopt -s nullglob

  for IMPL in ${INSTRUCTOR_WHEAT[@]} ; do
    for TEST in ${INSTRUCTOR_TESTS[@]} ;  do
      echo "$(realpath "$IMPL")"  \
           "$(realpath "$TEST")"  \
           "$(realpath "result")/$(to_result_path "$IMPL" "$TEST")"
    done
  done

  for IMPL in ${INSTRUCTOR_CHAFF[@]} ; do
    for TEST in ${INSTRUCTOR_TESTS[@]} ;  do
      echo "$(realpath "$IMPL")"  \
           "$(realpath "$TEST")"  \
           "$(realpath "result")/$(to_result_path "$IMPL" "$TEST")"
    done
  done

  for STUDENT in ${TO_ASSESS[@]} ; do
    for IMPL in $STUDENT/*code.arr ; do
      for TEST in ${INSTRUCTOR_TESTS[@]};  do
        echo "$(realpath "$IMPL")"  \
             "$(realpath "$TEST")"  \
             "$(realpath "result")/$(to_result_path "$IMPL" "$TEST")"
      done
    done
  done

  for STUDENT in ${TO_ASSESS[@]} ; do
    for TEST in $STUDENT/*tests.arr ; do
      for IMPL in ${INSTRUCTOR_WHEAT[@]};  do
        echo "$(realpath "$IMPL")"  \
             "$(realpath "$TEST")"  \
             "$(realpath "result")/$(to_result_path "$IMPL" "$TEST")"
      done
      for IMPL in ${INSTRUCTOR_CHAFF[@]};  do
        echo "$(realpath "$IMPL")"  \
             "$(realpath "$TEST")"  \
             "$(realpath "result")/$(to_result_path "$IMPL" "$TEST")"
      done
    done
  done ; } | UTILS_DIR=$UTILS_DIR $UTILS_DIR/powder-monkey/evaluate-many.sh "$UTILS_DIR/prehook.sh"

find ./result/ -size +0 -name 'error.txt' -print0 \
  | xargs -r0 realpath --relative-to=. \
  | tee problems

echo "[POSTFLIGHT] Coalescing Results"
$UTILS_DIR/jq --slurp -c '.' result/*/results.json \
  > results.json

cd $UTILS_DIR/result-processing
cargo run --bin test-eval --release -- $ASSIGNMENT_DIR/results.json > $ASSIGNMENT_DIR/results.csv
