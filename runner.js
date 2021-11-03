
requirejs(["q", "pyret-base/js/runtime", "pyret-base/js/post-load-hooks", "pyret-base/js/exn-stack-parser", "program"], function(Q, runtimeLib, loadHooksLib, stackLib, program) {

  var staticModules = program.staticModules;
  var depMap = program.depMap;
  var toLoad = program.toLoad;
  var uris = program.uris;
  var realm = { instantiated: {}, static: {}};
  var util = require('util');

  var main = toLoad[toLoad.length - 1];

  var runtime = runtimeLib.makeRuntime({
    stdout: function(s) { /*process.stdout.write(s);*/ },
    stderr: function(s) { /*process.stderr.write(s);*/ }
  });

  var EXIT_SUCCESS = 0;
  var EXIT_ERROR = 1;
  var EXIT_ERROR_RENDERING_ERROR = 2;
  var EXIT_ERROR_DISPLAYING_ERROR = 3;
  var EXIT_ERROR_CHECK_FAILURES = EXIT_SUCCESS;
  var EXIT_ERROR_JS = 5;
  var EXIT_ERROR_UNKNOWN = 6;

  runtime.setParam("command-line-arguments", process.argv.slice(1));

  var postLoadHooks = loadHooksLib.makeDefaultPostLoadHooks(runtime, {main: main, checkAll: true});
  postLoadHooks[main] = function(answer) {
    // var profile = runtime.getProfile();
    // if (profile.length > 0) {
    // profile.forEach(function(entry) { process.stderr.write(JSON.stringify(entry) + "\n"); });
    // }
    var checkerLib = runtime.modules["builtin://checker"];
    var checker = runtime.getField(runtime.getField(checkerLib, "provide-plus-types"), "values");
    var getStack = function(err) {

      err.val.pyretStack = stackLib.convertExceptionToPyretStackTrace(err.val, realm);

      var locArray = err.val.pyretStack.map(runtime.makeSrcloc);
      var locList = runtime.ffi.makeList(locArray);
      return locList;
    };
    var getStackP = runtime.makeFunction(getStack, "get-stack");
    // var toCall = runtime.getField(checker, "render-check-results-stack");
    var checks = runtime.getField(answer, "checks");

    // RETURNED FUNCTION MUST BE CALLED IN THE CONTEXT OF THE PYRET STACK
    function applyMethod(value, name, args) {
      return runtime.
        safeThen(function() {
          return runtime.getField(value, name);
        }, applyMethod).then(function(fun) {
          return fun.app.apply(value, args);
        })
    }

    // MUST NOT BE CALLED ON PYRET STACK
    function format(loc) {
      return applyMethod(loc, "format", [runtime.pyretTrue]);
    }

    var any = runtime.makeFunction(function(_){return runtime.pyretTrue;});
    var contents = runtime.ffi.toArray(checks);
    var result   = [];

    function render_TestResult(testresult) {
      function render_result(passed) {
        return function(result) {
          return format(result)
            .then(function(loc){return {loc: loc, passed: passed};});
        };
      }

      // We handle the `else` case as failure as we don't expect
      // future variants to be added which are considered "successful".
      // We need a different function because I don't really know how
      // `render_result` even extracts the `loc` field...
      // but it doesn't work for `else`.
      function render_else(passed) {
        return function(result) {
          return format(runtime.getField(result, "loc"))
            .then(function(loc){return {loc: loc, passed: passed};});
        };
      }
      return runtime.ffi.cases(any, "TestResult", testresult, {
         "success"                         : render_result(true),
         "failure-not-equal"               : render_result(false),
         "failure-not-different"           : render_result(false),
         "failure-not-satisfied"           : render_result(false),
         "failure-not-dissatisfied"        : render_result(false),
         "failure-wrong-exn"               : render_result(false),
         "failure-right-exn"               : render_result(false),
         "failure-exn"                     : render_result(false),
         "failure-no-exn"                  : render_result(false),
         "failure-raise-not-satisfied"     : render_result(false),
         "failure-raise-not-dissatisfied"  : render_result(false),
         "error-not-boolean"               : render_result(false),
         "failure-is-incomparable"         : render_result(false),
         "else"                            : render_else(false),
      });
    }

    function render_CheckBlockResult(checkblockresult) {
      return runtime.ffi.cases(any, "CheckBlockResult", checkblockresult, {
        "check-block-result": function(name,loc,keyword_check,test_results,maybe_err) {
          var results = runtime.ffi.toArray(test_results);
          var render  = [];
          return runtime.safeThen(function() {
              return runtime.eachLoop(runtime.makeFunction(function(i) {
                  return render_TestResult(results[i])
                    .then(function(rendered) {render.push(rendered);})
                    .start();
                }), 0, results.length);
            }, render_CheckBlockResult)
            .then(format(loc).start)
            .then(function(loc) {
              return { name : name,
                       loc  : loc,
                       error: runtime.ffi.isSome(maybe_err),
                       tests: render }; })
        }});
    }

    return runtime.safeCall(function() {
      return runtime.eachLoop(runtime.makeFunction(function(i) {
        return render_CheckBlockResult(contents[i])
          .then(function(rendered) { result.push(rendered); })
          .start();
      }), 0, contents.length);
    }, function(_) {
      process.stdout.write(JSON.stringify(result));
      process.stdout.write("\n")
      process.exit(EXIT_ERROR_CHECK_FAILURES);
    }, "check-block-comments: each: contents");
  }

  function renderErrorMessageAndExit(execRt, res) {
    if (execRt.isPyretException(res.exn)) {
      var rendererrorMod = execRt.modules["builtin://render-error-display"];
      var rendererror = execRt.getField(rendererrorMod, "provide-plus-types");
      var gf = execRt.getField;
      var exnStack = res.exn.stack;

      res.exn.pyretStack = stackLib.convertExceptionToPyretStackTrace(res.exn, realm);

      execRt.runThunk(
        function() {
          if (execRt.isObject(res.exn.exn) && execRt.hasField(res.exn.exn, "render-reason")) {
            return execRt.getColonField(res.exn.exn, "render-reason").full_meth(res.exn.exn);
          } else {
            return execRt.ffi.edEmbed(res.exn.exn);
          }
        },
        function(reasonResult) {
          if (execRt.isFailureResult(reasonResult)) {
            console.error("While trying to report that Pyret terminated with an error:\n" + JSON.stringify(res)
                          + "\nPyret encountered an error rendering that error:\n" + JSON.stringify(reasonResult)
                          + "\nStack:\n" + JSON.stringify(exnStack)
                          + "\nPyret stack:\n" + execRt.printPyretStack(res.exn.pyretStack, true) + "\n");
            process.exit(EXIT_ERROR_RENDERING_ERROR);
          } else {
            execRt.runThunk(
              function() {
                var cliRender = execRt.makeFunction(function(val) {
                  return execRt.toReprJS(val, execRt.ReprMethods["$cli"]);
                }, "cliRender");
                return gf(gf(rendererror, "values"), "display-to-string").app(
                  reasonResult.result,
                  cliRender,
                  execRt.ffi.makeList(res.exn.pyretStack.map(execRt.makeSrcloc)));
              },
              function(printResult) {
                if(execRt.isSuccessResult(printResult)) {
                  console.error(util.format(printResult.result));
                  console.error("\nPyret stack:\n" + execRt.printPyretStack(res.exn.pyretStack) + "\n");
                  process.exit(EXIT_ERROR);
                } else {
                  console.error(
                      "While trying to report that Pyret terminated with an error:\n" + JSON.stringify(res)
                      + "\ndisplaying that error produced another error:\n" + JSON.stringify(printResult)
                      + "\nStack:\n" + JSON.stringify(exnStack)
                      + "\nPyret stack:\n" + execRt.printPyretStack(res.exn.pyretStack, true) + "\n");
                  process.exit(EXIT_ERROR_DISPLAYING_ERROR);
                }
              }, "errordisplay->to-string");
          }
        }, "error->display");
    } else if (res.exn && res.exn.stack) {
      console.error("Abstraction breaking: Uncaught JavaScript error:\n" + util.format(res.exn));
      console.error("Stack trace:\n" + util.format(res.exn.stack) + "\n");
      process.exit(EXIT_ERROR_JS);
    } else {
      console.error("Unknown error result: " + util.format(res.exn) + "\n");
      process.exit(EXIT_ERROR_UNKNOWN);
    }
  }

  function isExit(execRt, result) {
    var exn = result.exn.exn;
    return execRt.ffi.isExit(exn) || execRt.ffi.isExitQuiet(exn);
  }

  function processExit(execRt, exn) {
    var exitCode = execRt.getField(exn, "code");
    if (execRt.ffi.isExit(exn)) {
      var message = "Exited with code " + exitCode.toString() + "\n";
      process.stdout.write(message);
    }
    process.exit(exitCode);
  }

  function onComplete(result) {
    if(runtime.isSuccessResult(result)) {
      //console.log("The program completed successfully");
      //console.log(result);
      process.exit(EXIT_SUCCESS);
    }
    else if (runtime.isFailureResult(result)) {
      if (runtime.isPyretException(result.exn) && isExit(runtime, result)) {
        processExit(runtime, result.exn.exn);
      }
      console.error("The run ended in error:");
      try {
        renderErrorMessageAndExit(runtime, result);
      } catch(e) {
        console.error("EXCEPTION!\n" + util.format(e) + "\n");
        process.exit(EXIT_ERROR_JS);
      }
    } else {
      console.error("The run ended in an unknown error: \n" + util.format(result) + "\n");
      console.error(result.exn.stack);
      process.exit(EXIT_ERROR_UNKNOWN);
    }
  }

  return runtime.runThunk(function() {
    runtime.modules = realm.instantiated;
    return runtime.runStandalone(staticModules, realm, depMap, toLoad, postLoadHooks);
  }, onComplete);
});
