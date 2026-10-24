import { describe, expect, it } from "vitest";
import json from "./contract.sample.json";
import library from "../../../src/gui.contract.json";
import { sample } from "./contract.sample";

describe("contract", () => {
  it("the typed sample is exactly what the shell crate's tests serialize", () => {
    expect(json).toStrictEqual(sample);
  });

  it("agrees with the library's own contract file, field for field", () => {
    // `revision` is the shell's ordering number, added around the library
    // snapshot; everything else must be exactly what `claude_acc::gui` sends.
    const { revision: _revision, ...snapshot } = sample.snapshot;
    expect(snapshot).toStrictEqual(library.snapshot);
    expect(sample.bindings).toStrictEqual(library.bindings);
    expect(sample.sessions).toStrictEqual(library.sessions);
    expect(sample.editors).toStrictEqual(library.editors);
    expect(sample.update_check).toStrictEqual(library.update_check);
    expect(sample.desktop_profiles).toStrictEqual(library.desktop_profiles);
    expect(sample.runtime_cloned).toStrictEqual(library.runtime_cloned);
    expect(sample.gui_errors).toStrictEqual(library.gui_errors);
  });
});
