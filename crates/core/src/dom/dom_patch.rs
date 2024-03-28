use crate::dom;
use crate::dom::dom_node::find_all_nodes;
use crate::dom::dom_node::DomInner;
use crate::dom::DomAttr;
use crate::dom::DomAttrValue;
use crate::dom::DomNode;
use crate::dom::{Application, Program};
use crate::vdom::EventCallback;
use crate::vdom::TreePath;
use crate::vdom::{Attribute, AttributeValue, Patch, PatchType};
use indexmap::IndexMap;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsValue;

/// a Patch where the virtual nodes are all created in the document.
/// This is necessary since the created Node  doesn't contain references
/// as opposed to Patch which contains reference to the vdom, which makes it hard
/// to be included in a struct
#[derive(Debug)]
pub struct DomPatch {
    /// The path to traverse to get to the target_element
    pub patch_path: TreePath,
    /// the target node
    pub target_element: DomNode,
    /// the patch variant
    pub patch_variant: PatchVariant,
}

/// patch variant
#[derive(Debug)]
pub enum PatchVariant {
    /// Insert nodes before the target node
    InsertBeforeNode {
        /// nodes to be inserted before the target node
        nodes: Vec<DomNode>,
    },
    /// Insert nodes after the target node
    InsertAfterNode {
        /// the nodes to be inserted after the target node
        nodes: Vec<DomNode>,
    },
    /// Append nodes into the target node
    AppendChildren {
        /// the children nodes to be appended into the target node
        children: Vec<DomNode>,
    },
    /// Add attributes to the target node
    AddAttributes {
        /// the attributes to be added to the target node
        attrs: Vec<DomAttr>,
    },
    /// Remove attributes from the target node
    RemoveAttributes {
        /// the attributes names to be removed
        attrs: Vec<DomAttr>,
    },
    /// Replace the target node with the replacement node
    ReplaceNode {
        /// the replacement node
        replacement: Vec<DomNode>,
    },
    /// Remove the target node
    RemoveNode,
    /// Clear the children of the target node
    ClearChildren,
    /// Move the target node before the node specified in the path location
    MoveBeforeNode {
        /// before the node at this location
        for_moving: Vec<DomNode>,
    },
    /// Move the target node after the node specified in the path location
    MoveAfterNode {
        /// after the node at this location
        for_moving: Vec<DomNode>,
    },
}

