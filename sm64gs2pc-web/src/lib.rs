use heck::ToKebabCase;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use yew::prelude::*;

/// Main app component
struct App {
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

    fn create(_: &Context<Self>) -> Self {
        App {
            cheat_name: String::new(),
            gameshark_code: String::new(),
            output: Err(String::from("No code entered")),
        }
    }

    fn changed(&mut self, _: &Context<Self>) -> bool {
        false
    }

    fn update(&mut self, _: &Context<Self>, msg: Msg) -> bool {
        match msg {
            Msg::InputCheatName { cheat_name } => self.cheat_name = cheat_name,
            Msg::InputGameSharkCode { gameshark_code } => self.gameshark_code = gameshark_code,
            Msg::DownloadPatch => {
                if let Ok(patch) = &self.output {
                    download_text_file(&self.get_filename(), patch)
                }
            }
        }
        self.output = self.generate_output();
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let output = match &self.output {
            Ok(patch) => html! {
                <pre style="color: blue"> { patch } </pre>
            },
            Err(err) => html! {
                <>
                    <pre style="color: red"> { format!("Error: {}", err) } </pre>
                    <p>
                        { "See the " }
                        <a href="https://github.com/sm64gs2pc/sm64gs2pc#limitations">
                            { "limitations" }
                        </a>
                        { "." }
                    </p>
                </>
            },
        };

        html! {
            <>
                <h1> { "sm64gs2pc" } </h1>
                <p> { "Convert Super Mario 64 GameShark codes to SM64 PC port patches" } </p>
                <a href="https://github.com/sm64gs2pc/sm64gs2pc"> { "GitHub repo" } </a>

                <hr />

                <h2> { "Base patches" } </h2>
                <p> { "Apply one of these before the cheat patch." } </p>
                <p> { "After applying a base patch, you can apply an unlimited amount of GameShark code patches." } </p>
                <ul>
                    <li>
                        <a href="/pkg/gameshark-base-sm64-port.patch">
                            { "sm64-port" }
                        </a>
                    </li>
                    <li>
                        <a href="/pkg/gameshark-base-sm64ex-nightly.patch">
                            { "sm64ex nightly" }
                        </a>
                    </li>
                </ul>

                <hr />

                <h2> { "Convert GameShark code to PC port patch" } </h2>
                // Cheat name input
                <input
                    type="text"
                    placeholder="Cheat name"
                    oninput={
                        ctx.link().callback(|input: InputEvent| {
                            Msg::InputCheatName { cheat_name: input.data().unwrap() }
                        })
                    }
                />
                <br />
                // Gameshark code input
                <textarea
                    placeholder="GameShark code"
                    oninput={
                        ctx.link().callback(|input: InputEvent| {
                            Msg::InputGameSharkCode { gameshark_code: input.data().unwrap() }
                        })
                    }
                />
                <br />
                // Patch download button
                <button
                    disabled={ self.output.is_err() }
                    onclick={ ctx.link().callback(|_| Msg::DownloadPatch) }
                >
                    { format!("Download {}", self.get_filename()) }
                </button>

                <hr />

                // Patch preview or error
                <h2> { "Output" } </h2>
                { output }

                <hr />

                <img alt="logo" src="https://raw.githubusercontent.com/sm64gs2pc/sm64gs2pc/master/logo.png" />
            </>
        }
    }
}

impl App {
    /// Generate output of patch conversion
    fn generate_output(&self) -> Result<String, String> {
        // Parse GameShark code
        let code = self
            .gameshark_code
            .parse::<sm64gs2pc::gameshark::Code>()
            .map_err(|err| err.to_string())?;

        // Convert to patch
        let patch = sm64gs2pc::DECOMP_DATA_STATIC
            .gs_code_to_patch(&self.cheat_name, code)
            .map_err(|err| err.to_string())?;

        Ok(patch)
    }

    /// Filename for downloading patch
    fn get_filename(&self) -> String {
        format!(
            "{}.patch",
            format!("gameshark-{}", self.cheat_name).to_kebab_case()
        )
    }
}

/// Download a text file with a given filename and text
fn download_text_file(filename: &str, file_text: &str) {
    // Get document
    let document = web_sys::window()
        .expect("window")
        .document()
        .expect("document");

    // Make <a> tag
    let anchor = document
        .create_element("a")
        .expect("create <a>")
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .expect("dyn into <a>");

    // Make URL of file
    let url = format!(
        "data:text/plain;charset=utf-8,{}",
        String::from(js_sys::encode_uri_component(file_text))
    );

    // Make <a> download file
    anchor.set_href(&url);
    anchor.set_download(filename);

    // Make <a> invisible
    anchor
        .style()
        .set_property("display", "none")
        .expect("a.style.display = none");

    let body = document.body().expect("document.body");
    let node = web_sys::Node::from(anchor.clone());

    // Add <a> to <body>
    body.append_child(&node).expect("body.append_child(a)");
    // Click the download link
    anchor.click();
    // Remove <a> from <body>
    body.remove_child(&node).expect("body.remove_child(a)");
}

/// App entry point
#[wasm_bindgen(start)]
pub fn run_app() {
    yew::start_app::<App>();
}
