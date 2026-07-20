import { test, expect, type Page } from "@playwright/test";

/**
 * Cross-browser smoke tests for the published `bijou128` npm package.
 *
 * These tests deliberately exercise the package _through `window.bijou128`_
 * (i.e. the same way a downstream consumer would use it) rather than
 * importing the Rust types directly. That catches regressions in any of:
 *
 * - the wasm-bindgen ABI (BigInt ↔ u128 marshalling, Uint8Array ↔ &[u8])
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

test("MAX_BYTES_U128 is 17", async ({ page }) => {
  const max = await page.evaluate(() => (window as any).bijoux.MAX_BYTES_U128());
  expect(max).toBe(17);
});

test("encodes tier-0 values as a single byte equal to the value", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128 } = (window as any).bijoux;
    return {
      zero: [...encodeU128(0n)],
      mid: [...encodeU128(42n)],
      max: [...encodeU128(239n)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.mid).toEqual([0x2a]);
  expect(result.max).toEqual([0xef]);
});

test("encodes tier-1 values with offset", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128 } = (window as any).bijoux;
    return {
      lower: [...encodeU128(240n)],
      mid: [...encodeU128(300n)],
      upper: [...encodeU128(495n)],
    };
  });
  expect(result.lower).toEqual([0xf0, 0x00]);
  expect(result.mid).toEqual([0xf0, 0x3c]);
  expect(result.upper).toEqual([0xf0, 0xff]);
});

test("encodes u128::MAX as 17 bytes starting with 0xFF", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128 } = (window as any).bijoux;
    const bytes = encodeU128((1n << 128n) - 1n);
    return { length: bytes.length, firstByte: bytes[0] };
  });
  expect(result.length).toBe(17);
  expect(result.firstByte).toBe(0xff);
});

test("encodedLenU128 agrees with encodeU128().length across tier boundaries", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128, encodedLenU128 } = (window as any).bijoux;
    const cases: bigint[] = [
      0n, 239n, 240n, 495n, 496n, 65_535n, 66_031n, 66_032n,
      1n << 32n, 1n << 64n, 1n << 96n, (1n << 128n) - 2n, (1n << 128n) - 1n,
    ];
    return cases.map((v) => ({
      value: v.toString(),
      computed: encodedLenU128(v),
      actual: encodeU128(v).length,
    }));
  });
  for (const row of result) {
    expect(row.computed, `encodedLenU128(${row.value}n)`).toBe(row.actual);
  }
});

test("round-trips every tier boundary", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128, decodeU128 } = (window as any).bijoux;
    const cases: bigint[] = [
      0n, 1n, 239n, 240n, 495n, 496n, 65_535n, 66_031n, 66_032n,
      1n << 32n, 1n << 64n, 1n << 96n, (1n << 128n) - 2n, (1n << 128n) - 1n,
    ];
    return cases.map((v) => {
      const bytes = encodeU128(v);
      const r = decodeU128(bytes);
      return {
        input: v.toString(),
        decoded: r.value.toString(),
        bytesRead: r.bytesRead,
        encodedLenU128: bytes.length,
      };
    });
  });
  for (const row of result) {
    expect(row.decoded, `round-trip ${row.input}n`).toBe(row.input);
    expect(row.bytesRead).toBe(row.encodedLenU128);
  }
});

test("decodeU128 reports bytesRead < input length when buffer has trailing data", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128, decodeU128 } = (window as any).bijoux;
    const head = encodeU128(500n); // 3 bytes
    const buf = new Uint8Array(head.length + 3);
    buf.set(head, 0);
    buf.set([0xaa, 0xbb, 0xcc], head.length);
    const r = decodeU128(buf);
    return { value: r.value.toString(), bytesRead: r.bytesRead, inputLength: buf.length };
  });
  expect(result.value).toBe("500");
  expect(result.bytesRead).toBe(3);
  expect(result.inputLength).toBe(6);
});

test("decodeU128 throws Bijou128DecodeError on empty input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeU128 } = (window as any).bijoux;
    try {
      decodeU128(new Uint8Array([]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, message: e.message } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou128DecodeError");
  expect(result.message).toContain("buffer too short");
});

test("decodeU128 throws Bijou128DecodeError on truncated tier-16 input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeU128 } = (window as any).bijoux;
    try {
      // 0xFF needs 16 payload bytes; supply 15.
      decodeU128(new Uint8Array([0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou128DecodeError");
});

test("Bijou128DecodeError thrown is an instance of the platform Error", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeU128 } = (window as any).bijoux;
    try {
      decodeU128(new Uint8Array([0xf0]));
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

test("decodeU128 throws TypeError on non-Uint8Array input (no silent truncation)", async ({ page }) => {
  // Guards the shipped dist against the silent-truncation footgun in a
  // real browser: a plain JS Array would be coerced via
  // `new Uint8Array(arr)`, truncating out-of-range elements
  // (1000 & 0xFF === 232). decodeU128/decodeAllU128 must reject anything that
  // isn't a real Uint8Array.
  const result = await page.evaluate(() => {
    const { decodeU128, decodeAllU128 } = (window as any).bijoux;
    const bad: unknown[] = [[1000], [0x00], null, 42, "nope"];
    const out = { fn: "", allTypeError: true };
    for (const input of bad) {
      for (const [name, f] of [
        ["decodeU128", decodeU128],
        ["decodeAllU128", decodeAllU128],
      ] as const) {
        try {
          f(input);
          out.fn = name;
          out.allTypeError = false;
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

test("encodeU128 throws RangeError for bigint >= 2n ** 128n", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128 } = (window as any).bijoux;
    try {
      encodeU128(1n << 128n);
      return { threw: false } as const;
    } catch (e: any) {
      return {
        threw: true,
        name: e.name,
        isError: e instanceof Error,
        messageHasRange: typeof e.message === "string" && e.message.includes("2**128"),
      } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("RangeError");
  expect(result.isError).toBe(true);
  expect(result.messageHasRange).toBe(true);
});

test("encodeU128 throws RangeError for negative bigint", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128, encodedLenU128 } = (window as any).bijoux;
    const cases = [
      () => encodeU128(-1n),
      () => encodeU128(-(1n << 127n)),
      () => encodedLenU128(-1n),
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

test("encodeU128 throws TypeError for non-bigint inputs", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128, encodedLenU128 } = (window as any).bijoux;
    const cases = [
      () => encodeU128(42),
      () => encodeU128("300"),
      () => encodeU128(null),
      () => encodeU128(undefined),
      () => encodedLenU128(42),
      () => encodedLenU128({} as any),
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

test("decodeAllU128 returns an Array<bigint> of every value in the buffer", async ({ page }) => {
  // Unlike bijou64 (which returns BigUint64Array), bijou128 returns a
  // plain Array because there is no BigUint128Array in the web platform.
  const result = await page.evaluate(() => {
    const { encodeU128, decodeAllU128 } = (window as any).bijoux;
    const merged = new Uint8Array([
      ...encodeU128(42n),
      ...encodeU128(500n),
      ...encodeU128(65535n),
      ...encodeU128(1n << 64n),
    ]);
    const values = decodeAllU128(merged);
    return {
      typeName: values.constructor.name,
      length: values.length,
      isArray: Array.isArray(values),
      asArray: values.map((v: bigint) => v.toString()),
    };
  });
  expect(result.typeName).toBe("Array");
  expect(result.isArray).toBe(true);
  expect(result.length).toBe(4);
  expect(result.asArray).toEqual(["42", "500", "65535", "18446744073709551616"]);
});

test("decodeAllU128 on an empty buffer returns an empty Array", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeAllU128 } = (window as any).bijoux;
    const empty = decodeAllU128(new Uint8Array(0));
    return { typeName: empty.constructor.name, length: empty.length };
  });
  expect(result.typeName).toBe("Array");
  expect(result.length).toBe(0);
});

test("decodeAllU128 throws Bijou128DecodeError on a malformed element", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeAllU128 } = (window as any).bijoux;
    try {
      decodeAllU128(new Uint8Array([0x42, 0xf0]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, isError: e instanceof Error } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou128DecodeError");
  expect(result.isError).toBe(true);
});

test("encodeU128 accepts the boundary values 0n and u128::MAX exactly", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU128 } = (window as any).bijoux;
    return {
      zero: [...encodeU128(0n)],
      max: [...encodeU128((1n << 128n) - 1n)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.max).toEqual([
    0xff, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe,
    0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe,
    0x0f,
  ]);
});
