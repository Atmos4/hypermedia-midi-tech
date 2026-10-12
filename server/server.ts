// Hypermedia order demo on Bun + TypeScript.
// Run:  bun --hot server.ts
// --hot reloads this file in place, and the state survives reloads.

// ---- Types ----------------------------------------------------

type Status = "draft" | "paid" | "shipped" | "cancelled";
type ActionName = "pay" | "coupon" | "cancel" | "ship" | "reorder";

interface Order {
  id: number;
  status: Status;
  total: number;
  baseTotal: number; // price before any coupon (not exposed)
  coupon: boolean;
}

// An action changes something (like an HTML form): POST
interface Action<N extends string = ActionName> {
  name: N;
  title: string;
  method: "POST";
  href: string;
}

// A link goes somewhere (like an HTML <a>): GET
interface Link {
  rel: string;
  title: string;
  href: string;
}

// ---- State (kept on globalThis so --hot reloads don't reset it) --

const g = globalThis as typeof globalThis & {
  orders?: Record<number, Order>;
  tickets?: Record<number, Ticket>;
};
const seedOrders = (): Record<number, Order> => ({
  1: { id: 1, status: "draft", total: 100, baseTotal: 100, coupon: false },
  2: { id: 2, status: "paid", total: 250, baseTotal: 250, coupon: false },
  3: { id: 3, status: "shipped", total: 80, baseTotal: 80, coupon: false },
});
const orders = (g.orders ??= seedOrders());

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
const tickets = (g.tickets ??= seedTickets());

// Demo helper (not part of the hypermedia API): back to the initial state
function reset() {
  for (const k of Object.keys(orders)) delete orders[Number(k)];
  Object.assign(orders, seedOrders());
  for (const k of Object.keys(tickets)) delete tickets[Number(k)];
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
  if (o.status === "shipped" || o.status === "cancelled") {
    a.push({ name: "reorder", title: "Re-order" });
  }

  return a.map((x) => ({ ...x, method: "POST", href: `/orders/${o.id}/${x.name}` }));
}

// State transitions, keyed by action name (the compiler checks every action has one)
const transitions: Record<ActionName, (o: Order) => void> = {
  pay: (o) => { o.status = "paid"; },
  coupon: (o) => { o.coupon = true; o.total = o.baseTotal * 0.9; },
  cancel: (o) => { o.status = "cancelled"; },
  ship: (o) => { o.status = "shipped"; },
  // back to the very first state
  reorder: (o) => { o.status = "draft"; o.coupon = false; o.total = o.baseTotal; },
};

// ---- Links: where can I go from here? --------------------------
// Clients never build these URLs themselves, they follow what they are given.

// The navigation bar: every order carries links to all orders, clients just draw them
const orderLinks = (): Link[] =>
  Object.values(orders).map((o) => ({
    rel: "item",
    title: `Order #${o.id}`,
    href: `/orders/${o.id}`,
  }));
const ticketLinks = (): Link[] => [{ rel: "collection", title: "All tickets", href: "/tickets" }];
const ticketListLinks = (): Link[] =>
  Object.values(tickets).map((t) => ({
    rel: "item",
    title: `${t.title} · ${t.state}`,
    href: `/tickets/${t.id}`,
  }));

// ---- JSON representations -------------------------------------

const orderJson = (o: Order) => ({
  id: o.id,
  title: `Order #${o.id}`,
  status: o.status,
  total: `$${o.total}`,
  links: orderLinks(),
  actions: actions(o), // the hypermedia controls
});
const ticketJson = (t: Ticket) => ({ ...t, links: ticketLinks(), actions: ticketActions(t) });

// ---- HTML (htmx) representations ------------------------------
// The same links, rendered as plain <a> tags.

const hash = (s: string) => Bun.hash(s).toString(36);

