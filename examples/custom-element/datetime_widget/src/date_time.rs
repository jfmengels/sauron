use sauron::dom::MountProcedure;
use sauron::wasm_bindgen::JsCast;
use sauron::{
    custom_element, html::attributes::*, html::events::*, html::*, jss, vdom::Callback,
    wasm_bindgen, web_sys, Attribute, Effects, JsValue, Node, WebComponent, *,
};
use std::fmt::Debug;
use sauron::dom::StatefulComponent;
use sauron::dom::DomAttr;
use sauron::dom::template;
use sauron::dom::DomAttrValue;
use sauron::vdom::AttributeName;
use sauron::dom::DomNode;

#[derive(Debug, Clone)]
pub enum Msg {
    DateChange(String),
    TimeChange(String),
    TimeOrDateModified(String),
    IntervalChange(f64),
    Mounted(MountEvent),
    ExternContMounted(web_sys::Node),
    BtnClick,
}

#[derive(Debug, Clone)]
pub struct DateTimeWidget<XMSG> {
    /// the host element the web editor is mounted to, when mounted as a custom web component
    host_element: Option<web_sys::Element>,
    date: String,
    time: String,
    cnt: i32,
    time_change_listener: Vec<Callback<String, XMSG>>,
    children: Vec<web_sys::Node>,
    external_children_node: Option<web_sys::Node>,
}

impl<XMSG> Default for DateTimeWidget<XMSG> {
    fn default() -> Self {
        Self {
            host_element: None,
            date: String::new(),
            time: String::new(),
            cnt: 0,
            time_change_listener: vec![],
            children: vec![],
            external_children_node: None,
        }
    }
}

impl<XMSG> DateTimeWidget<XMSG>
where
    XMSG: 'static,
{
    pub fn new(date: &str, time: &str) -> Self {
        DateTimeWidget {
            date: date.to_string(),
            time: time.to_string(),
            cnt: 0,
            ..Default::default()
        }
    }

    fn date_time(&self) -> String {
        format!("{} {}", self.date, self.time)
    }

    pub fn on_date_time_change<F>(mut self, f: F) -> Self
    where
        F: Fn(String) -> XMSG + 'static,
    {
        self.time_change_listener.push(Callback::from(f));
        self
    }
}

