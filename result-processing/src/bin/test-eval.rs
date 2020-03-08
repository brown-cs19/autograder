extern crate cs173_autograder_postprocessing;

extern crate serde;
extern crate serde_json;
extern crate csv;

#[macro_use]
extern crate serde_derive;

extern crate itertools;

use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::collections::{BTreeSet,BTreeMap};
use std::{io,env,fmt};

use cs173_autograder_postprocessing::*;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct FailureReasons {
  erroring_blocks: Set<TestBlockMetadata>,
  failing_tests: Set<TestMetadata>
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
struct TestSuiteEvaluation {
  wheats_accepted: Map<Implementation, bool>, // true if accepted
  chaffs_rejected: Map<Implementation, bool>, // true if rejected
  wheat_failure_reasons: Option<FailureReasons>
}

impl Default for TestSuiteEvaluation {
  fn default() -> TestSuiteEvaluation {
    TestSuiteEvaluation {
      wheats_accepted: Map::default(), // true if accepted
      chaffs_rejected: Map::default(), // true if rejected
      wheat_failure_reasons: None
    }
  }
}

impl TestSuiteEvaluation {
  fn to_results(&self) -> Vec<String> {
    let wheat_format: Vec<String> =
        self.wheats_accepted.iter()
          .map(|(wheat, &accepted)|
              if accepted { String::from("TRUE")} else { String::from("FALSE") }
	  ).collect();

    let chaff_format: Vec<String> =
        self.chaffs_rejected.iter()
          .map(|(chaff, &rejected)|
              if rejected { String::from("TRUE")} else { String::from("FALSE") }
	  ).collect();

    [wheat_format, chaff_format].concat()
  }
}

fn summarize(results: Map<Implementation, Result<Vec<TestBlock>, Error>>) -> TestSuiteEvaluation {
  let wheats : Map<&Implementation, &Result<Vec<TestBlock>, Error>> =
    results.iter().filter(|(i, r)| i.is_wheat()).collect();

  // true if any wheat simply could not be executed b/c of an error
  let invalid_suite = wheats.values().cloned().any(Result::is_err);

  let invalid_blocks : Set<TestBlockMetadata> =
    wheats.values().cloned().flatten()
      .flat_map(|blocks| blocks.iter())
      .filter(|block| block.error)
      .map(TestBlock::metadata)
      .collect();

  let invalid_tests : Set<TestMetadata> =
    wheats.values().cloned().flatten()
      .flat_map(|blocks| blocks.iter())
      .flat_map(|block| block.tests.iter())
      .filter(|test| !test.passed)
      .map(Test::metadata)
      .collect();

  let mut chaffs : Map<Implementation, Result<Vec<TestBlock>, Error>> =
    results.iter().filter(|(i, r)| i.is_chaff())
                  .map(|(i, r)| (i.clone(), r.clone()))
                  .collect();

  for result in chaffs.values_mut() {
    if let Ok(blocks) = result.as_mut() {
      blocks.retain(|block|
        invalid_blocks.iter()
          .map(|block| block.loc.rsplit('/').next().unwrap())
          .find(|&loc| loc == block.loc.rsplit('/').next().unwrap())
          .is_none());
      for block in blocks.iter_mut() {
          block.tests.retain(|test|
            invalid_tests.iter()
              .map(|test| test.loc.rsplit('/').next().unwrap())
              .find(|&loc| loc == test.loc.rsplit('/').next().unwrap())
              .is_none());
      }
    }
  }

  TestSuiteEvaluation {
    wheats_accepted:
      wheats.iter().map(|(&i, r)|
        (i.clone(),
          r.as_ref().map(|blocks|
            blocks.iter().all(|block|
              !block.error && block.tests.iter().all(|test|
                test.passed)))
            .map_err(|err| err.clone()).unwrap_or(false))).collect(),
    chaffs_rejected:
      chaffs.iter().map(|(i, r)|
        (i.clone(),
          r.as_ref().map(|blocks|
            blocks.iter().any(|block|
              block.error || block.tests.iter().any(|test|
                !test.passed)))
            .map_err(|err| err.clone()).unwrap_or(true))).collect(),
    wheat_failure_reasons:
      if !invalid_blocks.is_empty() || !invalid_tests.is_empty() {
        Some(FailureReasons {
          erroring_blocks: invalid_blocks,
          failing_tests: invalid_tests
        })
      } else {
        None
      }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Feature {
  pub test_suite     : TestSuite,
  pub result         : Result<usize, Error>,
}

fn impl_eval_to_iter(features : Vec<Feature>) -> Vec<String> {
    features.iter().map(|feature| {
	match feature.result {
            Ok(passed) => format!("{}", passed),
            Err(ref err) => format!("ERROR ({:?})", err),
          }
    }).collect()
}


fn main() {
  let results : Vec<Evaluation> = load_evaluation_from_args(env::args());

  let test_suites_by_student: Map<TestSuite, Map<Implementation, Result<Vec<TestBlock>, Error>>> =
    results.iter().filter(|evaluation| evaluation.test_suite.is_student())
      .fold(Map::default(), |mut grouped, evaluation|
        {
          grouped.entry(evaluation.test_suite.clone()).or_insert(Map::default())
            .insert(evaluation.implementation.clone(),evaluation.result.clone());
          grouped
        });

  let test_suites_by_student: Map<String, TestSuiteEvaluation> =
    test_suites_by_student.iter()
      .map(|(student, results)|
        (student.components().rev().nth(1).unwrap().as_os_str().to_string_lossy().into_owned(),
          summarize(results.clone())))
      .collect();

  let mut implementations_by_student : Map<Implementation, Vec<Evaluation>> = Map::new();
  for evaluation in results {
    if evaluation.implementation.to_string_lossy().contains("instructor") {
      continue;
    }
    implementations_by_student.entry(evaluation.implementation.clone())
      .or_insert(Vec::new())
      .push(evaluation);
  }

  let mut processed : Map<String, Vec<Feature>> = Map::new();

  for (implementation, evaluations) in implementations_by_student.iter() {
    for evaluation in evaluations {
      processed.entry(evaluation.implementation.components().rev().nth(1).unwrap().as_os_str().to_string_lossy().into_owned())
        .or_insert(Vec::new())
        .push(Feature {
          test_suite: evaluation.test_suite.clone(),
          result: evaluation.summary()
        });
    }
  }

  let students = test_suites_by_student.keys().chain(processed.keys()).unique();
  let mut wtr = csv::Writer::from_writer(io::stdout());

  use itertools::Itertools;

  let empty_eval = TestSuiteEvaluation::default();

  for student in students {
    let test_suite_evaluation = test_suites_by_student.get(student)
      .unwrap_or(&empty_eval);
    let implementation_evaluation = processed.get(student);
    if let Some(implementation_evaluation) = implementation_evaluation {
        let res = wtr.write_record(&[vec![student.to_owned()],
          test_suite_evaluation.to_results(),
          impl_eval_to_iter(implementation_evaluation.clone()),
        ].concat());
	if res.is_err() {
	    println!("Skipping {}: wrong number of entries", student);
	}

        wtr.flush().unwrap();
    } else {
	println!("Skipping {}: couldn't find evaluation", student);
    }
  }
}
