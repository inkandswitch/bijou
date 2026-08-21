import { expect } from "chai";
import * as debugBijoux from "../dist/esm/debug-node.js";

/**
 * panic=unwind contract tests.
 *
 * The bijoux API is total — no public function has a reachable panic — so
 * these tests use `__triggerPanicForTesting`, a doc-hidden hook that exists
 * only in the `/debug` package variant (gated on `debug_assertions`, which
 * the `wasm-debug` cargo profile enables and `release` does not).
 *
 * What is actually under test is wasm-bindgen's unwind glue, which is
 * identical in both variants: a Rust panic must surface as a catchable JS
 * `Error` with `name === "PanicError"`, and the Wasm instance must remain
 * fully usable afterward (destructors ran, no poisoned state). This is the
 * behavioral difference vs the old panic=abort builds, where any panic
 * killed the instance.
 *
 * Browser counterpart: `e2e/panic.spec.ts`.
 */

describe("panic=unwind (node, /debug variant)", () => {
  it("the release entry points do NOT ship the panic hook", async () => {
    const release = await import("../dist/esm/node.js");
    expect((release as any).__triggerPanicForTesting).to.equal(undefined);
  });

  it("a Rust panic surfaces as a catchable Error with name 'PanicError'", () => {
    let caught: unknown;
    try {
      (debugBijoux as any).__triggerPanicForTesting();
      expect.fail("expected __triggerPanicForTesting to throw");
    } catch (e) {
      caught = e;
    }

    expect(caught).to.be.instanceOf(Error);
    expect((caught as Error).name).to.equal("PanicError");
    expect((caught as Error).message).to.include("deliberate test panic");
  });

  it("the Wasm instance remains fully usable after a caught panic", () => {
    // Panic (and catch) several times to shake out cumulative state damage.
    for (let i = 0; i < 3; i++) {
      expect(() => (debugBijoux as any).__triggerPanicForTesting()).to.throw();
    }

    // Encode/decode round-trips across widths still work on the SAME instance.
    const encoded = debugBijoux.encodeU64(300n);
    expect([...encoded]).to.deep.equal([0xf8, 0x34]);
    expect(debugBijoux.decodeU64(encoded).value).to.equal(300n);

    expect(debugBijoux.decodeI64(debugBijoux.encodeI64(-1n)).value).to.equal(-1n);
    expect(debugBijoux.decodeU32(debugBijoux.encodeU32(42)).value).to.equal(42);

    // Structured (non-panic) errors still carry their own names, not
    // PanicError — the two error channels stay distinct.
    try {
      debugBijoux.decodeU64(new Uint8Array([0xff]));
      expect.fail("expected a decode error");
    } catch (e) {
      expect((e as Error).name).to.equal("Bijou64DecodeError");
    }
  });
});
