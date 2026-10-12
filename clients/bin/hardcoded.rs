// CLIENT A (desktop): the traditional heavy client. Business rules, URL layout and
// the shape of every response all live in here.
// Usage: cargo run --release --bin hardcoded
use iced::time;
use iced::widget::{button, container, text, Column, Row};
use iced::{Element, Subscription, Task};
use reqwest::header::ACCEPT;
use serde::Deserialize;
use serde_json::Value;

const API: &str = "http://localhost:3000";

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Client A: hardcoded")
        .window_size((640.0, 360.0))
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
        ("shipped", _) => vec![("reorder", "Re-order")],
        ("cancelled", _) => vec![("reorder", "Re-order")],
        _ => vec![],
    }
}

// Everything the screen shows: the list of orders (for the navigation) + the current order
#[derive(Debug, Clone)]
struct Snapshot {
    orders: Vec<Order>,
    current: Order,
}

#[derive(Debug, Clone)]
enum Message {
    Loaded(Result<Snapshot, String>),
    Go(u32),
    Went(Result<Snapshot, String>),
    Act(&'static str),
    Acted(Result<Snapshot, String>),
    Tick,
    Polled((Option<u32>, Result<Snapshot, String>)),
}

struct App {
    snapshot: Option<Snapshot>,
    error: Option<String>,
    offline: bool,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                snapshot: None,
                error: None,
                offline: false,
            },
            Task::perform(fetch(None), Message::Loaded),
        )
    }

    fn current_id(&self) -> Option<u32> {
        self.snapshot.as_ref().map(|s| s.current.id)
    }

    // Poll the server once a second, so the window follows server changes by itself
    fn subscription(&self) -> Subscription<Message> {
        time::every(time::milliseconds(1000)).map(|_| Message::Tick)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(Ok(snapshot)) => {
                self.snapshot = Some(snapshot);
                Task::none()
            }
            Message::Loaded(Err(e)) => {
                self.error = Some(e);
                Task::none()
            }
            // Navigation: this client decides where to go and builds the URL itself
            Message::Go(id) => Task::perform(fetch(Some(id)), Message::Went),
            Message::Went(Ok(snapshot)) => {
                self.snapshot = Some(snapshot);
                self.error = None;
                Task::none()
            }
            Message::Went(Err(e)) => {
                self.error = Some(e);
                Task::none()
            }
            Message::Act(name) => match self.current_id() {
                Some(id) => Task::perform(act(id, name), Message::Acted),
                None => Task::none(),
            },
            Message::Acted(Ok(snapshot)) => {
                self.snapshot = Some(snapshot);
                self.error = None;
                Task::none()
            }
            // Server refused: show why, then reload to see the real state
            Message::Acted(Err(e)) => {
                self.error = Some(e);
                Task::perform(fetch(self.current_id()), Message::Loaded)
            }
            Message::Tick => Task::perform(fetch_at(self.current_id()), Message::Polled),
            Message::Polled((requested, Ok(latest))) => {
                self.offline = false;
                if requested == self.current_id() {
                    self.snapshot = Some(latest); // ignore answers for an order we already left
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

        match &self.snapshot {
            None => body = body.push(text("Loading...").size(22)),
            Some(snapshot) => {
                // navigation built from the ids of a plain data endpoint
                for o in &snapshot.orders {
                    let link = button(text(format!("Order #{}", o.id)));
                    // the page we are on has no action: its button looks disabled
                    nav = nav.push(if o.id == snapshot.current.id {
                        link
                    } else {
                        link.on_press(Message::Go(o.id))
                    });
                }

                let order = &snapshot.current;
                body = body.push(text(format!("Order #{}", order.id)).size(32));
                body = body.push(
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
                body = body.push(buttons);
            }
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

async fn get(url: String) -> Result<reqwest::Response, String> {
    reqwest::Client::new()
        .get(url)
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())
}

// `id` = the order to show, or None for "the first one"
async fn fetch(id: Option<u32>) -> Result<Snapshot, String> {
    // a plain data endpoint: [{ id, status, total }, ...]
    let orders = get(format!("{API}/api/orders"))
        .await?
        .json::<Vec<Order>>()
        .await
        .map_err(|e| e.to_string())?;
    let id = id
        .or_else(|| orders.first().map(|o| o.id))
        .ok_or_else(|| "No orders".to_string())?;
    // hardcoded URL template
    let current = get(format!("{API}/orders/{id}"))
        .await?
        .json::<Order>()
        .await
        .map_err(|e| e.to_string())?;
    Ok(Snapshot { orders, current })
}

// Same as fetch, but remembers which order the answer is for
async fn fetch_at(id: Option<u32>) -> (Option<u32>, Result<Snapshot, String>) {
    (id, fetch(id).await)
}

async fn act(id: u32, name: &'static str) -> Result<Snapshot, String> {
    let res = reqwest::Client::new()
        .post(format!("{API}/orders/{id}/{name}")) // hardcoded URL template
        .header(ACCEPT, "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        let body: Value = res.json().await.map_err(|e| e.to_string())?;
        return Err(body["error"]
            .as_str()
            .unwrap_or("request failed")
            .to_string());
    }
    fetch(Some(id)).await
}
