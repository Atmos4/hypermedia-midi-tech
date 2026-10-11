// CLIENT B (desktop): a generic hypermedia client. It knows nothing about orders or tickets.
// Usage: cargo run --release --bin generic [-- /tickets/1]
use iced::time;
use iced::widget::{button, container, text, Column, Row};
use iced::{Element, Subscription, Task};
use reqwest::{header::ACCEPT, Method, Url};
use serde_json::Value;

const API: &str = "http://localhost:3000";

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Client B: generic hypermedia client")
        .window_size((560.0, 340.0))
        .subscription(App::subscription)
        .run()
}

// A hypermedia control: all the client needs to know to follow it
#[derive(Debug, Clone)]
struct Control {
    title: String,
    method: String,
    href: String,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<Value, String>),
    Follow(Control),
    Followed(Result<Value, String>),
    Refresh,
    Reset,
    ResetDone(Result<(), String>),
    Tick,
    Polled(Result<Value, String>),
}

struct App {
    this: Url,
    resource: Option<Value>,
    error: Option<String>,
    offline: bool,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let path = std::env::args()
            .nth(1)
            .unwrap_or_else(|| "/orders/1".into());
        let this = Url::parse(API).unwrap().join(&path).expect("invalid path");
        let task = Task::perform(load(this.clone()), Message::Loaded);
        (
            Self {
                this,
                resource: None,
                error: None,
                offline: false,
            },
            task,
        )
    }

    // Poll the server once a second, so the window follows server changes by itself
    fn subscription(&self) -> Subscription<Message> {
        time::every(time::milliseconds(1000)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(Ok(resource)) => {
                self.resource = Some(resource);
                Task::none()
            }
            Message::Loaded(Err(e)) => {
                self.error = Some(e);
                Task::none()
            }
            Message::Follow(control) => {
                Task::perform(follow(self.this.clone(), control), Message::Followed)
            }
            Message::Followed(Ok(resource)) => {
                self.resource = Some(resource);
                self.error = None;
                Task::none()
            }
            Message::Followed(Err(e)) => {
                self.error = Some(e);
                Task::perform(load(self.this.clone()), Message::Loaded)
            }
            Message::Refresh => {
                self.error = None;
                Task::perform(load(self.this.clone()), Message::Loaded)
            }
            Message::Tick => Task::perform(load(self.this.clone()), Message::Polled),
            Message::Polled(Ok(latest)) => {
                self.resource = Some(latest);
                self.offline = false;
                Task::none()
            }
            Message::Polled(Err(_)) => {
                self.offline = true;
                Task::none()
            }
            Message::Reset => Task::perform(reset(), Message::ResetDone),
            Message::ResetDone(Ok(())) => {
                self.error = None;
                Task::perform(load(self.this.clone()), Message::Loaded)
            }
            Message::ResetDone(Err(e)) => {
                self.error = Some(e);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // demo toolbar
        let toolbar = Row::new()
            .spacing(8)
            .push(button("Refresh").on_press(Message::Refresh))
            .push(button("Reset").on_press(Message::Reset));
        let mut col = Column::new().spacing(10).push(toolbar);

        match &self.resource {
            None => col = col.push(text("Loading...").size(22)),
            Some(Value::Object(props)) => {
                // Conventions of our little format: "title" is the heading,
                // "id" is identity (not displayed), "actions" are the controls.
                if let Some(title) = props.get("title") {
                    col = col.push(text(show(title)).size(32));
                }
                let facts: Vec<String> = props
                    .iter()
                    .filter(|(k, _)| !matches!(k.as_str(), "id" | "title" | "actions"))
                    .map(|(k, v)| format!("{}: {}", label(k), show(v)))
                    .collect();
                col = col.push(text(facts.join("    ")).size(22));

                // one button per control, whatever it is
                let actions = props
                    .get("actions")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let mut buttons = Row::new().spacing(8);
                for a in actions {
                    let control = Control {
                        title: a["title"].as_str().unwrap_or("?").to_string(),
                        method: a["method"].as_str().unwrap_or("POST").to_string(),
                        href: a["href"].as_str().unwrap_or("").to_string(),
                    };
                    buttons = buttons.push(
                        button(text(control.title.clone())).on_press(Message::Follow(control)),
                    );
                }
                col = col.push(buttons);
            }
            Some(_) => col = col.push(text("Unexpected response").size(22)),
        }

        if self.offline {
            col = col.push(
                text("Server unreachable, retrying...")
                    .size(20)
                    .color([0.8, 0.5, 0.0]),
            );
        }

        if let Some(error) = &self.error {
            col = col.push(text(format!("! {error}")).size(20).color([0.8, 0.1, 0.1]));
        }

        container(col).padding(24).into()
    }
}

fn label(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn show(v: &Value) -> String {
    match v {
        Value::Null => "-".to_string(),
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

// Demo helper, not part of the hypermedia API: put the server back to its initial state
async fn reset() -> Result<(), String> {
    reqwest::Client::new()
        .post(format!("{API}/reset"))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn load(url: Url) -> Result<Value, String> {
    let res = reqwest::Client::new()
        .get(url)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    res.json().await.map_err(|e| e.to_string())
}

async fn follow(base: Url, control: Control) -> Result<Value, String> {
    let href = base.join(&control.href).map_err(|e| e.to_string())?;
    let method = Method::from_bytes(control.method.as_bytes()).map_err(|e| e.to_string())?;
    let res = reqwest::Client::new()
        .request(method, href)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if res.status().is_success() {
        res.json().await.map_err(|e| e.to_string())
    } else {
        let body: Value = res.json().await.map_err(|e| e.to_string())?;
        Err(body["error"]
            .as_str()
            .unwrap_or("request failed")
            .to_string())
    }
}
