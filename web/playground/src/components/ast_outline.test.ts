import {describe, expect, it} from "vitest";
import {isFieldValueEmpty} from "./ast_outline";

describe("isFieldValueEmpty", () => {
  it("null is empty", () => {
    expect(isFieldValueEmpty(null)).toBe(true);
  });

  it("child node is not empty", () => {
    expect(isFieldValueEmpty({type: "Literal", source: "1"})).toBe(false);
  });

  it("non-empty string is not empty", () => {
    expect(isFieldValueEmpty("hello")).toBe(false);
  });

  it("false bool is empty", () => {
    expect(isFieldValueEmpty(false)).toBe(true);
  });

  it("true bool is not empty", () => {
    expect(isFieldValueEmpty(true)).toBe(false);
  });

  it("empty flags array is empty", () => {
    expect(isFieldValueEmpty([])).toBe(true);
  });

  it("non-empty flags array is not empty", () => {
    expect(isFieldValueEmpty(["DISTINCT"])).toBe(false);
  });
});
