import { test, expect, type Page } from "@playwright/test";

/**
 * Cross-browser panic=unwind contract tests.
 *
 * Node counterpart: `test/panic.test.ts` — see its header for the full
 * rationale. Browser coverage matters extra here: the unwind build emits
 * legacy Wasm exception-handling opcodes, whose support and behavior is
 * the most engine-dependent part of this package. A panic must surface as
 * a catchable `Error` with `name === "PanicError"`, and the same Wasm
 * instance must keep working afterward.
 *
 * The panic hook exists only in the `/debug` package variant, so this spec
 * imports `dist/esm/debug-web.js` directly instead of using the release
 * entry that `e2e/server/index.html` loads.
 */

const SERVER_URL = "/e2e/server/index.html";
const DEBUG_ENTRY = "/dist/esm/debug-web.js";

test.beforeEach(async ({ page }: { page: Page }) => {
  await page.goto(SERVER_URL);
  // The debug variant embeds an unoptimized std as base64, so give slower
  // engines room to decode + compile it.
  await page.evaluate(async (entry) => {
    (window as any).bijouxDebug = await import(entry);
  }, DEBUG_ENTRY);
});

test("release entry does not ship the panic hook; debug entry does", async ({ page }) => {
  await page.waitForFunction(() => (window as any).bijouxReady === true);
  const shape = await page.evaluate(() => ({
    release: typeof (window as any).bijoux.__triggerPanicForTesting,
    debug: typeof (window as any).bijouxDebug.__triggerPanicForTesting,
  }));
  expect(shape.release).toBe("undefined");
  expect(shape.debug).toBe("function");
});

test("a Rust panic surfaces as a catchable PanicError", async ({ page }) => {
  const caught = await page.evaluate(() => {
    const debug = (window as any).bijouxDebug;
    try {
      debug.__triggerPanicForTesting();
      return { threw: false };
    } catch (e) {
      const err = e as Error;
      return {
        threw: true,
        isError: err instanceof Error,
        name: err.name,
        messageIncludesMarker: err.message.includes("deliberate test panic"),
      };
    }
  });
  expect(caught).toEqual({
    threw: true,
    isError: true,
    name: "PanicError",
    messageIncludesMarker: true,
  });
});

test("the Wasm instance remains usable after caught panics", async ({ page }) => {
  const result = await page.evaluate(() => {
    const debug = (window as any).bijouxDebug;

    // Panic several times to shake out cumulative state damage.
    let panics = 0;
    for (let i = 0; i < 3; i++) {
      try {
        debug.__triggerPanicForTesting();
      } catch {
        panics++;
      }
    }

    // Same instance must still round-trip across widths, and structured
    // decode errors must stay distinct from PanicError.
    const encoded = debug.encodeU64(300n);
    let decodeErrorName = "";
    try {
      debug.decodeU64(new Uint8Array([0xff]));
    } catch (e) {
      decodeErrorName = (e as Error).name;
    }

    return {
      panics,
      bytes: [...encoded],
      roundTrip: debug.decodeU64(encoded).value === 300n,
      signedRoundTrip: debug.decodeI64(debug.encodeI64(-1n)).value === -1n,
      decodeErrorName,
    };
  });

  expect(result).toEqual({
    panics: 3,
    bytes: [0xf8, 0x34],
    roundTrip: true,
    signedRoundTrip: true,
    decodeErrorName: "Bijou64DecodeError",
  });
});
