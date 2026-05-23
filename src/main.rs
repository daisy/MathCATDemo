
use yew::prelude::*;
use yew::web_sys::Element;
// use wasm_bindgen::JsCast;
// use web_sys::{HtmlInputElement};
use regex::Regex;
#[macro_use]
extern crate lazy_static;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use cfg_if::cfg_if;
use libmathcat::*;


cfg_if! {
    if #[cfg(feature = "console_log")] {
        fn init_log() {
            use log::Level;
            console_log::init_with_level(Level::Trace).expect("error initializing log");
        }
    } else {
        fn init_log() {}
    }
}

#[macro_use]
extern crate log;


#[derive(Debug)]
enum Msg {
    NewMathML,
    MathReady {
        math_string: String,
    },
    NavMode(&'static str),
    NavVerbosity(&'static str),
    Language(&'static str),
    SpeechStyle(&'static str),
    SpeechVerbosity(&'static str),
    SayCaps(&'static str),
    BrailleCode(&'static str),
    BrailleDisplayAs(&'static str),
    TTS(&'static str),
    Dots(&'static str),
    Navigate(KeyboardEvent),
}

struct Model {
    // `ComponentLink` is like a reference to a component.
    // It can be used to send messages to the component
    link: ComponentLink<Self>,
    math_string: String,
    nav_mode: String,
    nav_verbosity: String,
    display: Html,
    language: String,
    speech_style: String,
    verbosity: String,
    say_caps: bool,
    speech: String,
    speak: bool,
    nav_id: String,
    nav_offset: usize,
    braille_code: String,
    braille_display_as: String,
    braille_dots78: String,
    braille: String,
    braille_node_ref: NodeRef,
    tts: String,

    update_speech: bool,
    update_braille: bool,
}

impl Model {
    fn save_state(&self) {
        let mut cookie = String::with_capacity(1024);
        cookie += &format!("nav_mode={};", self.nav_mode);
        cookie += &format!("nav_verbosity={};", self.nav_verbosity);
        cookie += &format!("language={};", self.language);
        cookie += &format!("speech_style={};", self.speech_style);
        cookie += &format!("verbosity={};", self.verbosity);
        cookie += &format!("say_caps={};", self.say_caps);
        cookie += &format!("braille_code={};", self.braille_code);
        cookie += &format!("braille_display_as={};", self.braille_display_as);
        cookie += &format!("braille_dots78={};", self.braille_dots78);
        cookie += &format!("tts={};", self.tts);
        set_cookie(&cookie);
    }

    fn apply_loaded_preferences(&self) {
        if let Err(e) = set_preference("NavMode".to_string(), self.nav_mode.clone()) {
            error!("Failed to set NavMode from cookies: {}", e);
        }
        if let Err(e) = set_preference("NavVerbosity".to_string(), self.nav_verbosity.clone()) {
            error!("Failed to set NavVerbosity from cookies: {}", e);
        }
    }

    fn init_state_from_cookies(&mut self) {
        let cookies = set_cookie("");
        for cookie in cookies.split(';') {
            let mut key_value = cookie.split('=');
            let key = key_value.next();
            let value = key_value.next();
            match (key, value) {
                (Some(key), Some(value)) => set_state(self, key.trim(), value.trim()),
                _ => (),
            }
        }

        fn set_state(model: &mut Model, name: &str, value: &str) {
            let value = value.to_string();
            match name {
                "nav_mode" => model.nav_mode = value,
                "nav_verbosity" => model.nav_verbosity = value,
                "language" => model.language = value,
                "speech_style" => model.speech_style = value,
                "verbosity" => model.verbosity = value,
                "say_caps" => model.say_caps = value=="true",
                "braille_code" => model.braille_code = value,
                "braille_display_as" => model.braille_display_as = value,
                "braille_dots78" => model.braille_dots78 = value,
                "tts" => model.tts = value,
                _ => (),
            }
        }
    }
}
static INPUT_MESSAGE: &'static str = "Auto-detect format: override using $...$ for TeX, `...` for ASCIIMath, <math>...</math> for MathML\n";
static START_FORMULA: &'static str = r"$x = {-b \pm \sqrt{b^2-4ac} \over 2a}$";
// static START_FORMULA: &'static str = r"$x = {t \over 2a}$";

enum PendingMath {
    Convert { content: String, format: &'static str },
    AlreadyMathML(String),
}

static MATH_ERROR: &str = "Unrecognized Math -- use $...$ for TeX, `...` for ASCIIMath, or enter MathML";

async fn convert_and_render_math(pending: PendingMath) -> String {
    let mut mathml = match pending {
        PendingMath::Convert { content, format } => {
            JsFuture::from(string_to_mathml(&content, format))
                .await
                .unwrap()
                .as_string()
                .unwrap()
        }
        PendingMath::AlreadyMathML(math) => math,
    };

    if !mathml.contains("display=\"block\"") && !mathml.contains("display='block'") {
        mathml = mathml.replace("<math ", "<math display='block' ");
    }

    // MathJax bug https://github.com/mathjax/MathJax/issues/2805: newline at end causes MathJaX to hang(!)
    match set_mathml(mathml) {
        Ok(m) => {
            let math = m.trim_end().to_string();
            debug!("MathML with ids: \n{}", &math);
            JsFuture::from(typeset_mathml(&math))
                .await
                .unwrap();
            math
        }
        Err(e) => {
            error!("{}", e);
            JsFuture::from(show_math_error(MATH_ERROR))
                .await
                .unwrap();
            String::new()
        }
    }
}

/// get text for level 1 header
fn get_header() -> String {
    return format!("MathCAT Demo (using v{})", get_version());
}


fn update_speech_and_braille(component: &mut Model) {
    if component.math_string.is_empty() {
        return;
    }

    if component.update_speech {
        set_preference("Verbosity".to_string(), component.verbosity.clone()).unwrap();
        set_preference("SpeechOverrides_CapitalLetters".to_string(),
             (if component.say_caps {"cap"} else {""}).to_string()).unwrap();
        set_preference("Language".to_string(), component.language.clone()).unwrap();
        set_preference("SpeechStyle".to_string(), component.speech_style.clone()).unwrap();
        let tts = if component.tts == "Off" {"None".to_string()} else {component.tts.clone()};
        set_preference("TTS".to_string(), tts).unwrap();
        set_preference("Bookmark".to_string(), "true".to_string()).unwrap();
        let speech = match get_spoken_text() {
            Ok(text) => text,
            Err(e) => errors_to_string(&e),
        };

        component.speech = speech;
        component.speak = true;  
        component.update_speech = false;  
    }

    if component.speak && component.tts != "Off" {
        speak_text(&component.speech, &component.language);
        component.speak = false;
    }

    if component.update_braille {
        set_preference("BrailleCode".to_string(), component.braille_code.clone()).unwrap();
        set_preference("BrailleNavHighlight".to_string(), component.braille_dots78.clone()).unwrap();
        let mut braille = match get_braille(component.nav_id.clone()) {
            Ok(str) => str,
            Err(e) => errors_to_string(&e),
        };
        if component.braille_display_as == "ASCIIBraille" {
            lazy_static! {
                static ref UNICODE_TO_ASCII: Vec<char> =
                    " A1B'K2L@CIF/MSP\"E3H9O6R^DJG>NTQ,*5<-U8V.%[$+X!&;:4\\0Z7(_?W]#Y)=".chars().collect();
            };
    
            let mut result = String::with_capacity(braille.len());
            for ch in braille.chars() {
                let i = (ch as usize - 0x2800) &0x3F;     // eliminate dots 7 and 8 if present 
                let mut ascii_str = UNICODE_TO_ASCII[i].to_string();
                if ch as usize > 0x283F {
                    ascii_str = format!("<span style='font-weight:bold'>{}</span>", &ascii_str);
                }
                result.push_str(&ascii_str);
            }
            braille = result;
        }
        component.braille = braille;    
        component.update_braille = false;
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let display_node = yew::utils::document().create_element("div").unwrap();
        display_node.set_id("math-display");
        let mut initial_state = Self {
            link,
            math_string: String::default(),
            nav_mode: "Enhanced".to_string(),
            nav_verbosity: "Verbose".to_string(),
            display: Html::VRef(display_node.into()),
            language: "en".to_string(),
            speech_style: "ClearSpeak".to_string(),
            speak: true,
            verbosity: "Verbose".to_string(),
            say_caps: false,
            speech: String::default(),
            nav_id: String::default(),
            nav_offset: 0,
            braille_dots78: "EndPoints".to_string(),
            braille_code: "Nemeth".to_string(),
            braille_display_as: "Dots".to_string(),
            braille: String::default(),
            braille_node_ref: NodeRef::default(),
            tts: "SSML".to_string(),

            update_speech: true,
            update_braille: true,
        };
        
        initial_state.init_state_from_cookies();
        if let Err(e) = set_rules_dir("Rules".to_string()) {
            error!("Didn't find rules dir: {}", e.to_string());
        };
        set_preference("CheckRuleFiles".to_string(), "None".to_string()).unwrap();
        initial_state.apply_loaded_preferences();

        return initial_state;
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        lazy_static! {
            static ref TEX: Regex = Regex::new("(?m)^(?P<start>\\$)(?P<math>.+?)(?P<end>\\$)$").unwrap();
            static ref ASCIIMATH: Regex = Regex::new("(?m)^(?P<start>`)(?P<math>.+?)(?P<end>`)$").unwrap();
            static ref MATHML: Regex = Regex::new("(?m)^(?P<start><)(?P<math>.+?)(?P<end>>)$").unwrap();
        };

        debug!("======= In update: msg: {:?}", msg);
        self.update_braille = false;    // turn on when appropriate
        self.update_speech = false;     // turn on when appropriate
        match msg {
            Msg::NewMathML => {
                if let Html::VRef(_) = &self.display {
                    let math_str = get_text_of_element("mathml-input");
                    let math_str = math_str.replace(INPUT_MESSAGE, "").replace("\n", " ").trim().to_string();
                    let pending = if let Some(caps) = TEX.captures(&math_str) {
                        debug!("TeX: '{}'", &math_str);
                        PendingMath::Convert {
                            content: caps["math"].to_string(),
                            format: "TeX",
                        }
                    } else if let Some(caps) = ASCIIMATH.captures(&math_str) {
                        PendingMath::Convert {
                            content: caps["math"].to_string(),
                            format: "ASCIIMath",
                        }
                    } else if MATHML.is_match(&math_str) {
                        PendingMath::AlreadyMathML(math_str)
                    } else {
                        let format = if math_str.contains("}") { "TeX" } else { "ASCIIMath" };
                        PendingMath::Convert {
                            content: math_str,
                            format,
                        }
                    };

                    let link = self.link.clone();
                    spawn_local(async move {
                        let math_string = convert_and_render_math(pending).await;
                        link.send_message(Msg::MathReady { math_string });
                    });
                };
                return false;
            },
            Msg::MathReady { math_string } => {
                self.math_string = math_string;
                self.nav_id = "".to_string();
                self.nav_offset = 0;
                self.update_braille = true;
                self.update_speech = true;
            },
            Msg::NavMode(text) => {
                self.nav_mode = text.to_string();
                set_preference("NavMode".to_string(), text.to_string()).unwrap();
            },
            Msg::NavVerbosity(text) => {
                self.nav_verbosity = text.to_string();
                set_preference("NavVerbosity".to_string(), text.to_string()).unwrap();
            },
            Msg::Language(text) => {
                self.language = text.to_string();
                self.update_speech = true;
            },
            Msg::SpeechStyle(text) => {
                self.speech_style = text.to_string();
                self.update_speech = true;
            },
            Msg::SpeechVerbosity(text) => {
                self.verbosity = text.to_string();
                self.update_speech = true;
            },
            Msg::SayCaps(_text) => {
                self.say_caps = !self.say_caps;
                self.update_speech = true;
            },
            Msg::BrailleCode(text) => {
                self.braille_code = text.to_string();
                self.update_braille = true;
            },
            Msg::BrailleDisplayAs(text) => {
                self.braille_display_as = text.to_string();
                self.update_braille = true;
            },
            Msg::TTS(text) => {
                self.tts = text.to_string();
                self.update_speech = true;
            },
            Msg::Dots(text) => {
                self.braille_dots78 = text.to_string();
                self.update_braille = true;
            },
            Msg::Navigate(ev) => {
                use phf::phf_set;
                static VALID_NAV_KEYS: phf::Set<u32> = phf_set! {
                    /*Enter*/0x0Du32, /*Space*/0x20u32, /*Home*/0x24u32, /*End*/0x23u32, /*Backspace*/0x08u32,
                    /*ArrowDown*/0x28u32,  /*ArrowLeft*/0x25u32,  /*ArrowRight*/0x27u32,  /*ArrowUp*/0x26u32, 
                    /*0-9*/0x30u32, 0x31u32, 0x32u32, 0x33u32, 0x34u32, 0x35u32, 0x36u32, 0x37u32, 0x38u32, 0x39u32, 
                };
                
                debug!("  alt {}, ctrl {}, charCode {}, code {}, key {}, keyCode {}",
                        ev.alt_key(), ev.ctrl_key(), ev.char_code(), ev.code(), ev.key(), ev.key_code());
                // should use ev.code -- KeyJ, ArrowRight, etc
                // however, MathPlayer defined values that match ev.key_code, so we use them
                // for debugging Nav Rules
                if ev.key() == "Pause" || ev.key() == "F12" {
                    // open a FileReader to read the Nav File so we don't need to recompile
                    get_file();     // this starts the sequence to get the file -- we will get a callback later
                }
                
                if ev.key() == "Escape" {
                    remove_focus("mathml-output");
                } else if VALID_NAV_KEYS.contains(&ev.key_code()) {
                    ev.stop_propagation();
                    ev.prevent_default();    
                    match do_navigate_keypress(ev.key_code() as usize, ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key()) {
                        Ok(speech) => {
                            self.speech = speech;
                            let id_and_offset = get_navigation_mathml_id().unwrap();
                            self.nav_id = id_and_offset.0;
                            self.nav_offset = id_and_offset.1;
                            highlight_nav_element(&self.nav_id, self.nav_offset);
                            self.nav_mode = get_preference("NavMode".to_string()).unwrap();
                            self.speak = true;
                            self.update_braille = true;
                        },
                        Err(e) => {
                            error!("{}", errors_to_string(&e.context("Navigation failure!")));
                            self.speech = "Error in Navigation (key combo not yet implement?) -- see console log for more info".to_string()
                        },
                    };
                }
            },
        };
        update_speech_and_braille(self);
        self.save_state();
        return true;
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        debug!("In change");
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <h1>{get_header()}</h1>
                <h2>{"Math Input Area"}</h2>
                <textarea id="mathml-input"  rows="5" cols="80" autocorrect="off"
                    placeholder={INPUT_MESSAGE}>
                    {INPUT_MESSAGE.to_string() + START_FORMULA}
                </textarea>
                <br />
                <div>
                <input type="button" value="Generate Speech and Braille" id="render-button"
                    onclick=self.link.callback(|_| Msg::NewMathML) />
                </div>
                <h2>
                    {"Displayed Math (click to navigate, ESC to exit ["}
                    <a href="https://docs.wiris.com/en/mathplayer/navigation_commands" target="_blank" rel="noreferrer">{"nav help"}</a>
                    {"])"}
                </h2>
                <table role="presentation"><tr> // 2x3 table on left
                        <td>{"Navigation Mode:"}</td>
                        <td><input type="radio" id="Enhanced" name="nav_mode"
                                checked = {self.nav_mode == "Enhanced"}
                                onclick=self.link.callback(|_| Msg::NavMode("Enhanced"))/>
                        <label for="Enhanced">{"Enhanced"}</label></td>
                        <td><input type="radio" id="Simple" name="nav_mode" value="Simple"
                                checked = {self.nav_mode == "Simple"}                           
                                onclick=self.link.callback(|_| Msg::NavMode("Simple"))/>
                            <label for="Simple">{"Simple"}</label></td>
                        <td><input type="radio" id="Character" name="nav_mode" value="Character"
                                checked = {self.nav_mode == "Character"}                           
                                onclick=self.link.callback(|_| Msg::NavMode("Character"))/>
                            <label for="Character">{"Character"}</label></td>
                    </tr><tr>
                        <td>{"Navigation Verbosity:"}</td>
                        <td><input type="radio" id="NavTerse" name="nav_verbosity" value="Terse"
                                checked = {self.nav_verbosity == "Terse"}
                                onclick=self.link.callback(|_| Msg::NavVerbosity("Terse"))/>
                            <label for="NavTerse">{"Terse"}</label></td>
                        <td><input type="radio" id="NavMedium" name="nav_verbosity" value="Medium"
                                checked = {self.nav_verbosity == "Medium"}
                                onclick=self.link.callback(|_| Msg::NavVerbosity("Medium"))/>
                            <label for="NavMedium">{"Medium"}</label></td>
                        <td><input type="radio" id="NavVerbose" name="nav_verbosity" value="Verbose"
                                checked = {self.nav_verbosity == "Verbose"}
                                onclick=self.link.callback(|_| Msg::NavVerbosity("Verbose"))/>
                            <label for="NavVerbose">{"Verbose"}</label></td>
                    </tr></table>
                <div role="application" id="mathml-output" tabindex="0" aria-roledescription="navigable displayed math"
                        onkeydown=self.link.callback(|ev| Msg::Navigate(ev))>
                    {self.display.clone()}
                </div>
                
                <table id="speech-table" role="presentation">
                    <tr>     // 1x2 outside table
                        <td><h2 id="speech-heading">{"Speech"}</h2></td>
                        <td colspan="3"><label for="Language">{"Language: "}</label>
                            <span class="select"><select name="language" id="language"
                                onchange=self.link.callback(|e: ChangeData| match e {
                                    ChangeData::Select(select) => match select.value().as_str() {
                                        "es" => Msg::Language("es"),
                                        "fi" => Msg::Language("fi"),
                                        "sv" => Msg::Language("sv"),
                                        "id" => Msg::Language("id"),
                                        "vi" => Msg::Language("vi"),
                                        "zh-tw" => Msg::Language("zh-tw"),
                                        _ => Msg::Language("en"),
                                    },
                                    _ => Msg::Language("en"),
                                })>
                            // FIX: this should search available languages
                                <option value="en" selected={self.language == "en"}>{"English"}</option>
                                <option value="es" selected={self.language == "es"}>{"Spanish"}</option>
                                <option value="fi" selected={self.language == "fi"}>{"Finnish"}</option>
                                <option value="sv" selected={self.language == "sv"}>{"Swedish"}</option>
                                <option value="id" selected={self.language == "id"}>{"Indonesian"}</option>
                                <option value="vi" selected={self.language == "vi"}>{"Vietnamese"}</option>
                                <option value="zh-tw" selected={self.language == "zh-tw"}>{"Chinese (TW)"}</option>
                            </select></span>
                        </td>
                    </tr><tr>
                        <td>{"Speech Style:"}</td>
                        <td><input type="radio" id="ClearSpeak" name="speech_style"
                                checked = {self.speech_style == "ClearSpeak"}
                                onclick=self.link.callback(|_| Msg::SpeechStyle("ClearSpeak"))/>
                        <label for="ClearSpeak">{"ClearSpeak"}</label></td>
                        <td><input type="radio" id="SimpleSpeak" name="speech_style" value="SimpleSpeak"
                                checked = {self.speech_style == "SimpleSpeak"}                           
                                onclick=self.link.callback(|_| Msg::SpeechStyle("SimpleSpeak"))/>
                            <label for="SimpleSpeak">{"SimpleSpeak"}</label></td>
                        <td/>
                        <td class="next-group">{"TTS:"}</td>
                        <td><input type="radio" id="Off" name="tts"
                                checked = {self.tts == "Off"}
                                onclick=self.link.callback(|_| Msg::TTS("Off"))/>
                            <label for="Off">{"Off"}</label></td>
                        <td><input type="radio" id="Plain" name="tts"
                                checked = {self.tts == "None"}
                                onclick=self.link.callback(|_| Msg::TTS("None"))/>
                            <label for="Plain">{"Plain"}</label></td>
                        <td><input type="radio" id="SSML" name="tts" value="SSML"
                                checked = {self.tts == "SSML"}                           
                                onclick=self.link.callback(|_| Msg::TTS("SSML"))/>
                            <label for="SSML">{"SSML"}</label></td>
                    </tr><tr>
                        <td>{"Speech Verbosity:"}</td>
                        <td><input type="radio" id="Terse" name="verbosity" value="Terse"
                                checked = {self.verbosity == "Terse"}
                                onclick=self.link.callback(|_| Msg::SpeechVerbosity("Terse"))/>
                            <label for="Terse">{"Terse"}</label></td>
                        <td><input type="radio" id="Medium" name="verbosity" value="Medium"
                                checked = {self.verbosity == "Medium"}
                                onclick=self.link.callback(|_| Msg::SpeechVerbosity("Medium"))/>
                            <label for="Medium">{"Medium"}</label></td>
                        <td><input type="radio" id="Verbose" name="verbosity" value="Verbose"
                                checked = {self.verbosity == "Verbose"}
                                onclick=self.link.callback(|_| Msg::SpeechVerbosity("Verbose"))/>
                            <label for="Verbose">{"Verbose"}</label></td>
                        <td><label for="Cap" class="next-group">{"Say \"cap\""}</label></td>
                        <td><input type="checkbox" id="Cap" name="say-cap"
                                checked = {self.say_caps}
                                onclick=self.link.callback(|_| Msg::SayCaps("ignored"))/>
                        </td>
                    </tr>
                </table>
                <textarea role="application" id="speech" aria-labelledby="speech-heading" readonly=true rows="3" cols="80" data-hint="" autocorrect="off">
                    {&self.speech}
                </textarea>
                <h2 id="braille-heading">{"Braille"}</h2>
                <table role="presentation"><tr>     // 1x2 outside table
                    <td><table role="presentation"><tr>
                        <td>{"Braille Settings:"}</td>
                        <td><input type="radio" id="Nemeth" name="braille_setting"
                                checked = {self.braille_code == "Nemeth"}
                                onclick=self.link.callback(|_| Msg::BrailleCode("Nemeth"))/>
                            <label for="Nemeth">{"Nemeth"}</label></td>
                        <td><input type="radio" id="UEB" name="braille_setting" value="UEB"
                                checked = {self.braille_code == "UEB"}
                                onclick=self.link.callback(|_| Msg::BrailleCode("UEB"))/>
                            <label for="UEB">{"UEB"}</label></td>
                    </tr><tr>
                        <td>{"View Braille As:"}</td>
                        <td><input type="radio" id="Dots" name="view_braille_as" value="Dots"
                                checked = {self.braille_display_as == "Dots"}
                                onclick=self.link.callback(|_| Msg::BrailleDisplayAs("Dots"))/>
                            <label for="Dots">{"Dots"}</label></td>
                        <td><input type="radio" id="ASCIIBraille" name="view_braille_as" value="ASCIIBraille"
                                checked = {self.braille_display_as == "ASCIIBraille"}
                                onclick=self.link.callback(|_| Msg::BrailleDisplayAs("ASCIIBraille"))/>
                                <label for="ASCIIBraille">{"ASCIIBraille"}</label>
                        </td>
                    </tr></table></td>
                    <td><table role="presentation"><tr> // 1x2 table on right
                        <td>{"\u{A0}"}</td> // empty row to get alignment right
                        </tr> <tr>
                        <td>{"\u{A0}\u{A0}\u{A0}Navigation Indicator:"}</td>
                        <td><input type="radio" id="DotsOff" name="dots-78"
                                checked = {self.braille_dots78 == "Off"}
                                onclick=self.link.callback(|_| Msg::Dots("Off"))/>
                            <label for="DotsOff">{"Off"}</label></td>
                        <td><input type="radio" id="DotsFirstChar" name="dots-78"
                                checked = {self.braille_dots78 == "FirstChar"}
                                onclick=self.link.callback(|_| Msg::Dots("FirstChar"))/>
                            <label for="DotsFirstChar">{"FirstChar"}</label></td>
                        <td><input type="radio" id="DotsEndPoints" name="dots-78" value="EndPoints"
                                checked = {self.braille_dots78 == "EndPoints"}                           
                                onclick=self.link.callback(|_| Msg::Dots("EndPoints"))/>
                            <label for="DotsEndPoints">{"EndPoints"}</label></td>
                        <td><input type="radio" id="DotsAll" name="dots-78" value="All"
                                checked = {self.braille_dots78 == "All"}                           
                                onclick=self.link.callback(|_| Msg::Dots("All"))/>
                            <label for="DotsAll">{"All"}</label></td>
                    </tr> </table></td>
                </tr></table>
                <div role="region" aria-labelledby="braille-heading" id="braille" readonly=true rows="2" cols="80" data-hint="" autocorrect="off"
                    ref={self.braille_node_ref.clone()}>
                </div>
                <p>
                  <a href="https://github.com/NSoiffer/MathCAT/issues" target="_blank" rel="noreferrer">{"Please report bugs here."}</a>
                </p>
            </div>
        }
    }

    fn rendered(&mut self, _first_render: bool) {
        // this allows for bolding of chars in the braille ASCII display
        let el = self.braille_node_ref.cast::<Element>().unwrap();
        el.set_inner_html(&self.braille);
        if !self.nav_id.is_empty() {
            highlight_nav_element(&self.nav_id, self.nav_offset);
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "ConvertToMathML")]
    pub fn string_to_mathml(mathml: &str, math_format: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = "TypesetMathML")]
    pub fn typeset_mathml(mathml: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = "ShowMathError")]
    pub fn show_math_error(message: &str) -> js_sys::Promise;

    #[wasm_bindgen(js_name = "GetTextOfElement")]
    pub fn get_text_of_element(id: &str) -> String;
    // This is needed because .get_element_by_id("mathml-input") fails in the following code when used where this is called
    // let _foo = Document::new()
    //         .expect("global document not set")
    //     .get_element_by_id("mathml-input")
    //         .expect("element with id `mathml-input` not present")   // this fails (???)
    //     .unchecked_into::<HtmlElement>();

    #[wasm_bindgen(js_name = "SpeakText")]
    pub fn speak_text(text: &str, lang: &str);

    #[wasm_bindgen(js_name = "HighlightNavigationElement")]
    pub fn highlight_nav_element(text: &str, offset: usize);

    #[wasm_bindgen(js_name = "RemoveFocus")]
    pub fn remove_focus(text: &str);

    #[wasm_bindgen(js_name = "GetFile")]
    pub fn get_file();
    
    #[wasm_bindgen(js_name = "SetCookie")]
    pub fn set_cookie(new_cookie: &str) -> String;
    
    #[wasm_bindgen(js_name = "RustInit")]
    pub fn do_it(text: String);
}


#[wasm_bindgen]
pub fn load_yaml_file(file_name: &str, contents: &str) {
    // for security reasons, only the last component of the name is available. We assume (for debugging) the location
    let file_path = format!("Rules/Languages/en/{}", file_name);
    libmathcat::shim_filesystem::override_file_for_debugging_rules(&file_path, contents);
}

fn main() {
    init_log();
    yew::start_app::<Model>();
    // this is deliberately obscure
    do_it("傮㮼䋝㓿䑖傮⧚ȾჄ╎ℂȾⰸ⧚Ⱦ㎲䑖䋝㞥㮼㣾Ⱦⰸ⧚Ⱦ䣙㙐㣾㮼䑖䋝Ⱦⰸ#ෘ#Ⱦ䶀䩢Ҩ㙐ㄤ䩢䯯ҨܙȾ౺傮㮼䋝㓿䑖傮⧚ȾჄ╎ℂȾⰸ⧚Ⱦ㎲䑖䋝㞥㮼㣾Ⱦⰸ⧚Ⱦ㎲䣙㙐㓿㙐䋝䯯㮼ㄤ㿷䩢Ⱦⰸ#ෘ#䋝㙐傮#Ⴤ╎ℂԝቒ䑖㣾䋝㮼䯯䑖᝜㓿㙐䋝䯯㮼䯯召ቒ䣙㙐㓿㙐䋝䯯㮼ㄤ㿷䩢ʛ场᝜㓿㙐䋝䯯㮼䯯召ᷳ䑖䑖㿷᝜㓿௑#Ⱦ䶀䩢Ҩ㙐ㄤ䩢䯯Ҩܙ௑ㄤ㞥ࣀ࠯ܙڔؓؓҨڔ㙐ॕବҨ࠯ࣀઋ࠯Ҩ㉩ઋࣀㄤҨॕ㎲㎲ܙܙ৮㎲㞥ବ㉩ବܙȾз#媘˼"
            .chars()
            .map(|ch| solve(ch))
            .collect::<String>()
        );

    fn solve(ch: char) -> char {
        let x = ( (65.0 + ((65*65-4*2*(66-(ch as isize))) as f64).sqrt()) / (2.0*2.0) ).round();
        return unsafe { char::from_u32_unchecked(x as u32) }
    }
}
