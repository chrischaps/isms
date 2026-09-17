// What moves each need meter, for the Meter tooltips. Every sentence is a
// rule in isms-core needs.rs (phase 5, GDD 4.2), told without its numbers,
// which are preset parameters the client does not carry. Where Food comes
// from depends on the society, so the hint reads the capabilities.

type T = (key: string) => string;
type Caps = { money: boolean; order_books: boolean; common_store: boolean };

export type NeedHints = { food: string; shelter: string; comfort: string };

export function needHints(t: T, caps: Caps | undefined): NeedHints {
  const plan = t("plan");
  const pantry = t("pantry").toLowerCase();
  const source = !caps
    ? `Keep Food in your ${pantry}.`
    : caps.order_books
      ? `Buy Food on the ${t("store")} screen, or set "keep Food at least" on your ${plan} and it bids for you every hour.`
      : caps.common_store
        ? `Your ${plan} draws Food from the common store: raise "keep Food at least" there.`
        : `Keep Food in your ${pantry}; your ${plan} can hold a stock for you.`;
  const wares = !caps || caps.order_books ? `Buy Wares on the ${t("store")} screen` : "Get Wares";
  return {
    food:
      `Each hour you eat one Food from your ${pantry} if there is any, and the meter rises; it falls every hour, faster the harder you work. ` +
      `${source} When Food runs low you produce less; a whole day under the marked line is hardship.`,
    shelter:
      `Rises every hour you have a ${t("dwelling").toLowerCase()} and falls every hour you do not. ` +
      "Rent or buy one on the Contracts screen. Without one you also produce less, and Comfort drains faster.",
    comfort:
      `Each hour you use up one Wares from your ${pantry}, when the meter has room for it, and the meter rises; it falls a little every hour, faster without a ${t("dwelling").toLowerCase()}. ` +
      `${wares} and keep a few on hand. Comfort does not change what you produce.`,
  };
}
