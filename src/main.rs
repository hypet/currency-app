use druid::im::Vector;
use druid::lens::Identity;
use druid::widget::{Container, Flex, List, Scroll};
use druid::{AppDelegate, AppLauncher, Command, DelegateCtx, Env, ExtEventSink, Handled, LensExt, Selector, Target, UnitPoint, WidgetExt, WindowDesc};
use druid::{widget::Label, Data, Lens, Widget};
use reqwest::blocking::Response;
use serde_json::Value;
use std::thread;
use std::time::Duration;

const UPDATE_PERIOD_SECONDS: u64 = 60;
const GUI_TEXT_SIZE: f64 = 16.0;
const BID_FIELD: &str = "bid";
const ASK_FIELD: &str = "ask";
const SET_CURRENCIES: Selector<Vector<Currency>> = Selector::new("set_currencies");

struct Delegate;
impl AppDelegate<AppState> for Delegate {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        app_state: &mut AppState,
        _env: &Env,
    ) -> Handled {
        if let Some(&list) = cmd.get(SET_CURRENCIES).as_ref() {
            app_state.currency_list = list.clone();
            return Handled::Yes;
        } 
        Handled::Yes
    }
}

#[derive(Clone, Data, Lens)]
struct AppState {
    currency_list: Vector<Currency>,
}

#[derive(Debug, Clone, Data, Lens)]
struct Currency {
    base: String,
    target: String,
    bid: f32,
    ask: f32,
}

impl Currency {
    fn new(base: &str, target: &str) -> Self {
        Currency { base: base.to_owned(), target: target.to_owned(), bid: 0.0, ask: 0.0 }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            currency_list: Vector::from(
                vec![
                    Currency::new("EUR", "USD"),
                    Currency::new("ETH", "USD"),
                    Currency::new("BTC", "USD"),
                ]
            ),
        }
    }
}

fn build_gui() -> impl Widget<AppState> {
    let col = Flex::column()
        .with_child(
            Flex::row()
                .with_child(Label::new("").fix_width(80.0))
                .with_child(Label::new("ASK").fix_width(60.0))
                .with_child(Label::new("BID").fix_width(60.0))
        )
        .with_child(
            Scroll::new(List::new(|| build_currency_item()).fix_width(200.0))
            .vertical()
            .lens(Identity.map(
                |d: &AppState| {
                    let v: Vector<Currency> = d.currency_list.clone();
                    v
                },
                |_, _| {},
            )),
        );

    Container::new(col)
        .align_vertical(UnitPoint::TOP)
        .center()
}

fn build_currency_item() -> impl Widget<Currency> {
    Flex::row()
        .with_child(
            Label::dynamic(|data: &String, _| data.clone())
                .with_text_size(GUI_TEXT_SIZE)
                .lens(Identity.map(
                    |c: &Currency| format!("{}/{}", c.base, c.target),
                    |_, _| {},
                ))
                .fix_width(80.0)
        )
        .with_child(
            Label::dynamic(|data: &String, _| data.clone())
                .with_text_size(GUI_TEXT_SIZE)
                .lens(Identity.map(
                    |c: &Currency| format!("{}", c.ask),
                    |_, _| {},
                ))
                .fix_width(60.0)
        )
        .with_child(
            Label::dynamic(|data: &String, _| data.clone())
                .with_text_size(GUI_TEXT_SIZE)
                .lens(Identity.map(
                    |c: &Currency| format!("{}", c.bid),
                    |_, _| {},
                ))
                .fix_width(60.0)
        )
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_state = AppState::default();

    let window = WindowDesc::new(build_gui())
        .title("Currency Rates")
        .window_size((240.0, 170.0));

    let launcher = AppLauncher::with_window(window);
    let event_sink = launcher.get_external_handle();

    let currency_list = app_state.currency_list.clone();
    let handle = thread::spawn(
        move || {
            loop {
                call_api(&currency_list, event_sink.clone());
                thread::sleep(Duration::from_secs(UPDATE_PERIOD_SECONDS));
            }
        }
    );

    launcher
        .delegate(Delegate)
        .launch(app_state)
        .expect("Failed to launch application");

    handle.join().unwrap();
    Ok(())
}

fn call_api(currency_list: &Vector<Currency>, sink: ExtEventSink) {
    let pair_param = pair_param(currency_list);
    let url = format!("https://economia.awesomeapi.com.br/last/{}", pair_param);
    let response: Response = reqwest::blocking::get(&url).unwrap();
    match response.status() {
        reqwest::StatusCode::OK => {
            let body = response.text().unwrap();
            let resp: Value = serde_json::from_str(&body).unwrap();
            let currencies: Vector<Currency> = currency_list.iter().map(|c| {
                let pair_name: String = format!("{}{}", c.base, c.target);

                let ask: f32 = resp[&pair_name][ASK_FIELD].as_str().unwrap().parse().unwrap();
                let bid: f32 = resp[&pair_name][BID_FIELD].as_str().unwrap().parse().unwrap();
                Currency { base: c.base.clone(), target: c.target.clone(), bid, ask }
            }).collect();
            sink.submit_command(SET_CURRENCIES, currencies, Target::Auto)
                .expect("command failed to submit");
        },
        _ => {
            eprintln!("Unexpected error");
        },
    };
}

fn pair_param(currency_list: &Vector<Currency>) -> String {
    currency_list.into_iter()
        .map(|c| format!("{base}-{target}", base = c.base, target = c.target))
        .collect::<Vec<String>>()
        .join(",")
}
