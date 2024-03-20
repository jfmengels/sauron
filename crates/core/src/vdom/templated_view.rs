use crate::dom::SkipDiff;
use std::rc::Rc;
use std::fmt;
use crate::vdom::Node;

/// Templated view
pub struct TemplatedView<MSG>{
    /// the view node
    pub view: Box<Node<MSG>>,
    /// the extracted template based on the view,
    /// this will be generated by the view macro
    pub template: Rc<dyn Fn() -> Node<MSG>>,
    /// the extracted skip diff based on the view
    /// this will be generated by the view macro
    pub skip_diff: Rc<dyn Fn() -> SkipDiff>,
}

impl<MSG> Clone for TemplatedView<MSG>{
    
    fn clone(&self) -> Self {
        Self {
            view: self.view.clone(),
            template: Rc::clone(&self.template),
            skip_diff: Rc::clone(&self.skip_diff),
        }
    }
}

impl<MSG> fmt::Debug for TemplatedView<MSG>{

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result{
        f.debug_struct("TemplatedView")
            .field("view", &self.view)
            .field("template", &(&self.template)())
            .field("skip_diff", &(&self.skip_diff)())
            .finish()
    }
}

impl<MSG> PartialEq for TemplatedView<MSG>{

    fn eq(&self, other: &Self) -> bool {
        self.view == other.view
    }
}

impl<MSG> Eq for TemplatedView<MSG> {}
