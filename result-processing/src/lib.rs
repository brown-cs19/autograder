extern crate serde;
extern crate serde_json;
extern crate csv;

#[macro_use]
extern crate serde_derive;

extern crate itertools;

use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use itertools::Itertools;
use std::collections::{BTreeSet,BTreeMap};
use std::io;
use std::env;
use std::hash::Hash;

pub type Map<K,V> = BTreeMap<K,V>;
pub type Set<T>   = BTreeSet<T>;

pub trait AllOk<T, E, F> : Iterator
  where F: FnMut(Self::Item) -> Result<T, E>
{
  fn all_ok<A, B>(mut self, mut f: F) -> Result<A, B>
    where Self: Sized,
          A: Default + Extend<T>,
          B: Default + Extend<E>,
  {
    let stream = self.by_ref();

    let mut ok_items  = A::default();
    let mut err_items = B::default();

    loop {
        match stream.next() {
            Some(item) => {
                let item = f(item);
                match item {
                    Ok(ok_item) => {ok_items.extend(Some(ok_item));},
                    Err(err_item) => {
                        err_items.extend(Some(err_item));
                        err_items.extend(stream.map(f).filter_map(Result::err));
                        return Err(err_items);
                    }
                }
            },
            None => return Ok(ok_items)
        };
    }
  }
}

impl<I: ?Sized, T, E, F> AllOk<T, E, F> for I where
  I: Iterator,
  F: FnMut(Self::Item) -> Result<T, E> { }

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct Implementation(PathBuf);

impl std::ops::Deref for Implementation {
  type Target = PathBuf;
  fn deref(&self) -> &PathBuf {
    &self.0
  }
}

