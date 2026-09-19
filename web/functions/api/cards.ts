import {
  type CardMapping,
  type Env,
  type StripeCard,
  type StripeCardholder,
  errorResponse,
  fail,
  isEmail,
  json,
  stripeRequest,
} from "../_lib/common";

interface Context {
  request: Request;
  env: Env;
}

export const onRequestGet = async (context: Context): Promise<Response> => {
  try {
    const email = (
      new URL(context.request.url).searchParams.get("email") ?? ""
    )
      .trim()
      .toLowerCase();

    if (!isEmail(email)) {
      return fail(400, "a valid email is required");
    }
    const kv = context.env.STELLAR_CARD_KV;
    if (!kv) {
      return fail(500, "STELLAR_CARD_KV binding is not configured");
    }

    const listed = await stripeRequest<{ data?: StripeCardholder[] }>(
      context.env,
      "/v1/issuing/cardholders?limit=100"
    );
    const cardholder = (listed.data ?? []).find(
      (item) => (item.email ?? "").toLowerCase() === email
    );
    if (!cardholder) {
      return json({ ok: true, cards: [] });
    }

    const cards = await stripeRequest<{ data?: StripeCard[] }>(
      context.env,
      `/v1/issuing/cards?cardholder=${cardholder.id}&limit=20`
    );

    const result = [];
    for (const card of cards.data ?? []) {
      const raw = await kv.get(`card:${card.id}`);
      const mapping = raw ? (JSON.parse(raw) as CardMapping) : null;
      result.push({
        id: card.id,
        last4: card.last4,
        exp: `${String(card.exp_month).padStart(2, "0")}/${String(
          card.exp_year
        ).slice(-2)}`,
        status: card.status,
        brand: card.brand ?? "visa",
        created: card.created ?? 0,
        amountUsd: mapping?.amountUsd ?? null,
        feeTxHash: mapping?.txHash ?? null,
      });
    }
    return json({ ok: true, cards: result });
  } catch (error) {
    return errorResponse(error);
  }
};
