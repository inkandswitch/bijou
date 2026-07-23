import { test, expect, type Page } from "@playwright/test";

/**
 * Cross-browser smoke tests for the published `bijou64` npm package.
 *
 * These tests deliberately exercise the package _through `window.bijou64`_
 * (i.e. the same way a downstream consumer would use it) rather than
 * importing the Rust types directly. That catches regressions in any of:
 *
 * - the wasm-bindgen ABI (BigInt ↔ u64 marshalling, Uint8Array ↔ &[u8])
 * - the wasm-bodge "web" entrypoint (base64 init path)
 * - the package.json subpath exports
 * - cross-browser BigInt + Uint8Array support
 *
 * If a test fails in one browser but not the others, the bug is almost
 * certainly in the JS engine's BigInt or wasm support, not in our code.
 */

const SERVER_URL = "/e2e/server/index.html";

test.beforeEach(async ({ page }: { page: Page }) => {
  await page.goto(SERVER_URL);
  await page.waitForFunction(() => (window as any).bijouxReady === true, {
    timeout: 30_000,
  });
});

test("MAX_BYTES_U64 is 9", async ({ page }) => {
  const max = await page.evaluate(() => (window as any).bijoux.MAX_BYTES_U64());
  expect(max).toBe(9);
});

test("encodes tier-0 values as a single byte equal to the value", async ({ page }) => {
  // Run all three encodeU64 calls inside a single evaluate to keep the BigInt
  // boundary inside the page (Playwright would otherwise need to serialise
  // BigInts back across the IPC boundary, which it does not always handle
  // cleanly across all browsers).
  const result = await page.evaluate(() => {
    const { encodeU64 } = (window as any).bijoux;
    return {
      zero: [...encodeU64(0n)],
      mid: [...encodeU64(42n)],
      max: [...encodeU64(247n)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.mid).toEqual([0x2a]);
  expect(result.max).toEqual([0xf7]);
});

test("encodes tier-1 values with offset", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU64 } = (window as any).bijoux;
    return {
      lower: [...encodeU64(248n)],
      mid: [...encodeU64(300n)],
      upper: [...encodeU64(503n)],
    };
  });
  expect(result.lower).toEqual([0xf8, 0x00]);
  expect(result.mid).toEqual([0xf8, 0x34]);
  expect(result.upper).toEqual([0xf8, 0xff]);
});

test("encodes u64::MAX as 9 bytes starting with 0xFF", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU64 } = (window as any).bijoux;
    const bytes = encodeU64((1n << 64n) - 1n);
    return { length: bytes.length, firstByte: bytes[0] };
  });
  expect(result.length).toBe(9);
  expect(result.firstByte).toBe(0xff);
});

test("encodedLenU64 agrees with encodeU64().length across tier boundaries", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU64, encodedLenU64 } = (window as any).bijoux;
    const cases: bigint[] = [
      0n, 247n, 248n, 503n, 504n, 65_535n, 66_039n, 66_040n,
      16_843_255n, 1n << 32n, (1n << 64n) - 2n, (1n << 64n) - 1n,
    ];
    return cases.map((v) => ({
      value: v.toString(),
      computed: encodedLenU64(v),
      actual: encodeU64(v).length,
    }));
  });
  for (const row of result) {
    expect(row.computed, `encodedLenU64(${row.value}n)`).toBe(row.actual);
  }
});

test("round-trips every tier boundary", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU64, decodeU64 } = (window as any).bijoux;
    const cases: bigint[] = [
      0n, 1n, 247n, 248n, 503n, 504n, 65_535n, 66_039n, 66_040n,
      16_843_255n, 1n << 32n, (1n << 64n) - 2n, (1n << 64n) - 1n,
    ];
    return cases.map((v) => {
      const bytes = encodeU64(v);
      const r = decodeU64(bytes);
      return {
        input: v.toString(),
        decoded: r.value.toString(),
        bytesRead: r.bytesRead,
        encodedLenU64: bytes.length,
      };
    });
  });
  for (const row of result) {
    expect(row.decoded, `round-trip ${row.input}n`).toBe(row.input);
    expect(row.bytesRead).toBe(row.encodedLenU64);
  }
});

