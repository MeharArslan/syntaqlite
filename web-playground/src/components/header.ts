// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {Attrs} from "../app/app";
import "./header.css";

export class Header implements m.ClassComponent<Attrs> {
  dialectNameInput = "";
  popoverOpen = false;
  selectedFileName = "";

  view(vnode: m.Vnode<Attrs>) {
    const {app} = vnode.attrs;
    const hasUpload = app.dialect.uploaded !== null;

    return m("header.sq-toolbar", [
      m("div.sq-toolbar__left", [
        m("span.sq-toolbar__brand", [m("span.sq-toolbar__kicker", "syntaqlite"), " Playground"]),
      ]),
      m("div.sq-toolbar__right", [
        m(
          "button.sq-toolbar__theme-toggle",
          {
            type: "button",
            title: "Toggle theme",
            onclick: () => {
              app.theme.toggle();
              m.redraw();
            },
          },
          app.theme.current === "dark" ? "Light" : "Dark",
        ),
        m("div.sq-dialect-popover", {class: this.popoverOpen ? "sq-dialect-popover--open" : ""}, [
          m(
            "button.sq-dialect-popover__trigger",
            {
              type: "button",
              onclick: (e: Event) => {
                e.stopPropagation();
                this.popoverOpen = !this.popoverOpen;
              },
            },
            hasUpload ? `Dialect: ${app.dialect.uploaded?.label}` : "Dialect",
          ),
          m("div.sq-dialect-popover__backdrop", {
            onclick: () => this.closePopover(),
          }),
          m(
            "div.sq-dialect-popover__panel",
            {
              onclick(e: Event) {
                e.stopPropagation();
              },
            },
            [
              m("div.sq-dialect-popover__row", [
                m("span.sq-dialect-popover__label", "File"),
                m(
                  "div.sq-dialect-popover__file-btn",
                  {
                    onclick() {
                      const input = document.getElementById(
                        "dialect-file-input",
                      ) as HTMLInputElement;
                      input?.click();
                    },
                  },
                  this.selectedFileName || "Choose .wasm file...",
                ),
                m("input.sq-dialect-popover__file-input#dialect-file-input[type=file]", {
                  accept: ".wasm,application/wasm",
                  onchange: (e: Event) => {
                    const input = e.target as HTMLInputElement;
                    const file = input.files?.[0];
                    if (file) {
                      this.selectedFileName = file.name;
                      app.dialect.loadFromFile(app.runtime, file, this.dialectNameInput);
                      this.closePopover();
                    }
                  },
                }),
              ]),
              m("div.sq-dialect-popover__row", [
                m("span.sq-dialect-popover__label", "Symbol"),
                m("input.sq-dialect-popover__name[type=text]", {
                  placeholder: "dialect name",
                  value: this.dialectNameInput,
                  oninput: (e: Event) => {
                    this.dialectNameInput = (e.target as HTMLInputElement).value;
                  },
                }),
              ]),
              hasUpload
                ? m("div.sq-dialect-popover__row", [
                    m(
                      "button.sq-ghost.sq-btn-sm",
                      {
                        type: "button",
                        onclick: () => {
                          app.dialect.clearUpload(app.runtime);
                          this.selectedFileName = "";
                          this.closePopover();
                        },
                      },
                      "Unload dialect",
                    ),
                  ])
                : null,
            ],
          ),
        ]),
      ]),
    ]);
  }

  private closePopover() {
    this.popoverOpen = false;
  }
}
