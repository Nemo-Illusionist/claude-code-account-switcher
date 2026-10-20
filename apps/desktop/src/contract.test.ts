import { describe, expect, it } from "vitest";
import json from "./contract.sample.json";
import { sample } from "./contract.sample";
import { errors } from "./api";

describe("contract", () => {
  it("the typed sample is exactly what the Rust tests serialize", () => {
    expect(json).toStrictEqual(sample);
  });

  it("every error code the backend can send has copy, and no other", () => {
    const codes = [...sample.gui_errors, ...sample.desktop_errors].sort();
    expect(Object.keys(errors).sort()).toStrictEqual(codes);
  });
});