test("decodeU64 reports bytesRead < input length when buffer has trailing data", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU64, decodeU64 } = (window as any).bijoux;
    const head = encodeU64(300n); // 2 bytes
    const buf = new Uint8Array(head.length + 3);
    buf.set(head, 0);
    buf.set([0xaa, 0xbb, 0xcc], head.length);
    const r = decodeU64(buf);
    return { value: r.value.toString(), bytesRead: r.bytesRead, inputLength: buf.length };
  });
  expect(result.value).toBe("300");
  expect(result.bytesRead).toBe(2);
  expect(result.inputLength).toBe(5);
});

test("decodeU64 throws Bijou64DecodeError on empty input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeU64 } = (window as any).bijoux;
    try {
      decodeU64(new Uint8Array([]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, message: e.message } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou64DecodeError");
  expect(result.message).toContain("buffer too short");
});

test("decodeU64 throws Bijou64DecodeError on truncated tier-8 input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeU64 } = (window as any).bijoux;
    try {
      // 0xFF needs 8 payload bytes; supply 7.
      decodeU64(new Uint8Array([0xff, 0, 0, 0, 0, 0, 0, 0]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou64DecodeError");
});

test("Bijou64DecodeError thrown is an instance of the platform Error", async ({ page }) => {
  // Sanity-check that the JsValue conversion preserves the JS Error
  // prototype chain — important for downstream code that relies on
  // `instanceof Error` rather than just `.name === "Bijou64DecodeError"`.
  const result = await page.evaluate(() => {
    const { decodeU64 } = (window as any).bijoux;
    try {
      decodeU64(new Uint8Array([0xf8]));
      return { threw: false } as const;
    } catch (e: any) {
      return {
        threw: true,
        isError: e instanceof Error,
        hasStack: typeof e.stack === "string" && e.stack.length > 0,
      } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.isError).toBe(true);
  expect(result.hasStack).toBe(true);
});

test("decodeU64 throws TypeError on non-Uint8Array input (no silent truncation)", async ({ page }) => {
  // Guards the shipped dist against the silent-truncation footgun in a
  // real browser: a plain JS Array would be coerced via
  // `new Uint8Array(arr)`, truncating out-of-range elements
  // (1000 & 0xFF === 232). decodeU64/decodeAllU64 must reject anything that
  // isn't a real Uint8Array.
  const result = await page.evaluate(() => {
    const { decodeU64, decodeAllU64 } = (window as any).bijoux;
    const bad: unknown[] = [[1000], [0x00], null, 42, "nope"];
    const out: { fn: string; allTypeError: boolean } = {
      fn: "",
      allTypeError: true,
    };
    for (const input of bad) {
      for (const [name, f] of [
        ["decodeU64", decodeU64],
        ["decodeAllU64", decodeAllU64],
      ] as const) {
        try {
          f(input);
          out.fn = name;
          out.allTypeError = false; // did not throw at all
        } catch (e: any) {
          if (!(e instanceof TypeError)) {
            out.fn = name;
            out.allTypeError = false;
          }
        }
      }
    }
    return out;
  });
  expect(result.allTypeError, `failing fn: ${result.fn}`).toBe(true);
});

test("encodeU64 throws RangeError for bigint >= 2n ** 64n", async ({ page }) => {
  // wasm-bindgen's default `bigint → u64` marshalling silently
  // truncates via `BigInt.asUintN(64, value)`, which would map any
  // bigint >= 2^64 to its low 64 bits and produce an arbitrary
  // encoding. bijou's API instead rejects with a RangeError so the
  // caller cannot accidentally violate canonicality at the boundary.
  const result = await page.evaluate(() => {
    const { encodeU64 } = (window as any).bijoux;
    try {
      encodeU64(1n << 64n); // exactly 2^64 — one past u64::MAX
      return { threw: false } as const;
    } catch (e: any) {
      return {
        threw: true,
        name: e.name,
        isError: e instanceof Error,
        messageHasRange: typeof e.message === "string" && e.message.includes("2**64"),
      } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("RangeError");
  expect(result.isError).toBe(true);
  expect(result.messageHasRange).toBe(true);
});

test("encodeU64 throws RangeError for negative bigint", async ({ page }) => {
  // Without the range check, two's-complement wraparound would encodeU64
  // `-1n` as the bytes for u64::MAX — a silent footgun.
  const result = await page.evaluate(() => {
    const { encodeU64, encodedLenU64 } = (window as any).bijoux;
    const cases = [
      () => encodeU64(-1n),
      () => encodeU64(-(1n << 63n)),
      () => encodedLenU64(-1n),
    ];
    return cases.map((fn) => {
      try {
        fn();
        return { threw: false } as const;
      } catch (e: any) {
        return { threw: true, name: e.name } as const;
      }
    });
  });
  for (const r of result) {
    expect(r.threw).toBe(true);
    expect(r.name).toBe("RangeError");
  }
});

test("encodeU64 throws TypeError for non-bigint inputs", async ({ page }) => {
  // wasm-bindgen does not enforce the &BigInt type at runtime — the JS
  // shim happily accepts any value. We must distinguish "wrong type"
  // from "out of range" so callers can give useful diagnostics. A plain
  // Number, string, null, or undefined should throw TypeError, NOT
  // RangeError (which would mislead the caller into thinking 42 is
  // out of the [0, 2^64) range — which it obviously isn't).
  const result = await page.evaluate(() => {
    const { encodeU64, encodedLenU64 } = (window as any).bijoux;
    const cases = [
      () => encodeU64(42),               // Number, not bigint
      () => encodeU64("300"),            // String
      () => encodeU64(null),
      () => encodeU64(undefined),
      () => encodedLenU64(42),
      () => encodedLenU64({} as any),    // arbitrary object
    ];
    return cases.map((fn) => {
      try {
        fn();
        return { threw: false } as const;
      } catch (e: any) {
        return {
          threw: true,
          name: e.name,
          isError: e instanceof Error,
          mentionsBigint: typeof e.message === "string" && e.message.toLowerCase().includes("bigint"),
        } as const;
      }
    });
  });
  for (const r of result) {
    expect(r.threw).toBe(true);
    expect(r.name).toBe("TypeError");
    expect(r.isError).toBe(true);
    expect(r.mentionsBigint).toBe(true);
  }
});

test("decodeAllU64 returns a BigUint64Array of every value in the buffer", async ({ page }) => {
  // Verify both the runtime type (typed array, not plain JS array) and
  // the round-trip behaviour.
  const result = await page.evaluate(() => {
    const { encodeU64, decodeAllU64 } = (window as any).bijoux;
    const merged = new Uint8Array([
      ...encodeU64(42n),
      ...encodeU64(300n),
      ...encodeU64(65535n),
      ...encodeU64((1n << 32n)),
    ]);
    const values = decodeAllU64(merged);
    return {
      typeName: values.constructor.name,
      length: values.length,
      isBigUint64Array: values instanceof BigUint64Array,
      asArray: Array.from(values).map((v) => v.toString()),
    };
  });
  expect(result.typeName).toBe("BigUint64Array");
  expect(result.isBigUint64Array).toBe(true);
  expect(result.length).toBe(4);
  expect(result.asArray).toEqual(["42", "300", "65535", "4294967296"]);
});

test("decodeAllU64 on an empty buffer returns an empty BigUint64Array", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeAllU64 } = (window as any).bijoux;
    const empty = decodeAllU64(new Uint8Array(0));
    return { typeName: empty.constructor.name, length: empty.length };
  });
  expect(result.typeName).toBe("BigUint64Array");
  expect(result.length).toBe(0);
});

test("decodeAllU64 throws Bijou64DecodeError on a malformed element", async ({ page }) => {
  // [0x42, 0xF8] — first byte decodes to 0x42 successfully, second
  // byte is a tag with no payload. decodeAllU64 must abort and surface
  // the error, NOT silently return the partial prefix.
  const result = await page.evaluate(() => {
    const { decodeAllU64 } = (window as any).bijoux;
    try {
      decodeAllU64(new Uint8Array([0x42, 0xF8]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, isError: e instanceof Error } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou64DecodeError");
  expect(result.isError).toBe(true);
});

test("encodeU64 accepts the boundary values 0n and u64::MAX exactly", async ({ page }) => {
  // The validation must not be off by one — 0n and (2^64 - 1) are
  // both valid inputs.
  const result = await page.evaluate(() => {
    const { encodeU64 } = (window as any).bijoux;
    return {
      zero: [...encodeU64(0n)],
      max: [...encodeU64((1n << 64n) - 1n)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.max).toEqual([0xff, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0x07]);
});
