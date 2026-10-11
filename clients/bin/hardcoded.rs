// CLIENT A (desktop): the traditional heavy client. Business rules and URLs live in here.
// Usage: cargo run --release --bin hardcoded
use iced::widget::{button, container, text, Column, Row};
use iced::{Element, Task};
use reqwest::header::ACCEPT;
use serde::Deserialize;
use serde_json::Value;

const API: &str = "http://localhost:3000";
const ID: u32 = 1;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Client A: hardcoded")
        .window_size((560.0, 340.0))
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
    match order.status.as_str() {
        "draft" => vec![
            ("pay", "Pay"),
            ("coupon", "Apply 10% coupon"),
            ("cancel", "Cancel"),
        ],
        "paid" => vec![("cancel", "Cancel"), ("ship", "Ship")],
        _ => vec![],
    }
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<Order, String>),
    Act(&'static str),
    Acted(Result<Order, String>),
    Refresh,
    Reset,
    ResetDone(Result<(), String>),
}

struct App {
    order: Option<Order>,
    error: Option<String>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                order: None,
                error: None,
            },
            Task::perform(fetch_order(), Message::Loaded),
        )
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
            Message::Refresh => {
                self.error = None;
                Task::perform(fetch_order(), Message::Loaded)
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
        let toolbar = Row::new()
            .spacing(8)
            .push(button("Refresh").on_press(Message::Refresh))
            .push(button("Reset").on_press(Message::Reset));
        let mut col = Column::new().spacing(14).push(toolbar);

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
