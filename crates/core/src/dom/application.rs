use crate::vdom::Node;
pub use skip_diff::{skip_if, SkipDiff, SkipPath};
use crate::dom::Cmd;

///
pub mod skip_diff;

/// An Application is the root component of your program.
/// Everything that happens in your application is done here.
///
pub trait Application: Sized + 'static {
    ///
    type MSG;
    ///  The application can implement this method where it can modify its initial state.
    ///  This method is called right after the program is mounted into the DOM.
    fn init(&mut self) -> Cmd<Self::MSG> {
        Cmd::none()
    }

    /// Update the component with a message.
    /// The update function returns a Dispatch, which can be executed by the runtime.
    ///
    /// Called each time an action is triggered from the view
    fn update(&mut self, _msg: Self::MSG) -> Cmd<Self::MSG>;

    /// Returns a node on how the component is presented.
    fn view(&self) -> Node<Self::MSG>;

    /// The css style for the application, will be mounted automatically by the program
    fn stylesheet() -> Vec<String> {
        vec![]
    }

    /// dynamic style of an application which will be reinjected when the application style changed
    fn style(&self) -> Vec<String> {
        vec![]
    }

    /// This is called after dispatching and updating the dom for the component
    /// This is for diagnostic and performance measurement purposes.
    ///
    /// Warning: DO NOT use for anything else other than the intended purpose
    fn measurements(&mut self, _measurements: Measurements){
    }
}

/// Contains the time it took for the last app update call for the component
/// TODO: Maybe rename to Diagnostics
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Measurements {
    /// The number of DOM nodes in this Component
    pub node_count: usize,
    /// Time it took for the Component to build it's view
    pub build_view_took: f64,
    /// Total number of patches applied on this update loop
    pub total_patches: usize,
    /// Time it took for the patching the DOM.
    pub dom_update_took: f64,
    /// Total time it took for the component dispatch
    pub total_time: f64,
    /// The total count reference of the Program App
    pub strong_count: usize,
    /// The total weak count reference of the Program App
    pub weak_count: usize,
}
