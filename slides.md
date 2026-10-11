---
theme: default
title: Hypermedia - let the server drive
class: text-center
transition: slide-left
mdc: true
---

# Hypermedia

Let the server drive

<div class="pt-8 opacity-60">30 minutes · one live demo</div>

<!--
Total: ~4 min intro, ~12 min demo, ~8 min concepts, ~5 min tradeoffs + Q&A.
-->

---

# Does this look familiar?

```js {all|2|3-4}
function render(order) {
  if (order.status === "paid") {
    showCancel()
    showShip()
  }
}
```

<v-click>

Every business rule change means changing the server **and** every client, together.

</v-click>

<!--
Ask the room: who has shipped a backend change that broke a mobile app?
Line: we call it a REST API, but it behaves like a shared library with a network in the middle.
-->

---

# The web already solved this

<v-clicks>

- 1960s: Ted Nelson coins **hypermedia**
- 2000: Roy Fielding's dissertation formalizes the web's architecture as **REST**
- The server tells the client what it can do next, in the response itself
- Browsers have no per-website logic. **Your API clients do.**

</v-clicks>

<!--
Optional one-liner: in 2008 Fielding complained that most "REST APIs" ignore this constraint.
If running long, cut this to one sentence and fold into the next slide.
-->

---

# Richardson Maturity Model

<v-clicks>

- **Level 0**: one endpoint, RPC over HTTP
- **Level 1**: resources, each with its own URL
- **Level 2**: HTTP verbs and status codes
- **Level 3**: hypermedia controls

</v-clicks>

<v-click>

<div class="pt-6 text-xl">Most APIs stop at level 2. <b>Today: level 3.</b></div>

</v-click>

<div class="absolute bottom-6 text-sm opacity-50">Model by Leonard Richardson, popularized by Martin Fowler</div>

---
layout: center
class: text-center
---

# The setup

One server. Three clients: **A**, **B**, **C**.

<div class="pt-6 opacity-80">A and B are native Rust desktop apps, like any heavy client.<br>C runs in a browser tab.<br>I'll only show you the server.</div>

<!--
Two native desktop windows and one browser tab. The server is TypeScript, the clients are Rust: they share nothing but the contract. Call C "the HTML one" for now.
One of the terminal clients was written the traditional way.
Ask: which one do you think survives a rule change?
-->

---

# The server: one function decides

```ts {all|3-6|8-10|12}
function actions(o: Order): Action[] {
  const a = []
  if (o.status === "draft") {
    a.push({ name: "pay", title: "Pay" })
    if (!o.coupon) a.push({ name: "coupon", title: "Apply 10% coupon" })
  }
  if (o.status === "draft" || o.status === "paid") {
    a.push({ name: "cancel", title: "Cancel" })
  }
  if (o.status === "paid") a.push({ name: "ship", title: "Ship" })
  return a.map(x => ({ ...x, href: `/orders/${o.id}/${x.name}` }))
}
```

<!--
Show the raw JSON response from curl or the browser so they see the actions array.
-->

---

# Live demo

<v-clicks>

1. Walk an order through draft, paid, shipped in all three clients
2. Prediction time: who survives the next change?
3. Edit `actions()` live: paid orders can **no longer be cancelled**
4. Reload. Which clients still show Cancel? What happens when you choose it?
5. Bonus: add a new `refund` action, then point client B at `/tickets/1`

</v-clicks>

<!--
Backup: keep a short screen recording in case the live edit fails.
-->

---
layout: center
class: text-center
---

# What just happened?

Let's open the black boxes.

---

# Client A: hardcoded

```rust {all|1-7|9}
// a native desktop app: no browser, no JavaScript, not even the same language
fn menu(order: &Order) -> Vec<(&str, &str)> {
    match order.status.as_str() {
        "draft" => vec![("pay", "Pay"), ("coupon", "Apply 10% coupon"), ("cancel", "Cancel")],
        "paid" => vec![("cancel", "Cancel"), ("ship", "Ship")],
        _ => vec![],
    }
}
client.post(format!("{API}/orders/{ID}/{name}")).send()?;
```

<v-click>

Rules and URLs are **duplicated** here. They go stale when the server changes.
The server's `409` still protects us, but the user sees a broken button.

</v-click>

---

# Client B: generic

```rust {all|1|2-5|7}
let actions = props.get("actions").and_then(Value::as_array); // the controls
for a in actions {
    let title = a["title"].as_str().unwrap_or("?");
    buttons = buttons.push(button(text(title)).on_press(Message::Follow(control)));
}
client.request(method, base.join(&href)?).send().await?;   // follow the link
// start with any URL: /orders/1 ... or /tickets/1
```

<v-clicks>

- ~20 lines, no notion of "paid", "draft" or even "order"
- Renders whatever controls the server sends
- Same client, **different resource**: point it at `/tickets/1` and it just works
- This is the **uniform interface** at work

</v-clicks>

---

# Client C: HTML + htmx

<v-clicks>

- HTML is a hypermedia format, but it has gaps
- Only `<a>` and `<form>` make requests
- Only click and submit trigger them
- Only GET and POST, and every response replaces the whole page

</v-clicks>

<v-click>

**htmx removes those limits.** It's HTML's hypermedia controls, finished.
~14 KB, one script tag, no build step, no client-side state.

</v-click>

---

# One line is the whole mental model

```html
<button hx-post="/orders/1/pay" hx-target="#order" hx-swap="outerHTML">
  Pay
</button>
```

<v-click>

"When clicked, POST here, and replace `#order` with the HTML that comes back."

</v-click>

<v-click>

<div class="pt-4">Same backend, different format: hypermedia is a property of the <b>design</b>, not of HTML.</div>

</v-click>

<!--
Keep htmx syntax under ~90 seconds. The subject is hypermedia, not htmx.
Pre-empt "isn't this jQuery/PHP again?": the server sends state-appropriate controls, not just content.
Related: Hotwire/Turbo, Unpoly.
-->

---

# The concepts, mapped to what you saw

| Concept | In the demo |
|---|---|
| Hypermedia controls | the `actions` array / htmx buttons |
| HATEOAS | server drives state; client knew one URL |
| Uniform interface | one generic client, orders and tickets |
| Format independence | HTML and JSON from one backend; no browser needed |

---

# What the client still has to know

<v-clicks>

- Hypermedia **moves** coupling, it doesn't remove it
- The client no longer knows *when* an action is valid
- It still needs to know what an action **means**
- Best fit: human-facing UIs, changing workflows, clients you can't update

</v-clicks>

---

# The costs

<v-clicks>

- Larger payloads: controls in every response
- Thinner tooling than OpenAPI: fewer generators and client libraries
- Fewer people know it: hiring and onboarding effort
- Little payoff for machine clients with fixed behavior
- Format choice (HAL, Siren, JSON:API, custom) is real work

</v-clicks>

---
layout: center
---

# Takeaways

<v-clicks>

1. Ask **who decides what's possible next?** Let the server answer.
2. You don't need a framework. Add an `actions` list to one resource.
3. htmx is already a hypermedia client.

</v-clicks>

---
layout: center
class: text-center
---

# Questions?

<!--
Prep:
- GraphQL/OpenAPI? Those describe a fixed contract. Hypermedia describes next valid steps at runtime.
- Standard format needed? No, but a standard gives you reusable clients.
- Versioning? Reduces it for workflow changes, not for data shape changes.
- Why didn't it take over? Tooling, the SPA era favoring JSON contracts, and "REST" being redefined as level 2.
-->
