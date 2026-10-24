import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { Meter } from "./components";
import { CopyProvider } from "./i18n";

afterEach(cleanup);

const meter = (percent: number) => {
  render(
    <CopyProvider lang="en">
      <Meter label="5-hour window" value={{ percent, resets_at: null }} now={0} />
    </CopyProvider>,
  );
  const element = screen.getByRole("meter");
  const read = (name: string) => element.getAttribute(name);
  const bar = element.firstElementChild as HTMLElement;
  return {
    label: screen.getByText(/%$/).textContent,
    now: read("aria-valuenow"),
    min: read("aria-valuemin"),
    max: read("aria-valuemax"),
    text: read("aria-valuetext"),
    width: bar.style.width,
  };
};

it("every channel of the meter reports the same figure", () => {
  // Regression: `valuenow` was clamped while the label was not (120), the
  // label was rounded while `valuenow` was not (99.5), and `valuenow` was
  // clamped at 0 while the label said -5. One element, two numbers, each time.
  const cases: [number, Record<string, string>][] = [
    [-5, { label: "-5%", now: "-5", min: "-5", max: "100", text: "-5%", width: "0%" }],
    [0, { label: "0%", now: "0", min: "0", max: "100", text: "0%", width: "0%" }],
    [42, { label: "42%", now: "42", min: "0", max: "100", text: "42%", width: "42%" }],
    [99.5, { label: "100%", now: "100", min: "0", max: "100", text: "100%", width: "100%" }],
    [120, { label: "120%", now: "120", min: "0", max: "120", text: "120%", width: "100%" }],
    [120.4, { label: "120%", now: "120", min: "0", max: "120", text: "120%", width: "100%" }],
  ];
  for (const [percent, expected] of cases) {
    expect(meter(percent), String(percent)).toStrictEqual(expected);
    cleanup();
  }
});

it("sets the document's language and title from the table", () => {
  document.documentElement.lang = "";
  document.title = "";
  render(
    <CopyProvider lang="en">
      <span />
    </CopyProvider>,
  );
  expect(document.documentElement.lang).toBe("en");
  expect(document.title).toBe("Claude Accounts");
});
