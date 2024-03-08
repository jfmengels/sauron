use crate::dom::DomAttr;
use crate::dom::DomAttrValue;
use crate::html::attributes::{class, classes, Attribute};
use crate::vdom::AttributeName;
use crate::vdom::Leaf;
use crate::{dom::Effects, vdom::Node};
use std::any::TypeId;
use crate::vdom::LeafComponent;
use crate::dom::Cmd;
use crate::dom::Application;
use crate::dom::Program;
use crate::dom::program::MountProcedure;
use crate::dom::program::AppContext;
use crate::dom::events::on_mount;
use std::rc::Rc;
use std::cell::RefCell;
use crate::dom::program::ActiveClosure;
use std::collections::VecDeque;

/// A component has a view and can update itself.
///
/// The update function returns an effect which can contain
/// follow ups and effects. Follow ups are executed on the next
/// update loop of this component, while the effects are executed
/// on the parent component that mounts it.
pub trait Component<MSG, XMSG>
where
    MSG: 'static,
{
    /// init the component
    fn init(&mut self) -> Effects<MSG, XMSG> {
        Effects::none()
    }

    /// Update the model of this component and return
    /// follow up and/or effects that will be executed on the next update loop
    fn update(&mut self, msg: MSG) -> Effects<MSG, XMSG>;

    /// the view of the component
    fn view(&self) -> Node<MSG>;

    /// component can have static styles
    fn stylesheet() -> Vec<String>
    where
        Self: Sized,
    {
        vec![]
    }

    /// in addition, component can contain dynamic style
    /// which can change when the model is updated
    fn style(&self) -> Vec<String> {
        vec![]
    }

    /// return the component name
    /// defaults to the struct simplified name
    fn component_name() -> String
    where
        Self: Sized,
    {
        extract_simple_struct_name::<Self>()
    }

    /// prefix the class bane
    fn prefix_class(class_name: &str) -> String
    where
        Self: Sized,
    {
        let component_name = Self::component_name();
        if class_name.is_empty() {
            component_name
        } else {
            format!("{component_name}__{class_name}")
        }
    }

    /// create a classname prepended with this component name
    fn class_ns(class_name: &str) -> Attribute<MSG>
    where
        Self: Sized,
    {
        class(Self::prefix_class(class_name))
    }

    /// create namespaced class names to pair that evaluates to true
    fn classes_ns_flag(pair: impl IntoIterator<Item = (impl ToString, bool)>) -> Attribute<MSG>
    where
        Self: Sized,
    {
        let class_list = pair.into_iter().filter_map(|(class, flag)| {
            if flag {
                Some(Self::prefix_class(&class.to_string()))
            } else {
                None
            }
        });

        classes(class_list)
    }

    /// create a selector class prepended with this component name
    fn selector_ns(class_name: &str) -> String
    where
        Self: Sized,
    {
        let component_name = Self::component_name();
        if class_name.is_empty() {
            format!(".{component_name}")
        } else {
            format!(".{component_name}__{class_name}")
        }
    }

    /// create namesspaced selector from multiple classnames
    fn selectors_ns(class_names: impl IntoIterator<Item = impl ToString>) -> String
    where
        Self: Sized,
    {
        let selectors: Vec<String> = class_names
            .into_iter()
            .map(|class_name| Self::selector_ns(&class_name.to_string()))
            .collect();
        selectors.join(" ")
    }
}

pub(crate) fn extract_simple_struct_name<T: ?Sized>() -> String {
    let type_name = std::any::type_name::<T>();
    let name = if let Some(first) = type_name.split(['<', '>']).next() {
        first
    } else {
        type_name
    };
    name.rsplit("::")
        .next()
        .map(|s| s.to_string())
        .expect("must have a name")
}

/// A component that can be used directly in the view without mapping
pub trait StatefulComponent {
    /// create the stateful component with this attributes
    fn build(
        attrs: impl IntoIterator<Item = DomAttr>,
        children: impl IntoIterator<Item = web_sys::Node>,
    ) -> Self
    where
        Self: Sized;

    /// This will be invoked when a component is used as a custom element
    /// and the attributes of the custom-element has been modified
    ///
    /// if the listed attributes in the observed attributes are modified
    fn attribute_changed(
        &mut self,
        attr_name: AttributeName,
        old_value: DomAttrValue,
        new_value: DomAttrValue,
    ) where
        Self: Sized;

    /// build the template of this Component
    fn template(&self) -> web_sys::Node;

    /// remove the attribute with this name
    fn remove_attribute(&mut self, attr_name: AttributeName);

    /// append a child into this component
    fn append_child(&mut self, child: &web_sys::Node);

    /// remove a child in this index
    fn remove_child(&mut self, index: usize);

