import {
  type CardMapping,
  type Env,
  type StripeCard,
  errorResponse,
  fail,
  findOrCreateCardholder,
  isEmail,
  json,
  serializeCard,
  stripeRequest,
} from "../_lib/common";

// Demo-only issuance for the "Quick Buy" TRY preview.
//
// There is no live TRY-capable Stellar anchor yet (see docs/uat.md), so the TRY
// transfer leg of the quick flow is simulated in the UI. This endpoint issues a
// real Stripe **test-mode** virtual card so the download path is genuine, marks
// it as a demo in Stripe metadata and KV, and rate-limits abuse. It never
// touches the on-chain fee vault; the standard /api/card flow remains the only
// path that requires an on-chain fee.

interface QuickBody {
  amountUsd?: number;
  name?: string;
  email?: string;
  reference?: string;
}

interface Context {
  request: Request;
  env: Env;
}

const EMAIL_DAILY_LIMIT = 5;
const IP_DAILY_LIMIT = 12;

export const onRequestPost = async (context: Context): Promise<Response> => {
  try {
    const body = (await context.request.json().catch(() => ({}))) as QuickBody;
    const amountUsd = Number(body.amountUsd);
    const amountCents = Math.round(amountUsd * 100);
    const name = (body.name ?? "").trim() || "StellarCard Demo User";
    const email = (body.email ?? "").trim().toLowerCase();
    const reference = (body.reference ?? "").trim().slice(0, 32);

    if (!Number.isFinite(amountUsd) || amountCents < 500 || amountCents > 50_000) {
      return fail(400, "amountUsd must be between 5 and 500");
    }
    if (!isEmail(email)) {
      return fail(400, "a valid email is required to issue the card");
    }

    const kv = context.env.STELLAR_CARD_KV;
    if (!kv) {
      return fail(500, "STELLAR_CARD_KV binding is not configured");
    }

    const day = new Date().toISOString().slice(0, 10);
    const ip = context.request.headers.get("CF-Connecting-IP") ?? "unknown";
    const emailKey = `quick:email:${email}:${day}`;
    const ipKey = `quick:ip:${ip}:${day}`;
    const [emailCount, ipCount] = await Promise.all([
      kv.get(emailKey),
      kv.get(ipKey),
    ]);
    if (Number(emailCount ?? 0) >= EMAIL_DAILY_LIMIT) {
      return fail(429, "Daily demo-card limit reached for this email");
    }
    if (Number(ipCount ?? 0) >= IP_DAILY_LIMIT) {
      return fail(429, "Daily demo-card limit reached for this network");
    }
    await Promise.all([
      kv.put(emailKey, String(Number(emailCount ?? 0) + 1), {
        expirationTtl: 86_400,
      }),
      kv.put(ipKey, String(Number(ipCount ?? 0) + 1), {
        expirationTtl: 86_400,
      }),
    ]);

    const cardholder = await findOrCreateCardholder(context.env, {
      name,
      email,
      ip,
    });
    const created = await stripeRequest<StripeCard>(
      context.env,
      "/v1/issuing/cards",
      {
        method: "POST",
        body: new URLSearchParams({
          cardholder: cardholder.id,
          currency: "usd",
          type: "virtual",
          status: "active",
          "metadata[demo]": "quick-buy-try",
          "metadata[reference]": reference || "none",
        }),
      }
    );
    const card = await stripeRequest<StripeCard>(
      context.env,
      `/v1/issuing/cards/${created.id}?expand[]=number&expand[]=cvc`
    );

    const txHash = `demo-try:${reference || card.id}`;
    const mapping: CardMapping = {
      email,
      payer: "quick-buy-demo",
      txHash,
      amountUsd,
      name,
      demo: true,
    };
    await kv.put(`card:${card.id}`, JSON.stringify(mapping), {
      expirationTtl: 90 * 24 * 60 * 60,
    });

    return json({
      ok: true,
      demo: true,
      card: serializeCard(card, {
        amountUsd,
        holderName: name,
        feeTxHash: txHash,
        demo: true,
      }),
    });
  } catch (error) {
    return errorResponse(error);
  }
};
