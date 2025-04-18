use anyhow::{bail, Result};
use std::path::{Path, PathBuf};

/// All-in-one rust path type,
/// currently supports single [`Path`] and multiple owned [`PathBuf`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum PathExt<'p> {
    Single(&'p Path),
    MultiOwned(Vec<PathBuf>),
}

impl Default for PathExt<'_> {
    fn default() -> Self {
        Self::MultiOwned(vec![])
    }
}

impl<'p> From<&'p Path> for PathExt<'p> {
    fn from(value: &'p Path) -> Self {
        Self::Single(value)
    }
}

impl From<Vec<PathBuf>> for PathExt<'_> {
    fn from(value: Vec<PathBuf>) -> Self {
        Self::MultiOwned(value)
    }
}

pub struct PathIterator<'p> {
    inner: &'p PathExt<'p>,
    index: usize,
}

impl<'p> Iterator for PathIterator<'p> {
    type Item = &'p Path;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner {
            PathExt::Single(path) => {
                if self.index == 0 {
                    self.index += 1;
                    Some(path)
                } else {
                    None
                }
            }
            PathExt::MultiOwned(paths) => {
                if self.index < paths.len() {
                    let res = paths.get(self.index);
                    self.index += 1;
                    res.map(|pb| pb.as_path())
                } else {
                    None
                }
            }
        }
    }
}

impl IntoIterator for PathExt<'_> {
    type Item = PathBuf;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        match self {
            Self::MultiOwned(paths) => paths.into_iter(),
            Self::Single(path) => vec![path.to_path_buf()].into_iter(),
        }
    }
}

impl PathExt<'_> {
    /// Retrieve a single `Path` reference.
    ///
    /// - If this is a [`PathExt::Single`] variant, the enclosed path will be returned.
    /// - If this is a [`PathExt::MultiOwned`] variant, but only one path is in the vector
    ///   the first element in the vector will be returned.
    ///
    /// # Error
    /// Returns error if `self` contains multiple path values.
    pub fn single(&self) -> Result<&Path> {
        match self {
            Self::Single(path) => Ok(path),
            Self::MultiOwned(paths) if paths.len() == 1 => {
                // safe to unwrap, `paths` was proven not empty
                Ok(paths.iter().map(|pb| pb.as_path()).next().unwrap())
            }
            _ => bail!("expecting single `Path` type, got '{self:?}' instead."),
        }
    }

    pub fn iter(&self) -> PathIterator {
        PathIterator {
            index: 0,
            inner: self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PathExt;
    use std::path::{Path, PathBuf};

    #[test]
    fn single_iter() {
        let single = PathExt::from(Path::new("abc"));
        let mut single_iter = single.iter();
        assert_eq!(single_iter.next(), Some(Path::new("abc")));
        assert_eq!(single_iter.next(), None);
    }

    #[test]
    fn multi_iter() {
        let multi = PathExt::from(vec![PathBuf::from("a"), PathBuf::from("b")]);
        let mut multi_iter = multi.iter();

        assert_eq!(multi_iter.next(), Some(Path::new("a")));
        assert_eq!(multi_iter.next(), Some(Path::new("b")));
        assert_eq!(multi_iter.next(), None);
    }
}