    /// the component is attached to the dom
    fn connected_callback(&mut self);
    /// the component is removed from the DOM
    fn disconnected_callback(&mut self);

    /// the component is moved or attached to the dom
    fn adopted_callback(&mut self);
}

impl<COMP, MSG> Application<MSG> for COMP
where
    COMP: Component<MSG, ()> + StatefulComponent + 'static,
    MSG: 'static,
{
    fn init(&mut self) -> Cmd<Self, MSG> {
        Cmd::from(<Self as Component<MSG, ()>>::init(self))
    }

    fn update(&mut self, msg: MSG) -> Cmd<Self, MSG> {
        let effects = <Self as Component<MSG, ()>>::update(self, msg);
        Cmd::from(effects)
    }

    fn view(&self) -> Node<MSG> {
        <Self as Component<MSG, ()>>::view(self)
    }

    fn stylesheet() -> Vec<String> {
        <Self as Component<MSG, ()>>::stylesheet()
    }

    fn style(&self) -> Vec<String> {
        <Self as Component<MSG, ()>>::style(self)
    }
}

/// create a stateful component node
pub fn component<COMP, MSG, MSG2>(
    attrs: impl IntoIterator<Item = Attribute<MSG>>,
    children: impl IntoIterator<Item = Node<MSG>>,
) -> Node<MSG>
where
    COMP: Component<MSG2, ()> + StatefulComponent + 'static,
    MSG: Default + 'static,
    MSG2: 'static,
{

    let type_id = TypeId::of::<COMP>();
    let attrs = attrs.into_iter().collect::<Vec<_>>();

    // Note: we can not include the children in the build function
    // as the children here contains the MSG generic
    // and we can not discard the event listeners.
    //
    // The attribute(minus events) however can be used for configurations, for setting initial state 
    // of the stateful component.
    let app = COMP::build(attrs.clone().into_iter().map(|a|DomAttr::convert_attr_except_listener(&a)), []);
    let view = app.view();
    let app = Rc::new(RefCell::new(app));

    let program = Program{
        app_context: AppContext{
            app: Rc::clone(&app),
            current_vdom: Rc::new(RefCell::new(view)),
            pending_msgs: Rc::new(RefCell::new(VecDeque::new())),
            pending_cmds: Rc::new(RefCell::new(VecDeque::new())),
        }, 
        root_node: Rc::new(RefCell::new(None)),
        mount_node: Rc::new(RefCell::new(None)),
        node_closures: Rc::new(RefCell::new(ActiveClosure::new())),
        pending_patches: Rc::new(RefCell::new(VecDeque::new())),
        idle_callback_handles: Rc::new(RefCell::new(vec![])),
        animation_frame_handles: Rc::new(RefCell::new(vec![])),
        event_closures: Rc::new(RefCell::new(vec![])),
        closures: Rc::new(RefCell::new(vec![])),
        last_update: Rc::new(RefCell::new(None)),
    };
    let children:Vec<Node<MSG>> = children.into_iter().collect();
    let mount_event = on_mount(move|me|{
        log::info!("Component is now mounted..");
        let mut program = program.clone();
        program.mount(&me.target_node, MountProcedure::append());
        MSG::default()
    });
    let node = Node::Leaf(Leaf::Component(LeafComponent{
        comp: app,
        type_id,
        attrs: attrs.into_iter().chain([mount_event]).collect(),
        children: children.into_iter().collect(),
    }));
    node
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::html::*;
    use std::marker::PhantomData;

    #[test]
    fn test_extract_component_name() {
        enum Msg {}
        struct AwesomeEditor {}

        impl Component<Msg, ()> for AwesomeEditor {
            fn update(&mut self, _msg: Msg) -> Effects<Msg, ()> {
                Effects::none()
            }
            fn view(&self) -> Node<Msg> {
                div([], [])
            }
        }

        let name = extract_simple_struct_name::<AwesomeEditor>();
        println!("name: {name}");
        assert_eq!("AwesomeEditor", name);
    }

    #[test]
    fn test_name_with_generics() {
        struct ComplexEditor<XMSG> {
            _phantom2: PhantomData<XMSG>,
        }

        enum Xmsg {}

        let name = extract_simple_struct_name::<ComplexEditor<Xmsg>>();
        println!("name: {name}");
        assert_eq!("ComplexEditor", name);
    }

    #[test]
    fn test_name_with_2_generics() {
        struct ComplexEditor<MSG, XMSG> {
            _phantom1: PhantomData<MSG>,
            _phantom2: PhantomData<XMSG>,
        }

        enum Msg {}
        enum Xmsg {}

        let name = extract_simple_struct_name::<ComplexEditor<Msg, Xmsg>>();
        println!("name: {name}");
        assert_eq!("ComplexEditor", name);
    }
}