impl<XMSG> sauron::Component<Msg, XMSG> for DateTimeWidget<XMSG>
where
    XMSG: 'static,
{
    fn update(&mut self, msg: Msg) -> Effects<Msg, XMSG> {
        match msg {
            Msg::DateChange(date) => {
                log::trace!("date is changed to: {}", date);
                self.date = date;
                Effects::with_local(vec![Msg::TimeOrDateModified(self.date_time())])
            }
            Msg::TimeChange(time) => {
                log::trace!("time is changed to: {}", time);
                self.time = time;
                Effects::with_local(vec![Msg::TimeOrDateModified(self.date_time())])
            }
            Msg::TimeOrDateModified(date_time) => {
                log::trace!("time or date is changed: {}", date_time);
                let mut parent_msg = vec![];
                for listener in self.time_change_listener.iter() {
                    let pmsg = listener.emit(self.date_time());
                    parent_msg.push(pmsg);
                }
                if let Some(host_element) = self.host_element.as_ref() {
                    host_element
                        .set_attribute("date_time", &date_time)
                        .expect("change attribute");
                    host_element
                        .dispatch_event(&InputEvent::create_web_event_composed())
                        .expect("must dispatch event");
                } else {
                    log::debug!("There is no host_element");
                }
                Effects::with_external(parent_msg)
            }
            Msg::Mounted(mount_event) => {
                let mount_element: web_sys::Element = mount_event.target_node.unchecked_into();
                let root_node = mount_element.get_root_node();
                if let Some(shadow_root) = root_node.dyn_ref::<web_sys::ShadowRoot>() {
                    log::info!("There is a shadow root");
                    let host_element = shadow_root.host();
                    self.host_element = Some(host_element);
                } else {
                    log::warn!("There is no shadow root");
                }
                Effects::none()
            }
            Msg::ExternContMounted(target_node) => {
                log::info!("DateTime: extenal container mounted...");
                for child in self.children.iter(){
                    target_node.append_child(child).expect("must append");
                }
                self.external_children_node = Some(target_node);
                Effects::none()
            }
            Msg::BtnClick => {
                log::trace!("btn is clicked..");
                self.cnt += 1;
                Effects::none()
            }
            Msg::IntervalChange(interval) => {
                log::trace!("There is an interval: {}", interval);
                Effects::none()
            }
        }
    }

    fn stylesheet() -> Vec<String> {
        vec![jss! {
            ".datetimebox":{
                border: "1px solid green",
            },
            "button": {
              background: "#1E88E5",
              color: "white",
              padding: "10px 10px",
              margin: "10px 10px",
              border: 0,
              font_size: "1.5rem",
              border_radius: "5px",
            }
        }]
    }

    fn observed_attributes() -> Vec<AttributeName> {
        vec!["date", "time", "interval"]
    }


    fn view(&self) -> Node<Msg> {
        div(
            [class("datetimebox"), on_mount(Msg::Mounted)],
            [
                input(
                    [
                        r#type("date"),
                        class("datetimebox__date"),
                        on_change(|input| {
                            log::trace!("input: {:?}", input);
                            Msg::DateChange(input.value())
                        }),
                        value(&self.date),
                    ],
                    [],
                ),
                input(
                    [
                        r#type("time"),
                        class("datetimebox__time"),
                        on_change(|input| Msg::TimeChange(input.value())),
                        value(&self.time),
                    ],
                    [],
                ),
                input([r#type("text"), value(self.cnt)], []),
                button([on_click(move |_| Msg::BtnClick)], [text("Do something")]),
                div([class("external_children"), on_mount(|me|Msg::ExternContMounted(me.target_node))], [])
            ],
        )
    }
}

impl StatefulComponent for DateTimeWidget<()>{

    fn build(
        attrs: impl IntoIterator<Item = DomAttr>,
        children: impl IntoIterator<Item = web_sys::Node>,
    ) -> Self
    where
        Self: Sized,
    {
        DateTimeWidget::default()
    }

    fn template(&self) -> web_sys::Node {
        template::build_template(&Component::view(self))
    }

    /// this is called when the attributes in the mount is changed
    fn attribute_changed(
        &mut self,
        attr_name: &str,
        old_value: DomAttrValue,
        new_value: DomAttrValue,
    ) where
        Self: Sized,
    {
        match &*attr_name {
            "time" => {
                if let Some(new_value) = new_value.get_string(){
                    Component::update(self, Msg::TimeChange(new_value));
                }
            }
            "date" => {
                if let Some(new_value) = new_value.get_string(){
                    Component::update(self, Msg::DateChange(new_value));
                }
            }
            "interval" => {
                if let Some(new_value) = new_value.get_string() {
                    let new_value: f64 = str::parse(&new_value).expect("must parse to f64");
                    Component::update(self, Msg::IntervalChange(new_value));
                }
            }
            _ => log::warn!("unknown attr_name: {attr_name:?}"),
        }
    }

    fn append_child(&mut self, child: &web_sys::Node) {
        log::info!("DateTime: appending child{:?}", child.inner_html());
        if let Some(external_children_node) = self.external_children_node.as_ref(){
            log::info!("DateTime: ok appending..");
            external_children_node.append_child(child).expect("must append");
        }else{
            log::debug!("DateTime: Just pushing to children since the external holder is not yet mounted");
            self.children.push(child.clone());
        }
    }

    fn remove_attribute(&mut self, attr_name: AttributeName) {}

    fn remove_child(&mut self, index: usize) {}

    fn connected_callback(&mut self) {}

    fn disconnected_callback(&mut self) {}

    fn adopted_callback(&mut self) {}
}




#[wasm_bindgen]
pub struct DateTimeCustomElement {
    program: Program<DateTimeWidget<()>, Msg>,
    mount_node: web_sys::Node,
}

#[wasm_bindgen]
impl DateTimeCustomElement {
    #[wasm_bindgen(constructor)]
    pub fn new(node: JsValue) -> Self {
        let mount_node: web_sys::Node = node.unchecked_into();
        Self {
            program: Program::new(DateTimeWidget::<()>::default()),
            mount_node,
        }
    }

    #[allow(unused_variables)]
    #[wasm_bindgen(getter, static_method_of = Self, js_name = observedAttributes)]
    pub fn observed_attributes() -> JsValue {
        let attributes = DateTimeWidget::<()>::observed_attributes();
        serde_wasm_bindgen::to_value(&attributes).expect("convert to value")
    }

    #[wasm_bindgen(method, js_name = attributeChangedCallback)]
    pub fn attribute_changed_callback(
        &self,
        attr_name: &str,
        old_value: JsValue,
        new_value: JsValue,
    ) {

         self.program.app_mut().attribute_changed(
            attr_name,
            old_value.into(),
            new_value.into(),
        );
    }

    #[wasm_bindgen(method, js_name = connectedCallback)]
    pub fn connected_callback(&mut self) {
        self.program
            .mount(&self.mount_node, MountProcedure::append_to_shadow());

        let static_style = <DateTimeWidget<()> as Application<Msg>>::stylesheet().join("");
        self.program.inject_style_to_mount(&static_style);
        let dynamic_style =
            <DateTimeWidget<()> as Application<Msg>>::style(&self.program.app()).join("");
        self.program.inject_style_to_mount(&dynamic_style);

        self.program
            .update_dom(&sauron::dom::Modifier::default(), None)
            .expect("must update dom");
    }

    #[wasm_bindgen(method, js_name = disconnectedCallback)]
    pub fn disconnected_callback(&mut self) {}

    #[wasm_bindgen(method, js_name = adoptedCallback)]
    pub fn adopted_callback(&mut self) {}

    pub fn register() {
        let constructor: Closure<dyn FnMut(JsValue)> = Closure::new(|node: JsValue| {
            let new: Closure<dyn FnMut(JsValue) -> Self> =
                Closure::new(|node: JsValue| Self::new(node));
            js_sys::Reflect::set(&node, &JsValue::from_str("new"), &new.into_js_value())
                .unwrap_throw();
        });
        sauron::dom::register_web_component(
            "date-time",
            constructor.into_js_value(),
            Self::observed_attributes(),
        );
    }
}
pub fn register() {
    DateTimeCustomElement::register();
}

pub fn date<MSG, V: Into<Value>>(v: V) -> Attribute<MSG> {
    attr("date", v)
}

pub fn time<MSG, V: Into<Value>>(v: V) -> Attribute<MSG> {
    attr("time", v)
}

pub fn date_time<MSG>(
    attrs: impl IntoIterator<Item = Attribute<MSG>>,
    children: impl IntoIterator<Item = Node<MSG>>,
) -> Node<MSG> {
    register();
    html_element(None, "date-time", attrs, children, true)
}
