// Hypermedia order demo on Bun + TypeScript.
// Run:  bun --hot server.ts
// --hot reloads this file in place, and the order state survives reloads.
// ---- Types ----------------------------------------------------

type Status = "draft" | "paid" | "shipped" | "cancelled";
type ActionName = "pay" | "coupon" | "cancel" | "ship";

interface Order {
  id: number;
  status: Status;
  total: number;
  coupon: boolean;
}

interface Action<N extends string = ActionName> {
  name: N;
  title: string;
  method: "POST";
  href: string;
}

// ---- State (kept on globalThis so --hot reloads don't reset it) --

const g = globalThis as typeof globalThis & {
  orders?: Record<number, Order>;
  tickets?: Record<number, Ticket>;
};
const defaultOrder: Order = { id: 1, status: "draft", total: 100, coupon: false };
const seedOrders = (): Record<number, Order> => ({
  1: defaultOrder,
});

function resetOrder(o: Order) {
  o.id = 1;
  o.status = "draft";
  o.total = 100;
  o.coupon = false;
}


// ---- A second resource: proves the generic client is resource-agnostic ----

type TicketAction = "assign" | "close" | "reopen";
interface Ticket {
  id: number;
  title: string;
  state: "open" | "assigned" | "closed";
  assignee: string | null;
}

const seedTickets = (): Record<number, Ticket> => ({
  1: { id: 1, title: "Printer is on fire", state: "open", assignee: null },
});

let orders = (g.orders ??= seedOrders());
let tickets = (g.tickets ??= seedTickets());

// Demo helper (not part of the hypermedia API): back to the initial state
function reset() {
  Object.assign(orders, seedOrders());
  Object.assign(tickets, seedTickets());
}

function ticketActions(t: Ticket): Action<TicketAction>[] {
  const a: { name: TicketAction; title: string }[] = [];
  if (t.state === "open") a.push({ name: "assign", title: "Assign to Alice" });
  if (t.state === "assigned") a.push({ name: "close", title: "Close ticket" });
  if (t.state === "closed") a.push({ name: "reopen", title: "Reopen" });
  return a.map((x) => ({ ...x, method: "POST", href: `/tickets/${t.id}/${x.name}` }));
}

const ticketTransitions: Record<TicketAction, (t: Ticket) => void> = {
  assign: (t) => { t.state = "assigned"; t.assignee = "Alice"; },
  close: (t) => { t.state = "closed"; },
  reopen: (t) => { t.state = "open"; t.assignee = null; },
};

// ---------------------------------------------------------------
// THE DEMO MOMENT: all business rules about "what can I do now?"
// live here. Edit this function and save: every client updates
// without being touched.
// ---------------------------------------------------------------
function actions(o: Order): Action[] {
  const a: { name: ActionName; title: string }[] = [];
  if (o.status === "draft") {
    a.push({ name: "pay", title: "Pay" });
    if (!o.coupon) a.push({ name: "coupon", title: "Apply 10% coupon" });
  }
  // Try changing this live: remove "paid" so paid orders can't be cancelled
  if (o.status === "draft" || o.status === "paid") {
    a.push({ name: "cancel", title: "Cancel" });
  }
  if (o.status === "paid") a.push({ name: "ship", title: "Ship" });

  return a.map((x) => ({ ...x, method: "POST", href: `/orders/${o.id}/${x.name}` }));
}

// State transitions, keyed by action name (the compiler checks every action has one)
const transitions: Record<ActionName, (o: Order) => void> = {
  pay: (o) => { o.status = "paid"; },
  coupon: (o) => { o.coupon = true; o.total *= 0.9; },
  cancel: (o) => { o.status = "cancelled"; },
  ship: (o) => { o.status = "shipped"; },
};

// ---- Representations ------------------------------------------

const asJson = (o: Order) => ({
  id: o.id,
  title: `Order #${o.id}`,
  status: o.status,
  total: `$${o.total}`,
  actions: actions(o), // the hypermedia controls
});