impl Implementation {
  pub fn is_instructor(&self) -> bool {
    self.to_string_lossy().contains("instructor")
  }
  pub fn is_wheat(&self) -> bool {
    self.to_string_lossy().contains("wheat")
  }
  pub fn is_chaff(&self) -> bool {
    self.to_string_lossy().contains("chaff")
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct TestSuite(PathBuf);

impl std::ops::Deref for TestSuite {
  type Target = PathBuf;
  fn deref(&self) -> &PathBuf {
    &self.0
  }
}

impl TestSuite {
  pub fn is_instructor(&self) -> bool {
    self.to_string_lossy().contains("instructor")
  }
  pub fn is_student(&self) -> bool {
    self.to_string_lossy().contains("student")
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct Test {
  pub loc            : String,
  pub passed         : bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct TestMetadata {
  pub loc            : String,
}

impl Test {
  pub fn metadata(&self) -> TestMetadata {
    TestMetadata {
      loc : self.loc.clone()
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestBlock {
  pub name           : String,
  pub loc            : String,
  pub error          : bool,
  pub tests          : Vec<Test>,
}

impl TestBlock {
  pub fn metadata(&self) -> TestBlockMetadata {
    TestBlockMetadata {
      name: self.name.clone(),
      loc : self.loc.clone()
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct TestBlockMetadata {
  pub name           : String,
  pub loc            : String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Error {
  Unknown,
  Compilation,
  OutOfMemory,
  Timeout,
  Runtime,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Evaluation {
  #[serde(rename = "impl")]
  pub implementation : Implementation,
  #[serde(rename = "tests")]
  pub test_suite     : TestSuite,
  pub result         : Result<Vec<TestBlock>, Error>,
}

impl Evaluation {
  pub fn summary(&self) -> Result<usize, Error> {
    self.result.as_ref().map(|blocks| {
      blocks.iter().map(|block| block.tests.iter().filter(|test| test.passed).count()).sum()
    }).map_err(|err| err.clone())
  }
}

pub fn read_evaluation_from_file<P: AsRef<Path>>(path: P) -> Vec<Evaluation> {
  let mut file = File::open(path).unwrap();
  let mut contents = String::new();
  file.read_to_string(&mut contents).unwrap();
  let u = serde_json::from_str(&contents[..]).unwrap();
  u
}

pub fn load_evaluation_from_args(args: env::Args) -> Vec<Evaluation> {
  let args: Vec<String> = env::args().collect();
  let path = &args[1];
  let results : Vec<Evaluation> = read_evaluation_from_file(path);
  return results;
}

pub trait Specification {
  fn test_suite_soundness(&self,
    results : &Map<TestSuite, Map<Implementation, bool>>)
      -> Map<TestSuite, bool>;

  fn implementation_soundness(&self,
    results : &Map<TestSuite, Map<Implementation, bool>>)
      -> Map<Implementation, Result<Set<TestSuite>,Set<TestSuite>>>;
}

impl Specification for Set<TestSuite> {
  // A test suite is sound if it is a reference test suite.
  fn test_suite_soundness(&self,
    results : &Map<TestSuite, Map<Implementation, bool>>)
      -> Map<TestSuite, bool>
  {
    results.keys()
      .map(|test_suite| (test_suite.clone(), self.contains(test_suite)))
      .collect()
  }

  // an implementation is sound if it passes all reference test suites
  fn implementation_soundness(&self,
    results : &Map<TestSuite, Map<Implementation, bool>>)
      -> Map<Implementation, Result<Set<TestSuite>, Set<TestSuite>>>
  {
    let mut soundness : Map<_, Result<_, Set<_>>> = Map::new();
    // for each reference test suite
    for reference_suite in self.iter() {
      // for each (impl, all_passed) result for that test suite
      for (implementation, &all_passed) in results[reference_suite].iter() {
        // an implementation is sound
        match (soundness.entry(implementation.clone()).or_insert(Ok(Set::new())), all_passed) {
          // if this suite's assessment agrees with other assessments,
          // we add this suite to the list test suites with that conclusion
          (&mut Ok(ref mut result), true) | (&mut Err(ref mut result), false)
            => { result.insert(reference_suite.clone()); },
          // if this suite finds the implementation faulty but others do not,
          // we throw away the previous asssessment of soundness and add this
          // test suite to the set of suites that finds this impl faulty.
          (result @ &mut Ok(_) , false) => {
            let mut unsound_set = Set::new();
            unsound_set.insert(reference_suite.clone());
            *result = Err(unsound_set);
          },
          // otherwise, we do nothing.
          _ => {}
        }
      }
    }
    soundness
  }
}

// When an Implementation is used as the specification,
impl Specification for Set<Implementation> {
  // A test suite is sound if all of its tests pass the instructor implementations
  fn test_suite_soundness(&self,
    results : &Map<TestSuite, Map<Implementation, bool>>)
      -> Map<TestSuite, bool>
  {
    // wtf.
    results.iter()
      .map(|(test_suite, results)|
        (test_suite.clone(), self.iter().all(|reference_impl| results[reference_impl])))
      .collect()
  }

  // An implementation is sound iff all tests in all sound test suites pass it.
  fn implementation_soundness(&self,
    results : &Map<TestSuite, Map<Implementation, bool>>)
      -> Map<Implementation, Result<Set<TestSuite>,Set<TestSuite>>>
  {
    let test_suite_soundness = self.test_suite_soundness(results);
    let mut implementation_soundness = Map::new();
    for implementation in results.values()
      .flat_map(|results| results.keys()).unique()
    {
      // implementation note: if there are no sound test suites,
      // the rhs will evaluate to true.
      implementation_soundness.insert(implementation.clone(),
        results.iter()
          .filter(|&(test_suite, _)| test_suite_soundness[test_suite])
          .all_ok(|(test_suite, results)|
            if *results.get(&implementation)
                .expect(&format!("{:?}\n{:?}", test_suite, implementation))
            {
              Ok(test_suite.clone())
            } else {
              Err(test_suite.clone())
            }));
    }
    implementation_soundness
  }
}

pub fn implies<T: Ord + Eq>(a: &Map<T, bool>, b: &Map<T, bool>) -> bool {
  a.iter().all(|(implementation, result)|
    b.get(implementation).unwrap_or(result) == result)
}

pub fn equivalent<T: Ord + Eq>(a: &Map<T, bool>, b: &Map<T, bool>) -> bool {
  implies(a, b) && implies(b, a)
}

/// Consumes results and a ground truth and produces a map from test suite to that suite's Performance
//pub fn summary<S: Specification>(
//  results : &Map<TestSuite, Map<Implementation, bool>>,
//  truth   : &S)
//    -> Map<TestSuite, Performance>
//{
//  let implementation_soundness = truth.implementation_soundness(results);
//  results.iter()
//    .map(|(test_suite, implementations)|
//      (test_suite.clone(),
//        summarize(implementations.iter()
//          .map(|(implementation, &all_passed)|
//            (implementation_soundness[implementation].is_ok(), all_passed)))))
//    .collect()
//}

pub fn results_map(results: Vec<Evaluation>) -> Map<TestSuite, Map<Implementation, bool>> {
  let mut results_map = Map::new();
  for Evaluation { implementation, test_suite, result } in results {
    results_map.entry(test_suite).or_insert_with(|| Map::new())
      .insert(implementation,result.as_ref().map(|blocks|
        blocks.iter().all(|block|
          !block.error && block.tests.iter().all(|test| test.passed)))
      // consider a timeout or compilation error to be a failure:
      .unwrap_or(false));
  }
  results_map
}

pub fn bucket<T: Hash + Eq + Ord>(results_map : &Map<T, Map<Implementation, bool>>) -> Set<Set<&T>> {
  let mut buckets = Map::new();
  for (a_test, a_results) in results_map.iter() {
    let mut bucket =
      buckets.entry(a_test)
        .or_insert_with(|| Set::new());
    bucket.insert(a_test.clone());
    for (b_test, b_results) in results_map.iter() {
      if a_test != b_test && equivalent(a_results, b_results) {
        bucket.insert(b_test.clone());
      }
    }
  }
  assert_eq!(results_map.len(), buckets.len());
  buckets.values().unique().cloned().collect()
}
