// Hypermedia order demo on Bun + TypeScript.
// Run:  bun --hot server.ts
// --hot reloads this file in place, and the state survives reloads.

type Status = "draft" | "paid" | "shipped" | "cancelled";
type ActionName = "pay" | "coupon" | "cancel" | "ship" | "reorder";

interface Order {
  id: number;
  status: Status;
  total: number;
  baseTotal: number; // price before any coupon (not exposed)
  coupon: boolean;
}

interface Link {
  title: string;
  href: string;
}

type ActionItem = {
  name: ActionName;
  title: string;
  enabled: boolean;
}

function action_list(o: Order): ActionItem[] {
  return [{
    name: "pay", title: "Pay",
    enabled: o.status === "draft"
  }, {
    name: "coupon", title: "Apply 10% coupon",
    enabled: o.status === "draft" && !o.coupon
  }, {
    name: "ship", title: "Ship",
    enabled: o.status === "paid"
  }, {
    name: "cancel", title: "Cancel",
    enabled: o.status === "draft"
  }, {
    name: "reorder", title: "Re-order",
    enabled: o.status === "shipped" || o.status === "cancelled"
  }];
}

function link_list(orders: Record<number, Order>): Link[] {
  let list = Object.values(orders).map((o) => ({
    title: `Order #${o.id}`,
    href: `/orders/${o.id}`,
  }));

  list.push({ title: "Tickets", href: "/tickets" })

  return list;
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
const seedOrders = (): Record<number, Order> => ({
  1: { id: 1, status: "draft", total: 100, baseTotal: 100, coupon: false },
  2: { id: 2, status: "paid", total: 250, baseTotal: 250, coupon: false },
  3: { id: 3, status: "shipped", total: 80, baseTotal: 80, coupon: false },
});
const orders = (g.orders ??= seedOrders());

type TicketAction = "assign" | "close" | "reopen";
interface Ticket {
  id: number;
  title: string;
  state: "open" | "assigned" | "closed";
  assignee: string | null;
}

const seedTickets = (): Record<number, Ticket> => ({
  1: { id: 1, title: "Printer is on fire", state: "open", assignee: null },
  2: { id: 2, title: "Water leakage", state: "open", assignee: null },
  3: { id: 3, title: "Server is down", state: "open", assignee: null },
});
const tickets = (g.tickets ??= seedTickets());

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

function actions(o: Order): Action[] {
  return action_list(o)
    .filter(x => x.enabled)
    .map((x) => ({ ...x, method: "POST", href: `/orders/${o.id}/${x.name}` }));
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

const ticketLinks = (): Link[] => [{ title: "All tickets", href: "/tickets" }];
const orderLinks = (): Link[] =>
  Object.values(orders).map((o) => ({
    title: `Order #${o.id}`,
    href: `/orders/${o.id}`,
  }));


const ticketListLinks = (): Link[] => {
  let list = Object.values(tickets).map((t) => ({
    title: `${t.title} · ${t.state}`,
    href: `/tickets/${t.id}`,
  }));
  list.push({ title: "Back to orders", href: "/orders/1" });
  return list;
}

// ---- JSON representations -------------------------------------

const orderJson = (o: Order) => ({
  id: o.id,
  title: `Order #${o.id}`,
  status: o.status,
  total: `$${o.total}`,
  links: link_list(orders),
  actions: actions(o), // the hypermedia controls
});
const orderSimpleJson = (o: Order) => ({
  id: o.id,
  title: `Order #${o.id}`,
  status: o.status,
  total: `$${o.total}`,
})
const ticketJson = (t: Ticket) => ({ ...t, links: ticketLinks(), actions: ticketActions(t) });

// ---- HTML (htmx) representations ------------------------------

const hash = (s: string) => Bun.hash(s).toString(36);

const orderInner = (o: Order) => `
  <h2>Order #${o.id}</h2>
  <p>Status: <span style="word-spacing:1.5rem">${o.status} </span> Total: $${o.total.toFixed(2)}</p>
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
  html{font-family:system-ui;background-color:#2b2d31;color:white;margin:.5rem;font-size:1.3em}
  h2{font-weight:400}
  header{display:flex;gap:2rem;align-items:center;margin-bottom:1.5rem}
  nav a,nav span{font-size:.9rem;margin-right:.5rem;color:white;background-color:#5865f2;padding:.4rem .6rem;text-decoration:none}
  nav span{opacity:.7}
  button{font-size:.9rem;color:white;background-color:#5865f2;margin-right:.5rem;padding:.4rem .6rem;border:none}
  hr{margin-top:.5rem;margin-bottom:1rem;opacity:.3}
</style>
<script src="https://unpkg.com/htmx.org@2"></script>
${view}
<hr>
<nav>${orderLinks()
    .map((l) => (l.href === path ? `<span>${l.title}</span>` : `<a href="${l.href}">${l.title}</a>`))
    .join("")}</nav>
`;

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

    "/api/orders/:id": {
      GET: (req) => {
        const o = orders[Number(req.params.id)];
        return o ? Response.json(orderSimpleJson(o)) : new Response("Not found", { status: 404 });
      },
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
