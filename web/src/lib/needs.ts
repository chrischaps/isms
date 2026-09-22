// What moves each need meter, for the Meter tooltips. Every sentence is a
// rule in isms-core needs.rs (phase 5, GDD 4.2), told without its numbers,
// which are preset parameters the client does not carry. Where Food comes
// from depends on the society, so the hint reads the capabilities.

type T = (key: string) => string;
type Caps = { money: boolean; order_books: boolean; common_store: boolean };

export type NeedHints = { food: string; shelter: string; comfort: string };

/** What a NeedCard's status line needs to know (docs/style.md §8.4). */
export type NeedState = {
  food: number;
  shelter: number;
  comfort: number;
  hardship: boolean;
  housed: boolean;
  /** Units of Food and Wares in the pantry. */
  pantryFood: number;
  pantryWares: number;
  threshold?: number;
};

/**
 * One short sentence under each meter: what is happening and, when it is
 * changing, which way (§8.4). The rates are preset parameters the client
 * does not carry, so the lines say the direction and the cause, not the
 * number. Shelter without a dwelling is falling with no rule to stop it,
 * which is the bible's own example of a tone override (§4.2).
 */
export function needStatus(t: T, s: NeedState): { food: string; shelter: string; comfort: string; shelterTone?: "attn" } {
  const line = s.threshold ?? 20;
  const pantry = t("pantry").toLowerCase();
  const dwelling = t("dwelling").toLowerCase();
  const food = s.hardship
    ? "In hardship: under the line for a whole day. Eat first."
    : s.food < line
      ? s.pantryFood > 0
        ? `Under the hardship line — eating from your ${pantry} now.`
        : `Under the hardship line and your ${pantry} is empty.`
      : s.pantryFood > 0
        ? s.food >= 99.5
          ? `Full. You eat 1 Food an hour from your ${pantry}.`
          : `Rising — you eat 1 Food an hour from your ${pantry}.`
        : `Falling every hour — your ${pantry} is empty.`;
  const shelter = s.housed
    ? s.shelter >= 99.5
      ? `Full. You have a ${dwelling}.`
      : `Rising — you have a ${dwelling}.`
    : `Falling every hour — you have no ${dwelling}.`;
  const comfort =
    s.pantryWares > 0
      ? s.comfort >= 99.5
        ? `Full. Wares in your ${pantry} keep it there.`
        : `Rising — you use a Wares from your ${pantry} when there is room.`
      : s.comfort >= 99.5
        ? "Full for now. It drifts down slowly without Wares."
        : "Drifting down slowly. Wares would top it up.";
  // The override only lifts a meter the value alone would call good; under 50 the thresholds already say attn or crit.
  return { food, shelter, comfort, shelterTone: !s.housed && s.shelter >= 50 ? "attn" : undefined };
}

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
      `${wares} and keep a few on hand. Comfort does not change what you produce or earn: it is a third of your wellbeing, the number this society is judged by, and the one meter that shows whether people here have more than enough.`,
  };
}
