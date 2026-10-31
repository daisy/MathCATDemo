
use yew::prelude::*;
use yew::web_sys::{Element, HtmlSelectElement};
use wasm_bindgen::JsCast;
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
        auto_speak: bool,
        generation: u64,
    },
    #[cfg(feature = "braille-input")]
    BrailleInput(String),
    #[cfg(feature = "braille-input")]
    BrailleInputCode(String),
    NavMode(&'static str),
    NavVerbosity(&'static str),
    Language(String),
    SpeechStyle(&'static str),
    SpeechVerbosity(&'static str),
    SayCaps(&'static str),
    BrailleCode(String),
    BrailleDisplayAs(&'static str),
    Tts(&'static str),
    Dots(&'static str),
    Navigate(KeyboardEvent),
    /// A keystroke in the math box. Used to restore text that braille input had greyed out.
    MathInputEdited,
}

/// Which input last produced the displayed math, speech, and braille.
#[cfg(feature = "braille-input")]
#[derive(Clone, Copy, PartialEq)]
enum OutputSource {
    None,
    Math,
    Braille,
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
    supported_languages: Vec<String>,
    speech_style: String,
    verbosity: String,
    say_caps: bool,
    speech: String,
    speak: bool,
    auto_speak: bool,
    nav_id: String,
    nav_offset: usize,
    braille_code: String,
    supported_braille_codes: Vec<String>,
    braille_display_as: String,
    braille_dots78: String,
    braille: String,
    braille_node_ref: NodeRef,
    #[cfg(feature = "braille-input")]
    braille_input: String,
    #[cfg(feature = "braille-input")]
    braille_input_code: String,
    #[cfg(feature = "braille-input")]
    braille_error: String,
    #[cfg(feature = "braille-input")]
    braille_input_generation: u64,
    #[cfg(feature = "braille-input")]
    output_source: OutputSource,
    /// Set when the user types in an input whose text is currently grey.
    #[cfg(feature = "braille-input")]
    math_input_revised: bool,
    #[cfg(feature = "braille-input")]
    braille_input_revised: bool,
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
        #[cfg(feature = "braille-input")]
        {
            cookie += &format!("braille_input_code={};", self.braille_input_code);
        }
        cookie += &format!("tts={};", self.tts);
        set_cookie(&cookie);
    }

    #[cfg(feature = "braille-input")]
    fn math_input_class(&self) -> &'static str {
        if self.output_source == OutputSource::Braille &&
           !self.math_input_revised
        {
            "stale-input"
        } else {
            ""
        }
    }

    #[cfg(feature = "braille-input")]
    fn braille_input_class(&self) -> &'static str {
        if self.output_source == OutputSource::Math &&
           !self.braille_input_revised
        {
            "stale-input"
        } else {
            ""
        }
    }

    #[cfg(not(feature = "braille-input"))]
    fn math_input_class(&self) -> &'static str {
        ""
    }

    fn apply_loaded_preferences(&self) {
        // Nav prefs are needed before the first keypress; the rest must match the UI
        // even if the user generates speech/braille without touching a control.
        set_pref("NavMode", &self.nav_mode);
        set_pref("NavVerbosity", &self.nav_verbosity);
        apply_speech_preferences(self);
        set_pref("BrailleCode", &self.braille_code);
        set_pref("BrailleNavHighlight", &self.braille_dots78);
    }

    /// Yew 0.18 sets the `selected` *attribute* on `<option>`, which does not update
    /// the live `<select>` value (same issue as `checked` vs `defaultChecked`).
    fn sync_selects(&self) {
        fn set_selected(id: &str, index: Option<usize>) {
            let Some(index) = index else { return };
            let Some(el) = yew::utils::document().get_element_by_id(id) else { return };
            let Ok(select) = el.dyn_into::<HtmlSelectElement>() else { return };
            let index = index as i32;
            if select.selected_index() != index {
                select.set_selected_index(index);
            }
        }

        set_selected(
            "language",
            self.supported_languages
                .iter()
                .position(|lang| lang == &self.language),
        );
        set_selected(
            "braille_code",
            self.supported_braille_codes
                .iter()
                .position(|code| code == &self.braille_code),
        );
        #[cfg(feature = "braille-input")]
        set_selected(
            "braille-input-code",
            BRAILLE_INPUT_CODES
                .iter()
                .position(|code| *code == self.braille_input_code),
        );
    }

    fn init_state_from_cookies(&mut self) {
        let cookies = set_cookie("");
        for cookie in cookies.split(';') {
            let mut key_value = cookie.split('=');
            let key = key_value.next();
            let value = key_value.next();
            if let (Some(key), Some(value)) = (key, value) {
                set_state(self, key.trim(), value.trim());
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
                #[cfg(feature = "braille-input")]
                "braille_input_code" if BRAILLE_INPUT_CODES.contains(&value.as_str()) => {
                    model.braille_input_code = value;
                }
                "tts" => model.tts = value,
                _ => (),
            }
        }
    }
}
static INPUT_MESSAGE: &str = "Auto-detect format: override using $...$ for TeX, `...` for ASCIIMath, <math>...</math> for MathML\n";
static START_FORMULA: &str = r"$x = {-b \pm \sqrt{b^2-4ac} \over 2a}$";
// static START_FORMULA: &'static str = r"$x = {t \over 2a}$";

/// North American ASCII braille: index is (Unicode braille - 0x2800) & 0x3F.
static UNICODE_TO_ASCII_BRAILLE: &str =
    " A1B'K2L@CIF/MSP\"E3H9O6R^DJG>NTQ,*5<-U8V.%[$+X!&;:4\\0Z7(_?W]#Y)=";

fn uses_computer_braille(code: &str) -> bool {
    code == "ASCIIMath" || code == "LaTeX"
}

#[cfg(feature = "braille-input")]
fn is_braille_cell(c: char) -> bool {
    (0x2800..=0x28FF).contains(&(c as u32))
}

fn set_pref(key: &str, value: impl AsRef<str>) {
    if let Err(e) = set_preference(key, value.as_ref()) {
        error!("Failed to set {}: {}", key, e);
    }
}

fn tts_preference(tts: &str) -> &str {
    if tts == "Off" { "None" } else { tts }
}

fn apply_speech_preferences(model: &Model) {
    set_pref("Verbosity", &model.verbosity);
    set_pref(
        "SpeechOverrides_CapitalLetters",
        if model.say_caps { "cap" } else { "" },
    );
    set_pref("Language", &model.language);
    set_pref("SpeechStyle", &model.speech_style);
    set_pref("TTS", tts_preference(&model.tts));
    set_pref("Bookmark", "true");
}

fn queue_math(link: ComponentLink<Model>, pending: PendingMath, generation: u64) {
    spawn_local(async move {
        let math_string = convert_and_render_math(pending).await;
        link.send_message(Msg::MathReady {
            math_string,
            auto_speak: true,
            generation,
        });
    });
}

/// North American Braille Computer Code. Printable ASCII uses the 6-dot cell from
/// `UNICODE_TO_ASCII_BRAILLE`. Dot 7 selects the uppercase block (@, A-Z, brackets, ^).
/// The same cell without dot 7 is the lowercase block (grave, a-z, braces, |, ~).
/// Underscore has no dot 7. Dot 8 is ignored so both 6-dot and 8-dot cells decode.
#[cfg(feature = "braille-input")]
fn computer_braille_cell_to_char(cell: u32) -> char {
    let index = ((cell - 0x2800) & 0x3F) as usize;
    let table_char = UNICODE_TO_ASCII_BRAILLE.chars().nth(index).unwrap();
    let dot7 = (cell & 0x40) != 0;
    let t = table_char as u32;
    match t {
        0x20..=0x3F => table_char,
        0x5F => {
            if dot7 { '\u{7F}' } else { '_' }
        }
        0x40..=0x5E => {
            if dot7 {
                table_char
            } else {
                char::from_u32(t + 0x20).unwrap()
            }
        }
        _ => table_char,
    }
}

#[cfg(feature = "braille-input")]
fn computer_braille_to_text(input: &str) -> String {
    input.chars().map(|ch| {
        let u = ch as u32;
        if is_braille_cell(ch) {
            computer_braille_cell_to_char(u)
        } else {
            ch
        }
    }).collect()
}

fn char_to_8dot_braille(ch: char) -> char {
    let c = ch as u32;
    let (table_char, dot7) = match c {
        0x20..=0x3F => (ch, false),
        0x40..=0x5E => (ch, true),
        0x5F => ('_', false),
        0x60..=0x7E => (char::from_u32(c - 0x20).unwrap(), false),
        _ => return ch,
    };
    // The table is ASCII, so the byte index is the 6-dot cell.
    let Some(index) = UNICODE_TO_ASCII_BRAILLE.find(table_char) else {
        return ch;
    };
    let mut bits = index as u32;
    if dot7 {
        bits |= 0x40;
    }
    char::from_u32(0x2800 + bits).unwrap()
}

fn ascii_to_8dot_braille(text: &str) -> String {
    text.chars().map(char_to_8dot_braille).collect()
}

fn escape_html(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            _ => result.push(ch),
        }
    }
    result
}

enum PendingMath {
    Convert { content: String, format: &'static str },
    AlreadyMathML(String),
}

static MATH_ERROR: &str = "Unrecognized Math -- use $...$ for TeX, `...` for ASCIIMath, or enter MathML";

#[cfg(feature = "braille-input")]
const BRAILLE_INPUT_CODES: &[&str] = &["UEB", "Nemeth", "ASCIIMath", "LaTeX"];

#[cfg(feature = "braille-input")]
fn is_unicode_braille_text(s: &str) -> bool {
    s.chars().all(|c| is_braille_cell(c) || c.is_whitespace())
}

/// Map North American ASCII braille to Unicode braille cells (U+2800..).
#[cfg(feature = "braille-input")]
fn ascii_braille_to_unicode(ascii: &str) -> String {
    lazy_static! {
        static ref ASCII_TO_UNICODE: std::collections::HashMap<char, char> = {
            let mut map = std::collections::HashMap::with_capacity(128);
            for (i, ch) in UNICODE_TO_ASCII_BRAILLE.chars().enumerate() {
                let unicode = char::from_u32(0x2800 + i as u32).unwrap();
                map.insert(ch, unicode);
                if ch.is_ascii_uppercase() {
                    map.insert(ch.to_ascii_lowercase(), unicode);
                }
            }
            map
        };
    }
    ascii.chars()
        .map(|c| ASCII_TO_UNICODE.get(&c).copied().unwrap_or(c))
        .collect()
}

#[cfg(feature = "braille-input")]
fn ensure_unicode_braille(input: &str) -> String {
    if is_unicode_braille_text(input) {
        input.to_string()
    } else {
        ascii_braille_to_unicode(input)
    }
}

/// A typed space next to Unicode braille is the blank cell, not U+0020.
#[cfg(feature = "braille-input")]
fn spaces_as_braille_blanks(input: &str) -> String {
    let has_braille = input.chars().any(is_braille_cell);
    if has_braille {
        input.replace(' ', "\u{2800}")
    } else {
        input.to_string()
    }
}

/// ASCIIMath and LaTeX accept literal text, or 6-dot / 8-dot computer braille
/// which is converted back to that text. UEB and Nemeth accept Unicode braille
/// or North American ASCII braille.
#[cfg(feature = "braille-input")]
fn braille_input_text(input: &str, code: &str) -> String {
    let input = spaces_as_braille_blanks(input);
    if uses_computer_braille(code) {
        if is_unicode_braille_text(&input) {
            computer_braille_to_text(&input)
        } else {
            input
        }
    } else {
        ensure_unicode_braille(&input)
    }
}

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
            debug!("MathML with ids: \n{}", math);
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
    format!("MathCAT Demo (using v{})", get_version())
}


fn selected_from_list(e: ChangeData, items: &[String], fallback: &str) -> String {
    match e {
        ChangeData::Select(select) => {
            let idx = select.selected_index();
            if idx >= 0 {
                items.get(idx as usize).cloned().unwrap_or_else(|| fallback.to_string())
            } else {
                fallback.to_string()
            }
        }
        _ => fallback.to_string(),
    }
}

fn apply_braille_code(component: &Model) {
    // MathCAT defaults to Nemeth. Keep it aligned with the dropdown even when there is no math yet.
    if !component.braille_code.is_empty() {
        set_pref("BrailleCode", &component.braille_code);
    }
}

fn update_speech_and_braille(component: &mut Model) {
    if component.math_string.is_empty() {
        apply_braille_code(component);
        return;
    }

    if component.update_speech {
        apply_speech_preferences(component);
        let speech = match get_spoken_text() {
            Ok(text) => text,
            Err(e) => errors_to_string(&e),
        };

        component.speech = speech;
        component.speak = component.auto_speak;
        component.auto_speak = true;
        component.update_speech = false;  
    }

    // After speech prefs: MathCAT's default is Nemeth, and Language/SpeechStyle reloads
    // must not leave that default in place while the Braille Code dropdown shows another code.
    apply_braille_code(component);

    if component.speak && component.tts != "Off" {
        speak_text(&component.speech, &component.language);
        component.speak = false;
    }

    if component.update_braille {
        set_pref("BrailleNavHighlight", &component.braille_dots78);
        // The math display already shows an <merror>. Don't braille that message.
        let mut braille = if component.math_string.contains("<merror") {
            String::new()
        } else {
            match get_braille(component.nav_id.clone()) {
                Ok(str) => str,
                Err(e) => {
                    error!("{}", errors_to_string(&e));
                    String::new()
                }
            }
        };
        if uses_computer_braille(&component.braille_code) {
            // These codes come back as ASCII text. Dots shows North American 8-dot computer braille.
            braille = if component.braille_display_as == "Dots" {
                ascii_to_8dot_braille(&braille)
            } else {
                escape_html(&braille)
            };
        } else if component.braille_display_as == "ASCIIBraille" {
            lazy_static! {
                static ref UNICODE_TO_ASCII: Vec<char> =
                    UNICODE_TO_ASCII_BRAILLE.chars().collect();
            };
    
            let mut result = String::with_capacity(braille.len());
            for ch in braille.chars() {
                let i = (ch as usize - 0x2800) &0x3F;     // eliminate dots 7 and 8 if present
                // ASCII Braille uses < > &; this string is assigned via innerHTML.
                let ascii = match UNICODE_TO_ASCII[i] {
                    '&' => "&amp;".to_string(),
                    '<' => "&lt;".to_string(),
                    '>' => "&gt;".to_string(),
                    c => c.to_string(),
                };
                if ch as usize > 0x283F {
                    result.push_str(&format!("<span style='font-weight:bold'>{}</span>", ascii));
                } else {
                    result.push_str(&ascii);
                }
            }
            braille = result;
        }
        component.braille = braille;    
        component.update_braille = false;
    }
}

#[cfg(feature = "braille-input")]
fn convert_braille_input(model: &mut Model) {
    model.braille_input_generation = model.braille_input_generation.wrapping_add(1);
    stop_speech();
    model.speak = false;

    let text = model.braille_input.trim().to_string();
    if text.is_empty() {
        // A deleted input should not leave the previous expression on screen.
        model.output_source = OutputSource::None;
        model.braille_error.clear();
        model.math_string.clear();
        model.speech.clear();
        model.braille.clear();
        model.nav_id.clear();
        model.nav_offset = 0;
        model.speak = false;
        model.update_speech = false;
        model.update_braille = false;
        let _ = set_mathml("<math></math>");
        spawn_local(async {
            let _ = JsFuture::from(show_math_error("")).await;
        });
        return;
    }

    let code = model.braille_input_code.clone();
    model.braille_code = code.clone();
    set_pref("BrailleCode", &code);
    let notation = braille_input_text(&text, &code);
    // LaTeX uses MathCAT's parser so incomplete input (`\frac`, `2^`) stays in
    // the tree. MathJax's TeX converter rejects those.
    let pending = match set_mathml_from_braille(&notation, &code) {
        Ok(mathml) => PendingMath::AlreadyMathML(mathml),
        Err(e) => {
            let message = errors_to_string(&e);
            error!("Braille input error: {}", message);
            model.braille_error = message;
            return;
        }
    };
    model.braille_error.clear();
    let generation = model.braille_input_generation;
    queue_math(model.link.clone(), pending, generation);
}

fn radio(
    link: &ComponentLink<Model>,
    id: &'static str,
    name: &'static str,
    label: &'static str,
    checked: bool,
    msg: impl Fn(MouseEvent) -> Msg + 'static,
) -> Html {
    html! {
        <>
            <input type="radio" id={id} name={name} value={id}
                checked={checked}
                onclick=link.callback(msg) />
            <label for={id}>{label}</label>
        </>
    }
}

fn option_list<'a>(items: impl IntoIterator<Item = &'a str>, current: &str) -> Html {
    let current = current.to_string();
    html! {
        <>
            { for items.into_iter().map(|item| {
                let item = item.to_string();
                let selected = current == item;
                html! {
                    <option key={item.clone()} value={item.clone()} selected={selected}>{item}</option>
                }
            }) }
        </>
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
            supported_languages: Vec::new(),
            speech_style: "ClearSpeak".to_string(),
            speak: true,
            verbosity: "Verbose".to_string(),
            say_caps: false,
            speech: String::default(),
            nav_id: String::default(),
            nav_offset: 0,
            braille_dots78: "EndPoints".to_string(),
            braille_code: "Nemeth".to_string(),
            supported_braille_codes: Vec::new(),
            braille_display_as: "Dots".to_string(),
            braille: String::default(),
            braille_node_ref: NodeRef::default(),
            #[cfg(feature = "braille-input")]
            braille_input: String::default(),
            #[cfg(feature = "braille-input")]
            braille_input_code: "UEB".to_string(),
            #[cfg(feature = "braille-input")]
            braille_error: String::default(),
            #[cfg(feature = "braille-input")]
            braille_input_generation: 0,
            #[cfg(feature = "braille-input")]
            output_source: OutputSource::None,
            #[cfg(feature = "braille-input")]
            math_input_revised: false,
            #[cfg(feature = "braille-input")]
            braille_input_revised: false,
            tts: "SSML".to_string(),

            update_speech: true,
            update_braille: true,
            auto_speak: true,
        };
        
        initial_state.init_state_from_cookies();
        if let Err(e) = set_rules_dir("Rules") {
            error!("Didn't find rules dir: {}", e);
        };
        set_preference("CheckRuleFiles", "None").unwrap();
        match get_supported_languages() {
            Ok(langs) => initial_state.supported_languages = langs,
            Err(e) => error!("Failed to get supported languages: {}", e),
        }
        match get_supported_braille_codes() {
            Ok(codes) => initial_state.supported_braille_codes = codes,
            Err(e) => error!("Failed to get supported braille codes: {}", e),
        }
        initial_state.apply_loaded_preferences();

        initial_state
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
        let mut should_render = true;
        match msg {
            Msg::NewMathML => {
                if let Html::VRef(_) = &self.display {
                    let math_str = get_text_of_element("mathml-input");
                    let math_str = math_str.replace(INPUT_MESSAGE, "").replace("\n", " ").trim().to_string();
                    let pending = if let Some(caps) = TEX.captures(&math_str) {
                        debug!("TeX: '{}'", math_str);
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

                    queue_math(self.link.clone(), pending, 0);
                };
                should_render = false;
            },
            Msg::MathReady { math_string, auto_speak, generation } => {
                #[cfg(feature = "braille-input")]
                if generation != 0 && generation != self.braille_input_generation {
                    should_render = false;
                }
                #[cfg(not(feature = "braille-input"))]
                {
                    let _ = generation;
                }
                if !should_render {
                    // no-op, preserve the stale render suppression only when the feature is enabled
                } else {
                    self.math_string = math_string;
                    self.nav_id = "".to_string();
                    self.nav_offset = 0;
                    self.update_braille = true;
                    self.update_speech = true;
                    self.auto_speak = auto_speak;
                    #[cfg(feature = "braille-input")]
                    {
                        self.output_source = if generation == 0 {
                            OutputSource::Math
                        } else {
                            OutputSource::Braille
                        };
                        self.math_input_revised = false;
                        self.braille_input_revised = false;
                    }
                }
            },
            Msg::MathInputEdited => {
                #[cfg(feature = "braille-input")]
                if self.output_source == OutputSource::Braille &&
                   !self.math_input_revised
                {
                    self.math_input_revised = true;
                    return true;
                }
                return false;
            },
            #[cfg(feature = "braille-input")]
            Msg::BrailleInput(text) => {
                if text == self.braille_input {
                    should_render = false;
                } else {
                    // Typing restores the normal color before this input generates output.
                    self.braille_input_revised = true;
                    self.braille_input = text;
                    convert_braille_input(self);
                }
            },
            #[cfg(feature = "braille-input")]
            Msg::BrailleInputCode(code) => {
                if code == self.braille_input_code {
                    should_render = false;
                } else {
                    self.braille_input_code = code;
                    convert_braille_input(self);
                }
            },
            Msg::NavMode(text) => {
                self.nav_mode = text.to_string();
                set_pref("NavMode", text);
            },
            Msg::NavVerbosity(text) => {
                self.nav_verbosity = text.to_string();
                set_pref("NavVerbosity", text);
            },
            Msg::Language(text) => {
                self.language = text;
                set_pref("Language", &self.language);
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
                self.braille_code = text;
                set_pref("BrailleCode", &self.braille_code);
                self.update_braille = true;
            },
            Msg::BrailleDisplayAs(text) => {
                self.braille_display_as = text.to_string();
                self.update_braille = true;
            },
            Msg::Tts(text) => {
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
                            self.nav_mode = get_preference("NavMode").unwrap();
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
        if should_render {
            update_speech_and_braille(self);
        }
        self.save_state();
        should_render
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        debug!("In change");
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        let languages = self.supported_languages.clone();
        let language = self.language.clone();
        let on_language = self.link.callback(move |e: ChangeData| {
            Msg::Language(selected_from_list(e, &languages, &language))
        });
        let braille_codes = self.supported_braille_codes.clone();
        let braille_code = self.braille_code.clone();
        let on_braille_code = self.link.callback(move |e: ChangeData| {
            Msg::BrailleCode(selected_from_list(e, &braille_codes, &braille_code))
        });

        #[cfg(feature = "braille-input")]
        let braille_input_ui = {
            let codes: Vec<String> = BRAILLE_INPUT_CODES.iter().map(|code| code.to_string()).collect();
            let current = self.braille_input_code.clone();
            let on_braille_input_code = self.link.callback(move |e: ChangeData| {
                Msg::BrailleInputCode(selected_from_list(e, &codes, &current))
            });
            html! {
                <>
                    <div id="braille-input-row">
                        <label id="braille-input-code-label" for="braille-input-code">{"Braille Input: "}</label>
                        <span class="select"><select name="braille_input_code" id="braille-input-code" aria-labelledby="braille-input-code-label"
                            onchange=on_braille_input_code>
                        { option_list(BRAILLE_INPUT_CODES.iter().copied(), &self.braille_input_code) }
                        </select></span>
                        <input type="text" id="braille-input" size="80" autocorrect="off"
                            autocomplete="off"
                            class={self.braille_input_class()}
                            aria-label="Braille input"
                            aria-describedby="braille-error"
                            oninput=self.link.callback(|e: InputData| Msg::BrailleInput(e.value)) />
                    </div>
                    <div id="braille-error-row" aria-live="assertive">
                        <label for="braille-error">{"Braille Error: "}</label>
                        <input type="text" id="braille-error" size="80" readonly=true
                            value={self.braille_error.clone()} />
                    </div>
                </>
            }
        };
        #[cfg(not(feature = "braille-input"))]
        let braille_input_ui = Html::default();

        html! {
            <div>
                <h1>{get_header()}</h1>
                <h2 id="math-input-heading">
                    <label id="math-input-label" for="mathml-input">{"Math Input Area"}</label>
                </h2>
                <textarea id="mathml-input" aria-labelledby="math-input-label" rows="5" cols="80" autocorrect="off"
                    class={self.math_input_class()}
                    oninput=self.link.callback(|_| Msg::MathInputEdited)
                    placeholder={INPUT_MESSAGE}>
                    {INPUT_MESSAGE.to_string() + START_FORMULA}
                </textarea>
                <br />
                <div>
                <input type="button" value="Generate Speech and Braille" id="render-button"
                    onclick=self.link.callback(|_| Msg::NewMathML) />
                </div>
                { braille_input_ui }
                <h2 id="math-display-heading">
                    {"Displayed Math (click to navigate, ESC to exit ["}
                    <a href="https://daisy.github.io/MathCAT/nav-commands.html" target="_blank" rel="noreferrer">{"nav help"}</a>
                    {"])"}
                </h2>
                <table role="presentation"><tr>
                        <td>{"Navigation Mode:"}</td>
                        <td>{ radio(&self.link, "Enhanced", "nav_mode", "Enhanced", self.nav_mode == "Enhanced", |_| Msg::NavMode("Enhanced")) }</td>
                        <td>{ radio(&self.link, "Simple", "nav_mode", "Simple", self.nav_mode == "Simple", |_| Msg::NavMode("Simple")) }</td>
                        <td>{ radio(&self.link, "Character", "nav_mode", "Character", self.nav_mode == "Character", |_| Msg::NavMode("Character")) }</td>
                    </tr><tr>
                        <td>{"Navigation Verbosity:"}</td>
                        <td>{ radio(&self.link, "NavTerse", "nav_verbosity", "Terse", self.nav_verbosity == "Terse", |_| Msg::NavVerbosity("Terse")) }</td>
                        <td>{ radio(&self.link, "NavMedium", "nav_verbosity", "Medium", self.nav_verbosity == "Medium", |_| Msg::NavVerbosity("Medium")) }</td>
                        <td>{ radio(&self.link, "NavVerbose", "nav_verbosity", "Verbose", self.nav_verbosity == "Verbose", |_| Msg::NavVerbosity("Verbose")) }</td>
                    </tr></table>
                <div role="application" id="mathml-output" tabindex="0"
                        aria-labelledby="math-display-heading"
                        aria-roledescription="navigable displayed math"
                        onkeydown=self.link.callback(Msg::Navigate)>
                    {self.display.clone()}
                </div>
                
                <table id="speech-table" role="presentation">
                    <tr>     // 1x2 outside table
                        <td><h2 id="speech-heading"><label id="speech-label" for="speech">{"Speech"}</label></h2></td>
                        <td colspan="3"><label id="language-label" for="language">{"Language: "}</label>
                            <span class="select"><select name="language" id="language" aria-labelledby="language-label"
                                onchange=on_language>
                            { option_list(self.supported_languages.iter().map(String::as_str), &self.language) }
                            </select></span>
                        </td>
                    </tr><tr>
                        <td>{"Speech Style:"}</td>
                        <td>{ radio(&self.link, "ClearSpeak", "speech_style", "ClearSpeak", self.speech_style == "ClearSpeak", |_| Msg::SpeechStyle("ClearSpeak")) }</td>
                        <td>{ radio(&self.link, "SimpleSpeak", "speech_style", "SimpleSpeak", self.speech_style == "SimpleSpeak", |_| Msg::SpeechStyle("SimpleSpeak")) }</td>
                        <td/>
                        <td class="next-group">{"TTS:"}</td>
                        <td>{ radio(&self.link, "Off", "tts", "Off", self.tts == "Off", |_| Msg::Tts("Off")) }</td>
                        <td>{ radio(&self.link, "Plain", "tts", "Plain", self.tts == "None", |_| Msg::Tts("None")) }</td>
                        <td>{ radio(&self.link, "SSML", "tts", "SSML", self.tts == "SSML", |_| Msg::Tts("SSML")) }</td>
                    </tr><tr>
                        <td>{"Speech Verbosity:"}</td>
                        <td>{ radio(&self.link, "Terse", "verbosity", "Terse", self.verbosity == "Terse", |_| Msg::SpeechVerbosity("Terse")) }</td>
                        <td>{ radio(&self.link, "Medium", "verbosity", "Medium", self.verbosity == "Medium", |_| Msg::SpeechVerbosity("Medium")) }</td>
                        <td>{ radio(&self.link, "Verbose", "verbosity", "Verbose", self.verbosity == "Verbose", |_| Msg::SpeechVerbosity("Verbose")) }</td>
                        <td><label for="Cap" class="next-group">{"Say \"cap\""}</label></td>
                        <td><input type="checkbox" id="Cap" name="say-cap"
                                checked = {self.say_caps}
                                onclick=self.link.callback(|_| Msg::SayCaps("ignored"))/>
                        </td>
                    </tr>
                </table>
                <textarea id="speech" aria-labelledby="speech-label" readonly=true rows="3" cols="80" autocorrect="off">
                    {&self.speech}
                </textarea>
                <table id="braille-table" role="presentation">
                    <tr>
                        <td><h2 id="braille-heading">{"Braille"}</h2></td>
                        <td colspan="3"><label id="braille-code-label" for="braille_code">{"Braille Code: "}</label>
                            <span class="select"><select name="braille_code" id="braille_code" aria-labelledby="braille-code-label"
                                onchange=on_braille_code>
                            { option_list(self.supported_braille_codes.iter().map(String::as_str), &self.braille_code) }
                            </select></span>
                        </td>
                    </tr><tr>
                        <td>{"View Braille As:"}</td>
                        <td>{ radio(&self.link, "Dots", "view_braille_as", "Dots", self.braille_display_as == "Dots", |_| Msg::BrailleDisplayAs("Dots")) }</td>
                        <td>{ radio(&self.link, "ASCIIBraille", "view_braille_as", "ASCIIBraille", self.braille_display_as == "ASCIIBraille", |_| Msg::BrailleDisplayAs("ASCIIBraille")) }</td>
                        <td/>
                        <td class="next-group">{"Navigation Indicator:"}</td>
                        <td>{ radio(&self.link, "DotsOff", "dots-78", "Off", self.braille_dots78 == "Off", |_| Msg::Dots("Off")) }</td>
                        <td>{ radio(&self.link, "DotsFirstChar", "dots-78", "FirstChar", self.braille_dots78 == "FirstChar", |_| Msg::Dots("FirstChar")) }</td>
                        <td>{ radio(&self.link, "DotsEndPoints", "dots-78", "EndPoints", self.braille_dots78 == "EndPoints", |_| Msg::Dots("EndPoints")) }</td>
                        <td>{ radio(&self.link, "DotsAll", "dots-78", "All", self.braille_dots78 == "All", |_| Msg::Dots("All")) }</td>
                    </tr>
                </table>
                <div role="textbox" aria-readonly="true" tabindex="0" aria-labelledby="braille-heading" id="braille"
                    ref={self.braille_node_ref.clone()}>
                </div>
                <p>
                  <a href="https://github.com/daisy/MathCAT/issues" target="_blank" rel="noreferrer">{"Please report bugs here."}</a>
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
        self.sync_selects();
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

    #[wasm_bindgen(js_name = "StopSpeech")]
    pub fn stop_speech();

    #[wasm_bindgen(js_name = "HighlightNavigationElement")]
    pub fn highlight_nav_element(text: &str, offset: usize);

    #[wasm_bindgen(js_name = "RemoveFocus")]
    pub fn remove_focus(text: &str);

    #[wasm_bindgen(js_name = "GetFile")]
    pub fn get_file();
    
    #[wasm_bindgen(js_name = "SetCookie")]
    pub fn set_cookie(new_cookie: &str) -> String;
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
}
