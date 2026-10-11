import type { ActionItem, Order } from "./types";

export function action_list(o: Order): ActionItem[] {
  return [
    {
      name: "pay",
      title: "Pay",
      enabled: o.status === "draft"
    },
    {
      name: "coupon",
      title: "Apply 10% coupon",
      enabled: o.status === "draft" && !o.coupon
    },
    {
      name: "ship",
      title: "Ship",
      enabled: o.status === "paid"
    },
    {
      name: "cancel",
      title: "Cancel",
      enabled: o.status === "draft" || o.status === "paid"
    },
  ];
}
