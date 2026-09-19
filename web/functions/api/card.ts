import {
  type CardMapping,
  type Env,
  type SerializedCard,
  type StripeCard,
  errorResponse,
  fail,
  findOrCreateCardholder,
  isEmail,
  isGAddress,
  isTxHash,
  json,
  serializeCard,
  stripeRequest,
  verifyFeePayment,
} from "../_lib/common";

interface IssueBody {
  payer?: string;
  txHash?: string;
  amountUsd?: number;
  name?: string;
  email?: string;
}

interface Context {
  request: Request;
  env: Env;
}

const REPLAY_TTL_SECONDS = 90 * 24 * 60 * 60;

export const onRequestPost = async (context: Context): Promise<Response> => {
  try {
    const body = (await context.request.json().catch(() => ({}))) as IssueBody;
    const payer = (body.payer ?? "").trim();
    const txHash = (body.txHash ?? "").trim().toLowerCase();
    const amountUsd = Number(body.amountUsd);
    const amountCents = Math.round(amountUsd * 100);
    const name = (body.name ?? "").trim() || "StellarCard Demo User";
    const email = (body.email ?? "").trim().toLowerCase();

    if (!isGAddress(payer)) {
      return fail(400, "payer must be a Stellar G-address");
    }
    if (!isTxHash(txHash)) {
      return fail(400, "txHash must be a Stellar transaction hash");
    }
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

    const replayKey = `fee:${txHash}`;
    if (await kv.get(replayKey)) {
      return fail(409, "This fee transaction was already used to issue a card");
    }

    const fee = await verifyFeePayment(context.env, {
      txHash,
      payer,
      amountCents,
    });

    await kv.put(replayKey, JSON.stringify({ state: "pending", payer }), {
      expirationTtl: 600,
    });

    try {
      const cardholder = await findOrCreateCardholder(context.env, {
        name,
        email,
        ip: context.request.headers.get("CF-Connecting-IP") ?? "127.0.0.1",
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
          }),
        }
      );
      const card = await stripeRequest<StripeCard>(
        context.env,
        `/v1/issuing/cards/${created.id}?expand[]=number&expand[]=cvc`
      );

      const mapping: CardMapping = { email, payer, txHash, amountUsd, name };
      await kv.put(`card:${card.id}`, JSON.stringify(mapping), {
        expirationTtl: REPLAY_TTL_SECONDS,
      });
      await kv.put(
        replayKey,
        JSON.stringify({ state: "issued", cardId: card.id, payer }),
        { expirationTtl: REPLAY_TTL_SECONDS }
      );

      const serialized: SerializedCard = serializeCard(card, {
        amountUsd,
        holderName: name,
        feeTxHash: txHash,
      });
      return json({ ok: true, card: serialized, fee });
    } catch (error) {
      await kv.delete(replayKey);
      throw error;
    }
  } catch (error) {
    return errorResponse(error);
  }
};
