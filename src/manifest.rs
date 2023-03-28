use std::{
    collections::{HashMap, HashSet},
    fs,
};

use anyhow::Result;
use glob::Pattern;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

#[derive(Deserialize, Serialize, Debug)]
pub struct Target {
    pub path: String,
    #[serde(default)]
    pub globs: HashSet<String>,
    #[serde(default, alias = "activated_by")]
    pub activated_by: HashSet<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Targets {
    pub base: String,
    pub targets: HashMap<String, Target>,
}

pub struct Manifest {
    targets: Targets,
    activated: HashSet<String>,
    path_to_activator: Vec<(Pattern, String)>,
    activator_to_target: Vec<(String, String)>,
}

impl std::fmt::Debug for Manifest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let in_situ: Vec<(&str, &String)> = self
            .path_to_activator
            .iter()
            .map(|(p, t)| (p.as_str(), t))
            .collect();

        f.debug_struct("Manifest")
            .field("targets", &self.targets)
            .field("activated", &self.activated)
            .field("path_to_activator", &in_situ)
            .field("activator_to_target", &self.activator_to_target)
            .finish()
    }
}

impl From<Targets> for Manifest {
    #[tracing::instrument]
    fn from(targets: Targets) -> Self {
        let mut path_to_activator = vec![];
        let mut activator_to_target = vec![];

        for (name, target) in &targets.targets {
            let base = vec![target.path.clone()];
            let paths = target
                .globs
                .iter()
                .chain(base.iter())
                .cloned()
                .map(|p| (Pattern::new(&p).expect(""), name.clone()));
            path_to_activator.extend(paths);

            let activators = target
                .activated_by
                .iter()
                .cloned()
                .map(|t| (t, name.clone()));
            activator_to_target.extend(activators);
        }

        Self {
            targets,
            activated: Default::default(),
            path_to_activator,
            activator_to_target,
        }
    }
}

impl Manifest {
    #[tracing::instrument]
    fn new_from_str(contents: &str) -> Result<Self> {
        let targets: Targets = serde_yaml::from_str(contents)?;
        let manifest = Manifest::from(targets);
        Ok(manifest)
    }

    #[tracing::instrument]
    pub fn new_from_path(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        Self::new_from_str(&contents)
    }

    #[tracing::instrument(skip(self))]
    pub fn base<'a>(&'a self) -> &'a str {
        &self.targets.base
    }

    #[tracing::instrument(skip(self))]
    fn test_path(&self, path: &str) -> Vec<String> {
        let out = self
            .path_to_activator
            .iter()
            .filter_map(|(p, t)| {
                debug!(
                    "testing path for target {}:\n  path:    {}\n  against: {}",
                    t,
                    path,
                    p.as_str()
                );
                p.matches(path).then_some(t)
            })
            .cloned()
            .collect();

        out
    }

    #[tracing::instrument(skip(self))]
    fn inactive_targets(&self) -> impl Iterator<Item = String> + '_ {
        self.targets
            .targets
            .keys()
            .filter(|k| !self.activated.contains(*k))
            .cloned()
    }

    #[tracing::instrument(skip(self))]
    fn test_inactive_targets(&self) -> Vec<String> {
        debug!("activated targets: {:?}", self.activated);

        self.inactive_targets()
            .inspect(|t| debug!("inactive target: {t}"))
            .filter(|t| {
                let target = self.targets.targets.get(t).unwrap();
                debug!("activated by: {:?}", target.activated_by);
                target.activated_by.intersection(&self.activated).count() > 0
            })
            .collect()
    }

    #[tracing::instrument(skip(self))]
    pub fn resolve(&mut self, changed_files: &Vec<String>) {
        trace!("resolving manifest");
        self.activated = changed_files
            .iter()
            .flat_map(|p| self.test_path(p))
            .collect();

        let mut i = 0;
        loop {
            trace!("activator resolution loop {}", i);
            let activated = self.test_inactive_targets();
            if activated.is_empty() {
                break;
            }
            self.activated.extend(activated);
            i += 1;
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn activated_targets(&self) -> Vec<(String, bool)> {
        self.targets
            .targets
            .keys()
            .map(|target| (target.clone(), self.activated.contains(target)))
            .collect()
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use rstest::rstest;

    fn sv(items: Vec<&str>) -> Vec<String> {
        items.iter().map(|i| i.to_string()).collect()
    }

    fn shs(items: Vec<&str>) -> HashSet<String> {
        items.iter().map(|i| i.to_string()).collect()
    }

    #[rstest]
    #[case(sv(vec!["file-a.txt"]), shs(vec!["service-a"]))]
    #[case(sv(vec!["file-b.txt"]), shs(vec!["service-b"]))]
    #[case(sv(vec!["dep.txt"]), shs(vec!["common-dependency", "service-a", "service-b"]))]
    fn test_happy_path(#[case] files: Vec<String>, #[case] expected: HashSet<String>) {
        let mut manifest = Manifest::new_from_path("./tests/happy.yaml").expect("known good");

        manifest.resolve(&files);

        assert_eq!(manifest.activated, expected);
    }

    #[rstest]
    #[case(sv(vec!["file-a.txt"]), shs(vec!["service-a", "service-b", "service-c", "service-d"]))]
    #[case(sv(vec!["file-b.txt"]), shs(vec!["service-b", "service-c", "service-d"]))]
    #[case(sv(vec!["file-c.txt"]), shs(vec!["service-c", "service-d"]))]
    #[case(sv(vec!["file-d.txt"]), shs(vec!["service-d"]))]
    fn test_chain(#[case] files: Vec<String>, #[case] expected: HashSet<String>) {
        let mut manifest = Manifest::new_from_path("./tests/chain.yaml").expect("known good");

        manifest.resolve(&files);
        assert_eq!(manifest.activated, expected);
    }

    #[rstest]
    #[case(sv(vec!["file-a.txt"]), shs(vec!["service-a", "service-b"]))]
    #[case(sv(vec!["file-b.txt"]), shs(vec!["service-a", "service-b"]))]
    #[case(sv(vec!["file-c.txt"]), shs(vec!["service-c"]))]
    fn test_recursive_activation(#[case] files: Vec<String>, #[case] expected: HashSet<String>) {
        let mut manifest = Manifest::new_from_path("./tests/recursive.yaml").expect("known good");

        manifest.resolve(&files);
        assert_eq!(manifest.activated, expected);
    }
}
