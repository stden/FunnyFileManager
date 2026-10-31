//! Purpose: Panel collections invalidate cached facts before mutable access.

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// A panel collection with an identity for its current contents.
///
/// Reads borrow the underlying collection. Mutable access replaces an observed
/// identity, so even in-place edits invalidate totals. Whole replacements use
/// `collection.into()`; clones remain independent when either copy changes.
#[derive(Debug, Clone)]
pub struct PanelCollection<T> {
    value: T,
    revision: Arc<()>,
}

impl<T> PanelCollection<T> {
    pub(super) fn revision(&self) -> &Arc<()> {
        &self.revision
    }
}

impl<T> From<T> for PanelCollection<T> {
    fn from(value: T) -> Self {
        Self {
            value,
            revision: Arc::new(()),
        }
    }
}

impl<T: Default> Default for PanelCollection<T> {
    fn default() -> Self {
        T::default().into()
    }
}

impl<T: PartialEq> PartialEq for PanelCollection<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for PanelCollection<T> {}

impl<'a, T: 'a> PanelCollection<T>
where
    &'a T: IntoIterator,
{
    pub fn iter(&'a self) -> <&'a T as IntoIterator>::IntoIter {
        (&self.value).into_iter()
    }
}

impl<'a, T: 'a> PanelCollection<T>
where
    &'a mut T: IntoIterator,
{
    pub fn iter_mut(&'a mut self) -> <&'a mut T as IntoIterator>::IntoIter {
        self.deref_mut().into_iter()
    }
}

impl<T> Deref for PanelCollection<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for PanelCollection<T> {
    fn deref_mut(&mut self) -> &mut T {
        // An unobserved identity can be reused until a cache or clone sees it.
        if Arc::strong_count(&self.revision) > 1 {
            self.revision = Arc::new(());
        }
        &mut self.value
    }
}

impl<A, T: FromIterator<A>> FromIterator<A> for PanelCollection<T> {
    fn from_iter<I: IntoIterator<Item = A>>(iter: I) -> Self {
        T::from_iter(iter).into()
    }
}

impl<'a, T> IntoIterator for &'a PanelCollection<T>
where
    &'a T: IntoIterator,
{
    type Item = <&'a T as IntoIterator>::Item;
    type IntoIter = <&'a T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.value).into_iter()
    }
}

impl<'a, T> IntoIterator for &'a mut PanelCollection<T>
where
    &'a mut T: IntoIterator,
{
    type Item = <&'a mut T as IntoIterator>::Item;
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.deref_mut().into_iter()
    }
}
