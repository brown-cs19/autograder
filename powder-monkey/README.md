This folder contains a basic set of utilities for orchestrating massive automated evaluations of Pyret programs.

The two utilities course staff will find helpful are [evaluate.sh](./evaluate.sh), which evaluates a single test suite against an single implementation, and [evaluate-many.sh](./evaluate-many.sh) which schedules evaluations on [a GridEngine cluster](https://cs.brown.edu/about/system/services/hpc/gridengine/).

The [evaluate-many.sh](./evaluate-many.sh) consumes its joblist from stdin. Each line of input should be a space-delmited (1) path to an implementation, (2) path to a test suite, and (3) path to a folder to store its output. For a complete example of invoking [evaluate-many.sh](./evaluate-many.sh), see [evaluate-example.sh](../example/evaluate-example.sh).

Be sure to modify [evaluate.sh](./evaluate.sh#L4) such that `PATH` includes an appropriate version of node.
