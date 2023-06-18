use crate::svc::user_api::api_login_user;
//use crate::api::user_api::api_login_user;
use crate::components::form_input::FormInput;
use crate::components::header::Header;
use crate::router;
use crate::store::{set_page_loading, set_show_alert, Store};
use crate::utils::get_google_url::get_google_url;

use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use validator::{Validate, ValidationErrors};
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::prelude::*;
use yew_router::prelude::*;
use yewdux::prelude::*;

#[derive(Validate, Debug, Default, Clone, Serialize, Deserialize)]

struct LoginUserSchema {
    #[validate(
        length(min = 1, message = "Email is required"),
        email(message = "Email is invalid")
    )]
    email: String,
    #[validate(
        length(min = 1, message = "Password is required"),
        length(min = 6, message = "Password must be at least 6 characters")
    )]
    password: String,
}

fn get_input_callback(
    name: &'static str,
    cloned_form: UseStateHandle<LoginUserSchema>,
) -> Callback<String> {
    Callback::from(move |value| {
        let mut data = cloned_form.deref().clone();
        match name {
            "email" => data.email = value,
            "password" => data.password = value,
            _ => (),
        }
        cloned_form.set(data);
    })
}

#[function_component(LoginPage)]
pub fn login_page() -> Html {
    let (_, dispatch) = use_store::<Store>();
    let form = use_state(|| LoginUserSchema::default());
    let validation_errors =
        use_state(|| Rc::new(RefCell::new(ValidationErrors::new())));
    let navigator = use_navigator().unwrap();

    let email_input_ref = NodeRef::default();
    let password_input_ref = NodeRef::default();

    let validate_input_on_blur = {
        let cloned_form = form.clone();
        let cloned_validation_errors = validation_errors.clone();
        Callback::from(move |(name, value): (String, String)| {
            let mut data = cloned_form.deref().clone();
            match name.as_str() {
                "email" => data.email = value,
                "password" => data.password = value,
                _ => (),
            }
            cloned_form.set(data);

            match cloned_form.validate() {
                Ok(_) => {
                    cloned_validation_errors
                        .borrow_mut()
                        .errors_mut()
                        .remove(name.as_str());
                }
                Err(errors) => {
                    cloned_validation_errors
                        .borrow_mut()
                        .errors_mut()
                        .retain(|key, _| key != &name);
                    for (field_name, error) in errors.errors() {
                        if field_name == &name {
                            cloned_validation_errors
                                .borrow_mut()
                                .errors_mut()
                                .insert(field_name.clone(), error.clone());
                        }
                    }
                }
            }
        })
    };

    let handle_email_input = get_input_callback("email", form.clone());
    let handle_password_input = get_input_callback("password", form.clone());

    let on_submit = {
        let cloned_form = form.clone();
        let cloned_validation_errors = validation_errors.clone();
        let store_dispatch = dispatch.clone();
        let cloned_navigator = navigator.clone();

        let cloned_email_input_ref = email_input_ref.clone();
        let cloned_password_input_ref = password_input_ref.clone();

        Callback::from(move |event: SubmitEvent| {
            event.prevent_default();

            let dispatch = store_dispatch.clone();
            let form = cloned_form.clone();
            let validation_errors = cloned_validation_errors.clone();
            let navigator = cloned_navigator.clone();

            let email_input_ref = cloned_email_input_ref.clone();
            let password_input_ref = cloned_password_input_ref.clone();

            spawn_local(async move {
                match form.validate() {
                    Ok(_) => {
                        let form_data = form.deref().clone();
                        set_page_loading(true, dispatch.clone());

                        let email_input =
                            email_input_ref.cast::<HtmlInputElement>().unwrap();
                        let password_input = password_input_ref
                            .cast::<HtmlInputElement>()
                            .unwrap();

                        email_input.set_value("");
                        password_input.set_value("");

                        let form_json =
                            serde_json::to_string(&form_data).unwrap();
                        let res = api_login_user(&form_json).await;
                        match res {
                            Ok(_) => {
                                set_page_loading(false, dispatch);
                                navigator.push(&router::Route::ProfilePage);
                            }
                            Err(e) => {
                                set_page_loading(false, dispatch.clone());
                                set_show_alert(e.to_string(), dispatch);
                            }
                        };
                    }
                    Err(e) => {
                        validation_errors.set(Rc::new(RefCell::new(e)));
                    }
                }
            });
        })
    };

    html! {
    <>
      <Header />
      <section class="bg-ct-blue-600 min-h-screen pt-20">
      <div class="container mx-auto px-6 py-12 h-full flex justify-center items-center">
        <div class="md:w-8/12 lg:w-5/12 bg-white px-8 py-10">
          <form
            onsubmit={on_submit}
          >
            <FormInput label="Email" name="email" input_type="email" input_ref={email_input_ref} handle_onchange={handle_email_input} errors={&*validation_errors} handle_on_input_blur={validate_input_on_blur.clone()} />
            <FormInput label="Password" name="password" input_type="password" input_ref={password_input_ref} handle_onchange={handle_password_input} errors={&*validation_errors} handle_on_input_blur={validate_input_on_blur.clone()}/>

            <div class="flex justify-between items-center mb-6">
              <div class="form-group form-check">
                <input
                  type="checkbox"
                  class="form-check-input appearance-none h-4 w-4 border border-gray-300 rounded-sm bg-white checked:bg-blue-600 checked:border-blue-600 focus:outline-none transition duration-200 mt-1 align-top bg-no-repeat bg-center bg-contain float-left mr-2 cursor-pointer"
                  id="exampleCheck3"
                />
                <label
                  class="form-check-label inline-block text-gray-800"
                  html="exampleCheck2"
                >
                  {"Remember me"}
                </label>
              </div>
              <a
                href="#!"
                class="text-blue-600 hover:text-blue-700 focus:text-blue-700 active:text-blue-800 duration-200 transition ease-in-out"
              >
                {"Forgot password?"}
              </a>
            </div>

            <button
              type="submit"
              class="inline-block px-7 py-4 bg-blue-600 text-white font-medium text-sm leading-snug uppercase rounded shadow-md hover:bg-blue-700 hover:shadow-lg focus:bg-blue-700 focus:shadow-lg focus:outline-none focus:ring-0 active:bg-blue-800 active:shadow-lg transition duration-150 ease-in-out w-full"
            >
              {"Sign in"}
            </button>

            <div class="flex items-center my-4 before:flex-1 before:border-t before:border-gray-300 before:mt-0.5 after:flex-1 after:border-t after:border-gray-300 after:mt-0.5">
              <p class="text-center font-semibold mx-4 mb-0">{"OR"}</p>
            </div>

            <a
              class="px-7 py-2 text-white font-medium text-sm leading-snug uppercase rounded shadow-md hover:shadow-lg focus:shadow-lg focus:outline-none focus:ring-0 active:shadow-lg transition duration-150 ease-in-out w-full flex justify-center items-center mb-3"
              style={ "background-color: #3b5998" }
              href={get_google_url(Some("/profile"))}
              role="button"
            >
              <img
                class="pr-2"
                src="assets/google.svg"
                alt=""
                 style={"height: 2.2rem"}
              />
             {" Continue with Google"}
            </a>
          </form>
          </div>
      </div>
    </section>
    </>
    }
}
