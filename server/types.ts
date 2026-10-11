export type Status = "draft" | "paid" | "shipped" | "cancelled";
export type ActionName = "pay" | "coupon" | "cancel" | "ship";

export interface Order {
  id: number;
  status: Status;
  total: number;
  coupon: boolean;
}

export type ActionItem = {
  name: ActionName;
  title: string;
  enabled: boolean;
}