const orderInner = (o: Order) => `
  <h2>Order #${o.id}</h2>
  <p>Status: <b>${o.status}</b> | Total: $${o.total.toFixed(2)}</p>
  ${actions(o)
    .map((a) => `<button hx-post="${a.href}" hx-target="#view" hx-swap="outerHTML">${a.title}</button>`)
    .join("\n  ")}
`;

// Every view polls itself every second. Sending ?v= (a hash of what is on screen)
// lets the server answer 204 when nothing changed, so htmx leaves the page,
// and any click in progress, alone. The hash covers state AND rules.
const pollWrap = (path: string, inner: string) =>
  `<div id="view" hx-get="${path}?v=${hash(inner)}" hx-trigger="every 1s" hx-swap="outerHTML">${inner}</div>`;

const shell = (title: string, path: string, view: string) => `<!doctype html>
<title>${title}</title>
<style>
  body{font-family:system-ui;background-color:#2b2d31;color:white;margin:2rem}
  header{display:flex;gap:2rem;align-items:center;margin-bottom:1.5rem}
  nav a,nav span{margin-right:.5rem;color:white;background-color:#5865f2;padding:.4rem .6rem;text-decoration:none}
  nav span{opacity:.7}
  button{color:white;background-color:#5865f2;margin-right:.5rem;padding:.4rem .6rem;border:none}
</style>
<script src="https://unpkg.com/htmx.org@2"></script>
<header>
  <nav>${orderLinks()
    .map((l) => (l.href === path ? `<span>${l.title}</span>` : `<a href="${l.href}">${l.title}</a>`))
    .join("")}</nav>
</header>
${view}`;

const html = (body: string) =>
  new Response(body, { headers: { "Content-Type": "text/html; charset=utf-8" } });

const wantsJson = (req: Request) => req.headers.get("accept")?.includes("application/json") ?? false;

// One resource, two formats: JSON for programs, HTML for browsers
function reply(
  req: Request,
  v: { path: string; title: string; json: unknown; inner: string; poll?: boolean },
): Response {
  if (wantsJson(req)) return Response.json(v.json);
  const htmx = req.headers.has("hx-request");
  if (v.poll && htmx && new URL(req.url).searchParams.get("v") === hash(v.inner)) {
    return new Response(null, { status: 204 });
  }
  const view = pollWrap(v.path, v.inner);
  return html(htmx ? view : shell(v.title, v.path, view));
}

const orderReply = (req: Request, o: Order, poll = false) =>
  reply(req, { path: `/orders/${o.id}`, title: `Order ${o.id}`, json: orderJson(o), inner: orderInner(o), poll });

// ---- Server ---------------------------------------------------

Bun.serve({
  port: 3000,

  routes: {
    // The entry point: the only URL a generic client needs to know.
    // The server decides where "home" is.
    "/": (req) =>
      Response.redirect(new URL(`/orders/${Object.keys(orders)[0]}`, req.url).href, 302),

    // Demo helper, not part of the hypermedia API. Views refresh themselves by polling.
    "/reset": {
      POST: () => {
        reset();
        return new Response(null, { status: 204 });
      },
    },

    "/tickets": {
      GET: () => Response.json({ title: "Tickets", links: ticketListLinks() }),
    },

    "/tickets/:id": {
      GET: (req) => {
        const t = tickets[Number(req.params.id)];
        return t ? Response.json(ticketJson(t)) : new Response("Not found", { status: 404 });
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
        return Response.json(ticketJson(t));
      },
    },

    // A plain endpoint with no links and no actions, just data (used by the hardcoded client)
    "/api/orders": {
      GET: () =>
        Response.json(Object.values(orders).map(({ id, status, total }) => ({ id, status, total: `$${total}` }))),
    },

    "/orders/:id": {
      GET: (req) => {
        const o = orders[Number(req.params.id)];
        return o ? orderReply(req, o, true) : new Response("Not found", { status: 404 });
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
        return orderReply(req, o);
      },
    },
  },

  fetch: () => new Response("Not found", { status: 404 }),
});

console.log("Listening on http://localhost:3000");
