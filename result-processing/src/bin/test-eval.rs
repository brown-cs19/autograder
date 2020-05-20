extern crate cs173_autograder_postprocessing;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::{env, fs::File, io::BufWriter};

use cs173_autograder_postprocessing::*;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
struct FailureReasons {
    erroring_blocks: Set<TestBlockMetadata>,
    failing_tests: Set<TestMetadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
struct TestSuiteEvaluation {
    wheats_accepted: Map<Implementation, bool>, // true if accepted
    chaffs_rejected: Map<Implementation, bool>, // true if rejected
    wheat_failure_reasons: Option<FailureReasons>,
}

impl Default for TestSuiteEvaluation {
    fn default() -> TestSuiteEvaluation {
        TestSuiteEvaluation {
            wheats_accepted: Map::default(), // true if accepted
            chaffs_rejected: Map::default(), // true if rejected
            wheat_failure_reasons: None,
        }
    }
}

fn summarize(results: Map<Implementation, Result<Vec<TestBlock>, Error>>) -> TestSuiteEvaluation {
    let wheats: Map<&Implementation, &Result<Vec<TestBlock>, Error>> =
        results.iter().filter(|(i, r)| i.is_wheat()).collect();

    let invalid_blocks: Set<TestBlockMetadata> = wheats
        .values()
        .cloned()
        .flatten()
        .flat_map(|blocks| blocks.iter())
        .filter(|block| block.error)
        .map(TestBlock::metadata)
        .collect();

    let invalid_tests: Set<TestMetadata> = wheats
        .values()
        .cloned()
        .flatten()
        .flat_map(|blocks| blocks.iter())
        .flat_map(|block| block.tests.iter())
        .filter(|test| !test.passed)
        .map(Test::metadata)
        .collect();

    let mut chaffs: Map<Implementation, Result<Vec<TestBlock>, Error>> = results
        .iter()
        .filter(|(i, r)| i.is_chaff())
        .map(|(i, r)| (i.clone(), r.clone()))
        .collect();

    for result in chaffs.values_mut() {
        if let Ok(blocks) = result.as_mut() {
            blocks.retain(|block| {
                invalid_blocks
                    .iter()
                    .map(|block| block.loc.rsplit('/').next().unwrap())
                    .find(|&loc| loc == block.loc.rsplit('/').next().unwrap())
                    .is_none()
            });
            for block in blocks.iter_mut() {
                block.tests.retain(|test| {
                    invalid_tests
                        .iter()
                        .map(|test| test.loc.rsplit('/').next().unwrap())
                        .find(|&loc| loc == test.loc.rsplit('/').next().unwrap())
                        .is_none()
                });
            }
        }
    }

    TestSuiteEvaluation {
        wheats_accepted: wheats
            .iter()
            .map(|(&i, r)| {
                (
                    i.clone(),
                    r.as_ref()
                        .map(|blocks| {
                            blocks.iter().all(|block| {
                                !block.error && block.tests.iter().all(|test| test.passed)
                            })
                        })
                        .map_err(|err| err.clone())
                        .unwrap_or(false),
                )
            })
            .collect(),
        chaffs_rejected: chaffs
            .iter()
            .map(|(i, r)| {
                (
                    i.clone(),
                    r.as_ref()
                        .map(|blocks| {
                            blocks.iter().any(|block| {
                                block.error || block.tests.iter().any(|test| !test.passed)
                            })
                        })
                        .map_err(|err| err.clone())
                        .unwrap_or(true),
                )
            })
            .collect(),
        wheat_failure_reasons: if !invalid_blocks.is_empty() || !invalid_tests.is_empty() {
            Some(FailureReasons {
                erroring_blocks: invalid_blocks,
                failing_tests: invalid_tests,
            })
        } else {
            None
        },
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Feature {
    test_suite: TestSuite,
    result: Result<(usize, usize), Error>,
}

#[derive(Clone, Debug, Serialize)]
struct GradescopeReport {
    visibility: String,
    stdout_visibility: String,
    tests: Vec<GradescopeTestReport>,
}

impl GradescopeReport {
    fn new(tests: Vec<GradescopeTestReport>) -> Self {
        Self {
            visibility: "after_published".to_owned(),
            stdout_visibility: "after_published".to_owned(),
            tests,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct GradescopeTestReport {
    name: String,
    score: usize,
    max_score: usize,
    output: String,
    visibility: String,
}

impl GradescopeTestReport {
    fn new(name: String, score: usize, max_score: usize, output: String) -> Self {
        Self {
            name,
            score,
            max_score,
            output,
            visibility: "after_published".to_owned(),
        }
    }

    fn new_visible(name: String, score: usize, max_score: usize, output: String) -> Self {
        Self {
            name,
            score,
            max_score,
            output,
            visibility: "visible".to_owned(),
        }
    }
}

fn main() {
    match env::args().collect::<Vec<_>>().as_slice() {
        [_, infile, outfile] => {
            let results: Vec<Evaluation> = read_evaluation_from_file(infile);

            let (wheat_chaff_results, test_results): (Vec<&Evaluation>, Vec<&Evaluation>) =
                results.iter().partition(|evaluation| {
                    evaluation.implementation.is_wheat() || evaluation.implementation.is_chaff()
                });

            let test_suite_evaluation = summarize(
                wheat_chaff_results
                    .into_iter()
                    .map(|evaluation| {
                        (evaluation.implementation.clone(), evaluation.result.clone())
                    })
                    .collect(),
            );
            let wheat_test_reports =
                test_suite_evaluation
                    .wheats_accepted
                    .into_iter()
                    .map(|(wheat, passed)| {
                        GradescopeTestReport::new(
                            wheat.file_name().unwrap().to_string_lossy().to_string(),
                            if passed { 1 } else { 0 },
                            1,
                            "Passed wheat".to_owned(),
                        )
                    });
            let chaff_test_reports =
                test_suite_evaluation
                    .chaffs_rejected
                    .into_iter()
                    .map(|(chaff, caught)| {
                        GradescopeTestReport::new(
                            chaff.file_name().unwrap().to_string_lossy().to_string(),
                            if caught { 1 } else { 0 },
                            1,
                            "Caught chaff".to_owned(),
                        )
                    });

            let functionality_reports = test_results.into_iter().flat_map(|result| {
                match (&result.test_suite, result.summary()) {
                    (_, Ok(block_results)) => block_results
                        .into_iter()
                        .map(|(block_name, passed, total)| {
                            GradescopeTestReport::new(
                                block_name,
                                passed,
                                total,
                                "Tests passed!".to_owned(),
                            )
                        })
                        .collect::<Vec<_>>()
                        .into_iter(),
                    (suite, Err(e)) => vec![GradescopeTestReport::new_visible(
                        suite.file_name().unwrap().to_string_lossy().to_string(),
                        0,
                        1,
                        format!("Error: {:?}", e),
                    )]
                    .into_iter(),
                }
            });

            let gradescope_report = GradescopeReport::new(
                wheat_test_reports
                    .chain(chaff_test_reports)
                    .chain(functionality_reports)
                    .collect(),
            );
            let out = File::create(outfile).unwrap();
            serde_json::to_writer(BufWriter::new(out), &gradescope_report).unwrap();
            println!("wrote output to {}", outfile);
        }
        _ => eprintln!("Usage: <infile> <outfile>"),
    }
}
