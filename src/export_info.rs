use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportInfo {
    pub root_package: Package,
    pub dependencies: Vec<Package>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub optional: bool,
    pub active: bool,
    pub globally_active: bool,
    pub features: Vec<Feature>,
    pub optionals: Vec<Optional>,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub name: String,
    pub active: bool,
    pub optional: bool,
    pub childs: Vec<Child>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct Child {
    pub name: String,
    pub optional: bool,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct Optional {
    pub name: String,
    pub active: bool,
}

impl Ord for Feature {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.name == "default" {
            return std::cmp::Ordering::Less;
        }
        match self.name.cmp(&other.name) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        match self.active.cmp(&other.active) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.childs.cmp(&other.childs)
    }
}

impl PartialOrd for Feature {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
