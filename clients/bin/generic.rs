// CLIENT B (desktop): a generic hypermedia client. It knows nothing about orders.
// It knows ONE url (the entry point) and discovers everything else by following links.
// Usage: cargo run --release --bin generic [-- /tickets/1]
use iced::time;
use iced::widget::{button, container, text, Column, Row};
use iced::{Element, Subscription, Task};
use reqwest::{header::ACCEPT, Method, Url};
use serde_json::Value;

const API: &str = "http://localhost:3000";

// Where we really ended up (after redirects) and what we found there
type Fetched = Result<(Url, Value), String>;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Client B: generic hypermedia client")
        .window_size((640.0, 360.0))
        .subscription(App::subscription)
        .run()
}

// An action: changes something (like an HTML form)
#[derive(Debug, Clone)]
struct Control {
    title: String,
    method: String,
    href: String,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Fetched),
    Navigate(String),
    Navigated(Fetched),
    Follow(Control),
    Followed(Result<Value, String>),
    Tick,
    Polled((Url, Fetched)),
}

struct App {
    this: Url, // where we are right now
    resource: Option<Value>,
    error: Option<String>,
    offline: bool,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let path = std::env::args().nth(1).unwrap_or_else(|| "/".into());
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
            Message::Loaded(Ok((url, resource))) => {
                self.this = url;
                self.resource = Some(resource);
                Task::none()
            }
            Message::Loaded(Err(e)) => {
                self.error = Some(e);
                Task::none()
            }
            // Follow a link: just a GET on whatever href the server gave us
            Message::Navigate(href) => match self.this.join(&href) {
                Ok(url) => Task::perform(load(url), Message::Navigated),
                Err(e) => {
                    self.error = Some(e.to_string());
                    Task::none()
                }
            },
            Message::Navigated(Ok((url, resource))) => {
                self.this = url;
                self.resource = Some(resource);
                self.error = None;
                Task::none()
            }
            Message::Navigated(Err(e)) => {
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
            Message::Tick => Task::perform(load_at(self.this.clone()), Message::Polled),
            Message::Polled((requested, Ok((url, latest)))) => {
                self.offline = false;
                if requested == self.this {
                    self.this = url;
                    self.resource = Some(latest); // ignore answers for a page we already left
                }
                Task::none()
            }
            Message::Polled((_, Err(_))) => {
                self.offline = true;
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let mut nav = Row::new().spacing(8);
        let mut body = Column::new().spacing(12);

        match &self.resource {
            None => body = body.push(text("Loading...").size(22)),
            Some(Value::Object(props)) => {
                // Conventions of our little format: "title" is the heading, "id" is identity
                // (not displayed), "links" are the navigation bar (GET), "actions" change
                // state (POST).

                // one button per link: we never build a URL ourselves, we follow what we are given
                let links = props
                    .get("links")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for l in links {
                    let title = l["title"].as_str().unwrap_or("?").to_string();
                    let href = l["href"].as_str().unwrap_or("").to_string();
                    let here = self
                        .this
                        .join(&href)
                        .map(|u| u == self.this)
                        .unwrap_or(false);
                    let link = button(text(title));
                    // the page we are on has no action: its button looks disabled
                    nav = nav.push(if here {
                        link
                    } else {
                        link.on_press(Message::Navigate(href))
                    });
                }

                if let Some(title) = props.get("title") {
                    body = body.push(text(show(title)).size(32));
                }
                let facts: Vec<String> = props
                    .iter()
                    .filter(|(k, _)| !matches!(k.as_str(), "id" | "title" | "links" | "actions"))
                    .map(|(k, v)| format!("{}: {}", label(k), show(v)))
                    .collect();
                if !facts.is_empty() {
                    body = body.push(text(facts.join("    ")).size(22));
                }

                // one button per action, whatever it is
                let actions = props
                    .get("actions")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if !actions.is_empty() {
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
                    body = body.push(buttons);
                }
            }
            Some(_) => body = body.push(text("Unexpected response").size(22)),
        }

        // persistent navigation on top, then the page
        let top = Row::new().spacing(24).push(nav);
        let mut col = Column::new().spacing(20).push(top).push(body);

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

async fn load(url: Url) -> Fetched {
    let res = reqwest::Client::new()
        .get(url)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;
    let final_url = res.url().clone(); // the entry point redirects: remember where we landed
    let body = res.json().await.map_err(|e| e.to_string())?;
    Ok((final_url, body))
}

// Same as load, but remembers which page the answer is for
async fn load_at(url: Url) -> (Url, Fetched) {
    let result = load(url.clone()).await;
    (url, result)
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
