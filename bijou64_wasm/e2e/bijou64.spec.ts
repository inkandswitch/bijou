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
  await page.waitForFunction(() => (window as any).bijou64Ready === true, {
    timeout: 30_000,
  });
});

test("MAX_BYTES is 9", async ({ page }) => {
  const max = await page.evaluate(() => (window as any).bijou64.MAX_BYTES());
  expect(max).toBe(9);
});

test("encodes tier-0 values as a single byte equal to the value", async ({ page }) => {
  // Run all three encode calls inside a single evaluate to keep the BigInt
  // boundary inside the page (Playwright would otherwise need to serialise
  // BigInts back across the IPC boundary, which it does not always handle
  // cleanly across all browsers).
  const result = await page.evaluate(() => {
    const { encode } = (window as any).bijou64;
    return {
      zero: [...encode(0n)],
      mid: [...encode(42n)],
      max: [...encode(247n)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.mid).toEqual([0x2a]);
  expect(result.max).toEqual([0xf7]);
});

test("encodes tier-1 values with offset", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode } = (window as any).bijou64;
    return {
      lower: [...encode(248n)],
      mid: [...encode(300n)],
      upper: [...encode(503n)],
    };
  });
  expect(result.lower).toEqual([0xf8, 0x00]);
  expect(result.mid).toEqual([0xf8, 0x34]);
  expect(result.upper).toEqual([0xf8, 0xff]);
});

test("encodes u64::MAX as 9 bytes starting with 0xFF", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode } = (window as any).bijou64;
    const bytes = encode((1n << 64n) - 1n);
    return { length: bytes.length, firstByte: bytes[0] };
  });
  expect(result.length).toBe(9);
  expect(result.firstByte).toBe(0xff);
});

test("encodedLen agrees with encode().length across tier boundaries", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, encodedLen } = (window as any).bijou64;
    const cases: bigint[] = [
      0n, 247n, 248n, 503n, 504n, 65_535n, 66_039n, 66_040n,
      16_843_255n, 1n << 32n, (1n << 64n) - 2n, (1n << 64n) - 1n,
    ];
    return cases.map((v) => ({
      value: v.toString(),
      computed: encodedLen(v),
      actual: encode(v).length,
    }));
  });
  for (const row of result) {
    expect(row.computed, `encodedLen(${row.value}n)`).toBe(row.actual);
  }
});

test("round-trips every tier boundary", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, decode } = (window as any).bijou64;
    const cases: bigint[] = [
      0n, 1n, 247n, 248n, 503n, 504n, 65_535n, 66_039n, 66_040n,
      16_843_255n, 1n << 32n, (1n << 64n) - 2n, (1n << 64n) - 1n,
    ];
    return cases.map((v) => {
      const bytes = encode(v);
      const r = decode(bytes);
      return {
        input: v.toString(),
        decoded: r.value.toString(),
        bytesRead: r.bytesRead,
        encodedLen: bytes.length,
      };
    });
  });
  for (const row of result) {
    expect(row.decoded, `round-trip ${row.input}n`).toBe(row.input);
    expect(row.bytesRead).toBe(row.encodedLen);
  }
});

test("decode reports bytesRead < input length when buffer has trailing data", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, decode } = (window as any).bijou64;
    const head = encode(300n); // 2 bytes
    const buf = new Uint8Array(head.length + 3);
    buf.set(head, 0);
    buf.set([0xaa, 0xbb, 0xcc], head.length);
    const r = decode(buf);
    return { value: r.value.toString(), bytesRead: r.bytesRead, inputLength: buf.length };
  });
  expect(result.value).toBe("300");
  expect(result.bytesRead).toBe(2);
  expect(result.inputLength).toBe(5);
});

test("decode throws DecodeError on empty input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decode } = (window as any).bijou64;
    try {
      decode(new Uint8Array([]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, message: e.message } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("DecodeError");
  expect(result.message).toContain("buffer too short");
});

test("decode throws DecodeError on truncated tier-8 input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decode } = (window as any).bijou64;
    try {
      // 0xFF needs 8 payload bytes; supply 7.
      decode(new Uint8Array([0xff, 0, 0, 0, 0, 0, 0, 0]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("DecodeError");
});

test("DecodeError thrown is an instance of the platform Error", async ({ page }) => {
  // Sanity-check that the JsValue conversion preserves the JS Error
  // prototype chain — important for downstream code that relies on
  // `instanceof Error` rather than just `.name === "DecodeError"`.
  const result = await page.evaluate(() => {
    const { decode } = (window as any).bijou64;
    try {
      decode(new Uint8Array([0xf8]));
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