impl<APP> Program<APP>
where
    APP: Application + 'static,
{
    pub(crate) fn convert_attr(&self, attr: &Attribute<APP::MSG>) -> DomAttr {
        DomAttr {
            namespace: attr.namespace,
            name: attr.name,
            value: attr
                .value
                .iter()
                .filter_map(|v| self.convert_attr_value(v))
                .collect(),
        }
    }

    fn convert_attr_value(&self, attr_value: &AttributeValue<APP::MSG>) -> Option<DomAttrValue> {
        match attr_value {
            AttributeValue::FunctionCall(v) => Some(DomAttrValue::FunctionCall(v.clone())),
            AttributeValue::Simple(v) => Some(DomAttrValue::Simple(v.clone())),
            AttributeValue::Style(v) => Some(DomAttrValue::Style(v.clone())),
            AttributeValue::EventListener(v) => {
                Some(DomAttrValue::EventListener(self.convert_event_listener(v)))
            }
            AttributeValue::Empty => None,
        }
    }

    fn convert_event_listener(
        &self,
        event_listener: &EventCallback<APP::MSG>,
    ) -> Closure<dyn FnMut(web_sys::Event)> {
        let program = self.downgrade();
        let event_listener = event_listener.clone();
        let closure: Closure<dyn FnMut(web_sys::Event)> =
            Closure::new(move |event: web_sys::Event| {
                let msg = event_listener.emit(dom::Event::from(event));
                let mut program = program.upgrade().expect("must upgrade");
                program.dispatch(msg);
            });
        closure
    }
    /// get the real DOM target node and make a DomPatch object for each of the Patch
    pub(crate) fn convert_patches(
        &self,
        target_node: &DomNode,
        patches: &[Patch<APP::MSG>],
    ) -> Result<Vec<DomPatch>, JsValue> {
        let nodes_to_find: Vec<(&TreePath, Option<&&'static str>)> = patches
            .iter()
            .map(|patch| (patch.path(), patch.tag()))
            .chain(
                patches
                    .iter()
                    .flat_map(|patch| patch.node_paths())
                    .map(|path| (path, None)),
            )
            .collect();

        let nodes_lookup = find_all_nodes(target_node, &nodes_to_find);

        let dom_patches:Vec<DomPatch> = patches.iter().map(|patch|{
            let patch_path = patch.path();
            let patch_tag = patch.tag();
            if let Some(target_node) = nodes_lookup.get(patch_path) {
                let target_tag = target_node.tag();
                match (patch_tag, target_tag){
                    (Some(patch_tag), Some(target_tag)) => {
                        if **patch_tag != target_tag{
                            panic!(
                                "expecting a tag: {patch_tag:?}, but found: {target_tag:?}"
                            );
                        }
                    }
                    _ => (),
                }
                self.convert_patch(&nodes_lookup, target_node, patch)
            } else {
                unreachable!("Getting here means we didn't find the element of next node that we are supposed to patch, patch_path: {:?}, with tag: {:?}", patch_path, patch_tag);
            }
        }).collect();

        Ok(dom_patches)
    }
    /// convert a virtual DOM Patch into a created DOM node Patch
    pub fn convert_patch(
        &self,
        nodes_lookup: &IndexMap<TreePath, DomNode>,
        target_element: &DomNode,
        patch: &Patch<APP::MSG>,
    ) -> DomPatch {
        let target_element = target_element.clone();
        let Patch {
            patch_path,
            patch_type,
            ..
        } = patch;

        let patch_path = patch_path.clone();

        match patch_type {
            PatchType::InsertBeforeNode { nodes } => {
                let nodes = nodes
                    .iter()
                    .map(|for_insert| self.create_dom_node(None, for_insert))
                    .collect();
                DomPatch {
                    patch_path,
                    target_element,
                    patch_variant: PatchVariant::InsertBeforeNode { nodes },
                }
            }
            PatchType::InsertAfterNode { nodes } => {
                let nodes = nodes
                    .iter()
                    .map(|for_insert| self.create_dom_node(None, for_insert))
                    .collect();
                DomPatch {
                    patch_path,
                    target_element,
                    patch_variant: PatchVariant::InsertAfterNode { nodes },
                }
            }

            PatchType::AddAttributes { attrs } => {
                // we merge the attributes here prior to conversion
                let attrs = Attribute::merge_attributes_of_same_name(attrs.iter().map(|a| *a));
                DomPatch {
                    patch_path,
                    target_element,
                    patch_variant: PatchVariant::AddAttributes {
                        attrs: attrs.iter().map(|a| self.convert_attr(a)).collect(),
                    },
                }
            }
            PatchType::RemoveAttributes { attrs } => DomPatch {
                patch_path,
                target_element,
                patch_variant: PatchVariant::RemoveAttributes {
                    attrs: attrs.iter().map(|a| self.convert_attr(a)).collect(),
                },
            },

            PatchType::ReplaceNode { replacement } => {
                let replacement = replacement
                    .iter()
                    .map(|node| self.create_dom_node(None, node))
                    .collect();
                DomPatch {
                    patch_path,
                    target_element,
                    patch_variant: PatchVariant::ReplaceNode { replacement },
                }
            }
            PatchType::RemoveNode => DomPatch {
                patch_path,
                target_element,
                patch_variant: PatchVariant::RemoveNode,
            },
            PatchType::ClearChildren => DomPatch {
                patch_path,
                target_element,
                patch_variant: PatchVariant::ClearChildren,
            },
            PatchType::MoveBeforeNode { nodes_path } => {
                let for_moving = nodes_path
                    .iter()
                    .map(|path| {
                        nodes_lookup
                            .get(path)
                            .expect("must have found the node")
                            .clone()
                    })
                    .collect();
                DomPatch {
                    patch_path,
                    target_element,
                    patch_variant: PatchVariant::MoveBeforeNode { for_moving },
                }
            }
            PatchType::MoveAfterNode { nodes_path } => {
                let for_moving = nodes_path
                    .iter()
                    .map(|path| {
                        nodes_lookup
                            .get(path)
                            .expect("must have found the node")
                            .clone()
                    })
                    .collect();
                DomPatch {
                    patch_path,
                    target_element,
                    patch_variant: PatchVariant::MoveAfterNode { for_moving },
                }
            }
            PatchType::AppendChildren { children } => {
                let children = children
                    .iter()
                    .map(|for_insert| self.create_dom_node(None, for_insert))
                    .collect();

                DomPatch {
                    patch_path,
                    target_element,
                    patch_variant: PatchVariant::AppendChildren { children },
                }
            }
        }
    }

    /// TODO: this should not have access to root_node, so it can generically
    /// apply patch to any dom node
    pub(crate) fn apply_dom_patches(
        &self,
        dom_patches: impl IntoIterator<Item = DomPatch>,
    ) -> Result<Option<DomNode>, JsValue> {
        let mut new_root_node = None;
        for dom_patch in dom_patches {
            if let Some(replacement_node) = self.apply_dom_patch(dom_patch)? {
                new_root_node = Some(replacement_node);
            }
        }
        Ok(new_root_node)
    }

    /// apply a dom patch to this root node,
    /// return a new root_node if it would replace the original root_node
    /// TODO: this should have no access to root_node, so it can be used in general sense
    pub(crate) fn apply_dom_patch(&self, dom_patch: DomPatch) -> Result<Option<DomNode>, JsValue> {
        let DomPatch {
            patch_path,
            target_element,
            patch_variant,
        } = dom_patch;

        match patch_variant {
            PatchVariant::InsertBeforeNode { nodes } => {
                for for_insert in nodes {
                    target_element
                        .insert_before(for_insert)
                        .expect("must insert");
                }
                Ok(None)
            }

            PatchVariant::InsertAfterNode { nodes } => {
                // we insert the node before this target element
                for for_insert in nodes.into_iter().rev() {
                    target_element
                        .insert_after(for_insert)
                        .expect("insert after");
                }
                Ok(None)
            }
            PatchVariant::AppendChildren { children } => {
                for child in children.into_iter() {
                    target_element.append_child(child).expect("append child");
                }
                Ok(None)
            }

            PatchVariant::AddAttributes { attrs } => {
                target_element.set_dom_attrs(attrs).unwrap();
                Ok(None)
            }
            PatchVariant::RemoveAttributes { attrs } => {
                for attr in attrs.iter() {
                    for att_value in attr.value.iter() {
                        match att_value {
                            DomAttrValue::Simple(_) => {
                                target_element.remove_dom_attr(attr)?;
                            }
                            // it is an event listener
                            DomAttrValue::EventListener(_) => {
                                let DomInner::Element { listeners, .. } = &target_element.inner
                                else {
                                    unreachable!("must be an element");
                                };
                                listeners.borrow_mut().as_mut().map(|listener| {
                                    listener.retain(|event, _| *event != attr.name)
                                });
                            }
                            DomAttrValue::Style(_) => {
                                target_element.remove_dom_attr(attr)?;
                            }
                            DomAttrValue::FunctionCall(_) => {
                                let DomInner::Element {
                                    element: target_element,
                                    ..
                                } = &target_element.inner
                                else {
                                    unreachable!("must be an element");
                                };
                                if attr.name == "inner_html" {
                                    target_element.set_inner_html("");
                                }
                            }
                            DomAttrValue::Empty => (),
                        }
                    }
                }
                Ok(None)
            }

            // This also removes the associated closures and event listeners to the node being replaced
            // including the associated closures of the descendant of replaced node
            // before it is actully replaced in the DOM
            PatchVariant::ReplaceNode { mut replacement } => {
                let first_node = replacement.remove(0);
                if target_element.is_fragment() {
                    assert!(
                        patch_path.is_empty(),
                        "this should only happen to root node"
                    );
                    let mount_node = self.mount_node.borrow();
                    let mount_node = mount_node.as_ref().expect("must have a mount node");
                    mount_node.append_child(first_node.clone()).unwrap();
                    for replace_node in replacement {
                        mount_node
                            .append_child(replace_node)
                            .expect("append root_node");
                    }
                } else {
                    target_element.replace_node(first_node.clone())?;
                    for replace_node in replacement.into_iter().rev() {
                        log::info!("Inserting the rest, after the first node: {}", replace_node.render_to_string());
                        first_node.insert_after(replace_node)?;
                    }
                }
                // always return the first_node as the new root_node
                // TODO: maybe use multiple root nodes
                if patch_path.path.is_empty() {
                    Ok(Some(first_node))
                } else {
                    Ok(None)
                }
            }
            PatchVariant::RemoveNode => {
                target_element.remove_node();
                Ok(None)
            }
            PatchVariant::ClearChildren => {
                target_element.clear_children();
                Ok(None)
            }
            PatchVariant::MoveBeforeNode { for_moving } => {
                if let Some(target_parent) = target_element.parent.borrow().as_ref() {
                    for move_node in for_moving {
                        target_parent.remove_child(&move_node);
                        target_element.insert_before(move_node).unwrap();
                    }
                } else {
                    panic!("unable to get the parent node of the target element");
                }
                Ok(None)
            }

            PatchVariant::MoveAfterNode { for_moving } => {
                if let Some(target_parent) = target_element.parent.borrow().as_ref() {
                    for move_node in for_moving {
                        target_parent.remove_child(&move_node);
                        target_element.insert_after(move_node).unwrap();
                    }
                }
                Ok(None)
            }
        }
    }
}
