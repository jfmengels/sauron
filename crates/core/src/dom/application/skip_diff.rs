use crate::vdom::TreePath;
use std::fmt;

/// if the expression evaluates to true,
/// diffing at this node will be skipped entirely
pub struct SkipDiff {
    expr: Box<dyn Fn() -> bool>,
    children: Vec<SkipDiff>,
}

impl PartialEq for SkipDiff{

    fn eq(&self, other: &Self) -> bool {
        (self.expr)() == (other.expr)()
            && self.children == other.children
    }
}

impl fmt::Debug for SkipDiff {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({},", (self.expr)())?;
        f.debug_list().entries(self.children.iter()).finish();
        write!(f, ")")?;
        Ok(())
    }
}

impl SkipDiff {
    /// new
    pub fn new(val: bool, children: impl IntoIterator<Item = Self>) -> Self {
        Self {
            expr: Box::new(move || val),
            children: children.into_iter().collect(),
        }
    }


    ///
    pub fn traverse(evals: &[SkipDiff]) -> Vec<TreePath> {
        let root = TreePath::root();
        if evals.len() == 1 {
            Self::traverse_recursive(&evals[0], root)
        } else {
            Self::traverse_list(evals, root)
        }
    }

    /// traverse the skip diff and return a list of TreePath that will be evaluated
    /// by the program
    fn traverse_list(evals: &[SkipDiff], current: TreePath) -> Vec<TreePath> {
        let mut paths = vec![];
        for (i, eval) in evals.iter().enumerate() {
            let more_paths = eval.traverse_recursive(current.traverse(i));
            paths.extend(more_paths);
        }
        paths
    }

    fn traverse_recursive(&self, current: TreePath) -> Vec<TreePath> {
        let mut paths = vec![];
        // if this SkipDiff evaluates to true, include it in the treepath to be diff
        if !(self.expr)() {
            paths.push(current.clone());
        }
        let more_paths = Self::traverse_list(&self.children, current);
        paths.extend(more_paths);
        paths
    }

    /// return true if this skip diff and its children can be skipped
    fn is_skippable_recursive(&self) -> bool {
        (self.expr)() && self.children.iter().all(Self::is_skippable_recursive)
    }

}


/// skip diffing the node is the val is true
pub fn skip_if(val: bool, children: impl IntoIterator<Item = SkipDiff>) -> SkipDiff {
    SkipDiff::new(val, children)
}