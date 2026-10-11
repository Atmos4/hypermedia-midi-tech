// CLIENT A (desktop): the traditional heavy client. Business rules and URLs live in here.
// Usage: cargo run --release --bin hardcoded
use iced::keyboard::Modifiers;
use iced::widget::{button, container, text, Column, Row};
use iced::{event, keyboard, time, Event};
use iced::{Element, Subscription, Task};
use reqwest::header::ACCEPT;
use serde::Deserialize;
use serde_json::Value;

const API: &str = "http://localhost:3000";
const ID: u32 = 1;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Client A: hardcoded")
        .window_size((560.0, 340.0))
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone, Deserialize)]
struct Order {
    id: u32,
    status: String,
    total: String,
}

// Rules duplicated from the server. These go stale when the server changes.
fn menu(order: &Order) -> Vec<(&'static str, &'static str)> {
    match (order.status.as_str(), order.total.contains("100")) {
        ("draft", true) => vec![
            ("pay", "Pay"),
            ("coupon", "Apply 10% coupon"),
            ("cancel", "Cancel"),
        ],
        ("draft", false) => vec![("pay", "Pay"), ("cancel", "Cancel")],
        ("paid", _) => vec![("cancel", "Cancel"), ("ship", "Ship")],
        _ => vec![],
    }
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<Order, String>),
    Act(&'static str),
    Acted(Result<Order, String>),
    Reset,
    ResetDone(Result<(), String>),
    Tick,
    Polled(Result<Order, String>),
}

struct App {
    order: Option<Order>,
    error: Option<String>,
    offline: bool,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                order: None,
                error: None,
                offline: false,
            },
            Task::perform(fetch_order(), Message::Loaded),
        )
    }

    // Poll the server once a second, so the window follows server changes by itself
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(time::milliseconds(1000)).map(|_| Message::Tick),
            event::listen_with(|event, _status, _window_id| {
                match event {
                    // Match whenever a key is pressed down
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        key: keyboard::Key::Character(c),
                        modifiers,
                        ..
                    }) => {
                        // Check if Ctrl + Shift are both held down
                        if c.as_str() == "z" && modifiers.contains(Modifiers::CTRL) {
                            Some(Message::Reset)
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(Ok(order)) => {
                self.order = Some(order);
                Task::none()
            }
            Message::Loaded(Err(e)) => {
                self.error = Some(e);
                Task::none()
            }
            Message::Act(name) => Task::perform(act(name), Message::Acted),
            Message::Acted(Ok(order)) => {
                self.order = Some(order);
                self.error = None;
                Task::none()
            }
            // Server refused: show why, then reload to see the real state
            Message::Acted(Err(e)) => {
                self.error = Some(e);
                Task::perform(fetch_order(), Message::Loaded)
            }
            Message::Tick => Task::perform(fetch_order(), Message::Polled),
            Message::Polled(Ok(latest)) => {
                self.order = Some(latest);
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
                Task::perform(fetch_order(), Message::Loaded)
            }
            Message::ResetDone(Err(e)) => {
                self.error = Some(e);
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // demo toolbar
        let mut col = Column::new().spacing(14);

        match &self.order {
            None => col = col.push(text("Loading...").size(22)),
            Some(order) => {
                col = col.push(text(format!("Order #{}", order.id)).size(32));
                col = col.push(
                    text(format!(
                        "Status: {}    Total: {}",
                        order.status, order.total
                    ))
                    .size(22),
                );
                let mut buttons = Row::new().spacing(8);
                for (name, title) in menu(order) {
                    buttons = buttons.push(button(title).on_press(Message::Act(name)));
                }
                col = col.push(buttons);
            }
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

async fn fetch_order() -> Result<Order, String> {
    let res = reqwest::Client::new()
        .get(format!("{API}/orders/{ID}"))
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    res.json::<Order>().await.map_err(|e| e.to_string())
}

async fn act(name: &'static str) -> Result<Order, String> {
    let res = reqwest::Client::new()
        .post(format!("{API}/orders/{ID}/{name}"))
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if res.status().is_success() {
        res.json::<Order>().await.map_err(|e| e.to_string())
    } else {
        let body: Value = res.json().await.map_err(|e| e.to_string())?;
        Err(body["error"]
            .as_str()
            .unwrap_or("request failed")
            .to_string())
    }
}
