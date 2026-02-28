import {describe, expect, it} from "vitest";
import {isFieldEmpty} from "./ast_outline";

describe("isFieldEmpty", () => {
  it("node: empty when child is undefined", () => {
    expect(isFieldEmpty({kind: "node", label: "x", child: undefined})).toBe(true);
  });

  it("node: not empty when child exists", () => {
    const child = {type: "node" as const, name: "N", fields: []};
    expect(isFieldEmpty({kind: "node", label: "x", child})).toBe(false);
  });

  it("span: empty when value is undefined", () => {
    expect(isFieldEmpty({kind: "span", label: "x", value: undefined})).toBe(true);
  });

  it("span: not empty when value is a string", () => {
    expect(isFieldEmpty({kind: "span", label: "x", value: "hello"})).toBe(false);
  });

  it("bool: empty when false", () => {
    expect(isFieldEmpty({kind: "bool", label: "x", value: false})).toBe(true);
  });

  it("bool: not empty when true", () => {
    expect(isFieldEmpty({kind: "bool", label: "x", value: true})).toBe(false);
  });

  it("enum: empty when value is undefined", () => {
    expect(isFieldEmpty({kind: "enum", label: "x", value: undefined})).toBe(true);
  });

  it("enum: not empty when value is set", () => {
    expect(isFieldEmpty({kind: "enum", label: "x", value: "ASC"})).toBe(false);
  });

  it("flags: empty when array is empty", () => {
    expect(isFieldEmpty({kind: "flags", label: "x", value: []})).toBe(true);
  });

  it("flags: not empty when array has items", () => {
    expect(isFieldEmpty({kind: "flags", label: "x", value: ["DISTINCT"]})).toBe(false);
  });
});
