use super::ToolInfo;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

/// A map of tools, contains the name and source package information.
///
/// This is basically a wrapper type `IndexMap`, but with tailored functionalities to suit
/// the needs of tools' installation and uninstallation.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Default, Clone)]
pub struct ToolMap(IndexMap<String, ToolInfo>);
pub struct ToolMapIter<'a> {
    iter: indexmap::map::Iter<'a, String, ToolInfo>,
}

impl<'a> Iterator for ToolMapIter<'a> {
    type Item = (&'a str, &'a ToolInfo);
    fn next(&mut self) -> Option<Self::Item> {
        let (name, info) = self.iter.next()?;
        // The `key` of each iteration prefers the identifier over the name.
        let identifier = info.identifier().unwrap_or(name.as_str());
        Some((identifier, info))
    }
}

impl ToolMap {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn iter(&self) -> ToolMapIter<'_> {
        ToolMapIter {
            iter: self.0.iter(),
        }
    }
}

impl Deref for ToolMap {
    type Target = IndexMap<String, ToolInfo>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ToolMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<(String, ToolInfo)> for ToolMap {
    fn from_iter<T: IntoIterator<Item = (String, ToolInfo)>>(iter: T) -> Self {
        Self(IndexMap::from_iter(iter))
    }
}

impl<'a> IntoIterator for &'a ToolMap {
    type Item = (&'a str, &'a ToolInfo);
    type IntoIter = ToolMapIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ToolMapIter {
            iter: self.0.iter(),
        }
    }
}
