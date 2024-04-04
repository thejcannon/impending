use std::collections::HashMap;

mod requirements_txt;
pub use requirements_txt::parse_requirements_txt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReqInfo {
    pub extras: Option<String>,
    pub version_spec: Option<String>,
    pub url_spec: Option<String>,
    pub marker_spec: Option<String>,
    pub opts: Option<String>,
}

impl ReqInfo {
    pub fn new() -> ReqInfo {
        ReqInfo {
            extras: None,
            version_spec: None,
            url_spec: None,
            marker_spec: None,
            opts: None,
        }
    }

    pub fn to_requirement(&self, pkgname: &String) -> String {
        [
            &Some(pkgname.clone()),
            &self.extras,
            &self.url_spec,
            &self.version_spec,
            &self.marker_spec,
            &self.opts,
        ]
        .into_iter()
        .filter_map(|o| o.as_ref().map(String::as_str))
        .collect::<_>()
    }
}

pub type ReqMap = HashMap<String, ReqInfo>;
pub type DepMap = HashMap<String, Vec<String>>;
