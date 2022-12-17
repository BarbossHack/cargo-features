#[derive(Debug)]
pub struct ExportInfo {
    pub root_package: Package,
    pub dependencies: Vec<Package>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub optional: bool,
    pub active: bool,
    pub globally_active: bool,
    pub features: Vec<Feature>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Feature {
    pub name: String,
    pub active: bool,
    pub childs: Vec<String>,
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
        if self.name == "default" {
            return Some(std::cmp::Ordering::Less);
        }
        match self.name.partial_cmp(&other.name) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.active.partial_cmp(&other.active) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.childs.partial_cmp(&other.childs)
    }
}
