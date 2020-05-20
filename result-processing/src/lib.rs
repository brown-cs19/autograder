extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::hash::Hash;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

pub type Map<K, V> = BTreeMap<K, V>;
pub type Set<T> = BTreeSet<T>;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct Implementation(PathBuf);

impl std::ops::Deref for Implementation {
    type Target = PathBuf;
    fn deref(&self) -> &PathBuf {
        &self.0
    }
}

impl Implementation {
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

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct Test {
    pub loc: String,
    pub passed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct TestMetadata {
    pub loc: String,
}

impl Test {
    pub fn metadata(&self) -> TestMetadata {
        TestMetadata {
            loc: self.loc.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestBlock {
    pub name: String,
    pub loc: String,
    pub error: bool,
    pub tests: Vec<Test>,
}

impl TestBlock {
    pub fn metadata(&self) -> TestBlockMetadata {
        TestBlockMetadata {
            name: self.name.clone(),
            loc: self.loc.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct TestBlockMetadata {
    pub name: String,
    pub loc: String,
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
    #[serde(rename = "code")]
    pub implementation: Implementation,
    #[serde(rename = "tests")]
    pub test_suite: TestSuite,
    pub result: Result<Vec<TestBlock>, Error>,
}

impl Evaluation {
    pub fn summary(&self) -> Result<Vec<(String, usize, usize)>, Error> {
        self.result
            .as_ref()
            .map(|blocks| {
                blocks
                    .iter()
                    .map(|block| {
                        let passed = block.tests.iter().filter(|test| test.passed).count();
                        let total = block.tests.len();
                        (block.name.clone(), passed, total)
                    })
                    .collect()
            })
            .map_err(|err| err.clone())
    }
}

pub fn read_evaluation_from_file<P: AsRef<Path>>(path: P) -> Vec<Evaluation> {
    let mut file = File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    serde_json::from_str(&contents[..]).unwrap()
}
