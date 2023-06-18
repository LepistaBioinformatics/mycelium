use std::{cell::RefCell, rc::Rc};
use validator::ValidationErrors;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    #[prop_or("text".to_string())]
    pub input_type: String,
    pub label: String,
    pub name: String,
    pub input_ref: NodeRef,
    pub handle_onchange: Callback<String>,
    pub handle_on_input_blur: Callback<(String, String)>,
    pub errors: Rc<RefCell<ValidationErrors>>,
}

#[function_component(FormInput)]
pub fn form_input_component(props: &Props) -> Html {
    let val_errors = props.errors.borrow();
    let errors = val_errors.field_errors().clone();
    let empty_errors = vec![];
    let error = match errors.get(&props.name.as_str()) {
        Some(error) => error,
        None => &empty_errors,
    };
    let error_message = match error.get(0) {
        Some(message) => message.to_string(),
        None => "".to_string(),
    };

    let handle_onchange = props.handle_onchange.clone();
    let onchange = Callback::from(move |event: Event| {
        let target = event.target().unwrap();
        let value = target.unchecked_into::<HtmlInputElement>().value();
        handle_onchange.emit(value);
    });

    let handle_on_input_blur = props.handle_on_input_blur.clone();
    let on_blur = {
        let cloned_input_name = props.name.clone();
        Callback::from(move |event: FocusEvent| {
            let input_name = cloned_input_name.clone();
            let target = event.target().unwrap();
            let value = target.unchecked_into::<HtmlInputElement>().value();
            handle_on_input_blur.emit((input_name, value));
        })
    };

    html! {
    <div class="mb-6">
      <input
        type={props.input_type.clone()}
        placeholder={props.label.clone()}
        class="form-control block w-full px-4 py-5 text-sm font-normal text-gray-700 bg-white bg-clip-padding border border-solid border-gray-300 rounded transition ease-in-out m-0 focus:text-gray-700 focus:bg-white focus:border-blue-600 focus:outline-none"
        ref={props.input_ref.clone()}
        onchange={onchange}
        onblur={on_blur}
      />
    <span class="text-red-700 text-sm mt-1">
        {error_message}
    </span>
    </div>
    }
}