const inner = (o: Order) => `
  <h2>Order #${o.id}</h2>
  <p>Status: <b>${o.status}</b> | Total: $${o.total.toFixed(2)}</p>
  ${actions(o)
    .map((a) => `<button hx-post="${a.href}" hx-target="#order" hx-swap="outerHTML">${a.title}</button>`)
    .join("\n  ")}
`;

// Changes whenever what the user sees changes: the state OR the rules
const version = (o: Order) => Bun.hash(inner(o)).toString(36);

// The fragment polls itself every second. Sending ?v= lets the server answer
// 204 (no change) so htmx leaves the page, and any click in progress, alone.
const fragment = (o: Order) =>
  `<div id="order" hx-get="/orders/${o.id}?v=${version(o)}" hx-trigger="every 1s" hx-swap="outerHTML">${inner(o)}</div>`;

const page = (o: Order) => `<!doctype html>
<title>Order ${o.id}</title>
<style>body{font-family:system-ui;margin:2rem}button{margin-right:.5rem;padding:.4rem .8rem}</style>
<script src="https://unpkg.com/htmx.org@2"></script>
<div style="margin-bottom:1.5rem">
  <button hx-post="/reset" hx-target="#order" hx-swap="outerHTML">Reset</button>
</div>
${fragment(o)}`;

const html = (body: string) =>
  new Response(body, { headers: { "Content-Type": "text/html; charset=utf-8" } });

function respond(req: Request, o: Order): Response {
  if (req.headers.get("accept")?.includes("application/json")) {
    return Response.json(asJson(o));
  }
  return html(req.headers.has("hx-request") ? fragment(o) : page(o));
}

// ---- Server ---------------------------------------------------

Bun.serve({
  port: 3000,

  routes: {
    "/": () => Response.redirect("/orders/1", 302),

    // Demo helper, not part of the hypermedia API
    "/reset": {
      POST: (req) => {
        if (!orders[1]) {
          return new Response("Not found", { status: 404 });
        }
        resetOrder(orders[1]);
        return req.headers.has("hx-request")
          ? html(fragment(orders[1]))
          : new Response(null, { status: 204 });
      },
    },

    "/tickets/:id": {
      GET: (req) => {
        const t = tickets[Number(req.params.id)];
        return t
          ? Response.json({ ...t, actions: ticketActions(t) })
          : new Response("Not found", { status: 404 });
      },
    },

    "/tickets/:id/:action": {
      POST: (req) => {
        const t = tickets[Number(req.params.id)];
        if (!t) return new Response("Not found", { status: 404 });
        const offered = ticketActions(t).find((a) => a.name === req.params.action);
        if (!offered) {
          return Response.json(
            { error: `'${req.params.action}' not allowed in state '${t.state}'` },
            { status: 409 },
          );
        }
        ticketTransitions[offered.name](t);
        return Response.json({ ...t, actions: ticketActions(t) });
      },
    },

    "/orders/:id": {
      GET: (req) => {
        const o = orders[Number(req.params.id)];
        if (!o) return new Response("Not found", { status: 404 });
        // htmx polling: the page already has this version, nothing to swap
        const seen = new URL(req.url).searchParams.get("v");
        if (req.headers.has("hx-request") && seen === version(o)) {
          return new Response(null, { status: 204 });
        }
        return respond(req, o);
      },
    },

    // One generic endpoint: the server only runs actions it currently offers
    "/orders/:id/:action": {
      POST: (req) => {
        const o = orders[Number(req.params.id)];
        if (!o) return new Response("Not found", { status: 404 });

        const offered = actions(o).find((a) => a.name === req.params.action);
        if (!offered) {
          return Response.json(
            { error: `'${req.params.action}' not allowed in state '${o.status}'` },
            { status: 409 },
          );
        }
        transitions[offered.name](o);
        return respond(req, o);
      },
    },
  },

  fetch: () => new Response("Not found", { status: 404 }),
});

console.log("Listening on http://localhost:3000");
