import {
  type CardMapping,
  type Env,
  type StripeCard,
  errorResponse,
  fail,
  isEmail,
  json,
  serializeCard,
  stripeRequest,
} from "../../_lib/common";

interface Context {
  request: Request;
  env: Env;
  params: { id: string };
}

export const onRequestGet = async (context: Context): Promise<Response> => {
  try {
    const id = context.params.id;
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

    const raw = await kv.get(`card:${id}`);
    if (!raw) {
      return fail(404, "Card not found for this demo environment");
    }
    const mapping = JSON.parse(raw) as CardMapping;
    if (mapping.email !== email) {
      return fail(403, "This card belongs to a different cardholder");
    }

    const card = await stripeRequest<StripeCard>(
      context.env,
      `/v1/issuing/cards/${id}?expand[]=number&expand[]=cvc`
    );
    return json({
      ok: true,
      card: serializeCard(card, {
        amountUsd: mapping.amountUsd,
        holderName: mapping.name,
        feeTxHash: mapping.txHash,
      }),
    });
  } catch (error) {
    return errorResponse(error);
  }
};
