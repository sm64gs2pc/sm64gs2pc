#![recursion_limit = "256"]

use heck::KebabCase;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use yew::prelude::*;

/// Main app component
struct App {
    /// Link to self
    link: ComponentLink<Self>,

    /// Name of the cheat
    cheat_name: String,

    /// The GameShark code to convert
    gameshark_code: String,

    /// Output of patch conversion. The patch is displayed in blue and errors
    /// are in red.
    output: Result<String, String>,
}

/// Main component message
enum Msg {
    /// Cheat name was edited
    InputCheatName {
        /// New cheat name
        cheat_name: String,
    },
    /// GameShark code was edited
    InputGameSharkCode {
        /// New GameShark code
        gameshark_code: String,
    },
    /// Patch download button was clicked
    DownloadPatch,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create((): Self::Properties, link: ComponentLink<Self>) -> Self {
        App {
            link,
            cheat_name: String::new(),
            gameshark_code: String::new(),
            output: Err(String::from("No code entered")),
        }
    }

    fn change(&mut self, (): Self::Properties) -> bool {
        false
    }

    fn update(&mut self, msg: Msg) -> ShouldRender {
        match msg {
            Msg::InputCheatName { cheat_name } => self.cheat_name = cheat_name,
            Msg::InputGameSharkCode { gameshark_code } => self.gameshark_code = gameshark_code,
            Msg::DownloadPatch => {
                if let Ok(patch) = &self.output {
                    download_patch(&self.get_filename(), patch)
                }
            }
        }
        self.output = self.generate_output();
        true
    }

    fn view(&self) -> Html {
        let output = match &self.output {
            Ok(patch) => html!(<pre style="color: blue"> { patch } </pre>),
            Err(err) => html!(<pre style="color: red"> { err } </pre>),
        };

        html! {
            <>
                <h1 style="font-family: sans-serif"> { "sm64gs2pc" } </h1>
                <hr />
                <input
                    type="text"
                    placeholder="Cheat name"
                    oninput=self.link.callback(|input_data: InputData| {
                        Msg::InputCheatName { cheat_name: input_data.value }
                    })
                />
                <br />
                <textarea
                    placeholder="GameShark code"
                    oninput=self.link.callback(|input_data: InputData| {
                        Msg::InputGameSharkCode { gameshark_code: input_data.value }
                    })
                />
                <br />
                <button
                    disabled=self.output.is_err()
                    onclick=self.link.callback(|_| Msg::DownloadPatch)
                >
                    { format!("Download {}", self.get_filename()) }
                </button>
                <hr />
                { output }
            </>
        }
    }
}

impl App {
    fn generate_output(&self) -> Result<String, String> {
        let code = self
            .gameshark_code
            .parse::<sm64gs2pc::gameshark::Code>()
            .map_err(|err| err.to_string())?;

        let patch = sm64gs2pc::DECOMP_DATA_STATIC
            .gs_code_to_patch(&self.cheat_name, code)
            .map_err(|err| err.to_string())?;

        Ok(patch)
    }

    fn get_filename(&self) -> String {
        format!(
            "{}.patch",
            format!("sm64gs2pc-{}", self.cheat_name).to_kebab_case()
        )
    }
}

fn download_patch(filename: &str, patch: &str) {
    let document = web_sys::window()
        .expect("window")
        .document()
        .expect("document");

    let anchor = document
        .create_element("a")
        .expect("create <a>")
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .expect("dyn into <a>");

    let url = format!(
        "data:text/plain;charset=utf-8,{}",
        String::from(js_sys::encode_uri_component(patch))
    );

    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor
        .style()
        .set_property("display", "none")
        .expect("a.style.display = none");

    let body = document.body().expect("document.body");
    let node = web_sys::Node::from(anchor.clone());

    body.append_child(&node).expect("body.append_child(a)");
    anchor.click();
    body.remove_child(&node).expect("body.remove_child(a)");
}

/// App entry point
#[wasm_bindgen(start)]
pub fn run_app() {
    yew::start_app::<App>()
}
