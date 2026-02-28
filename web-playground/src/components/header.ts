// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {Attrs} from "../app/app";
import {VERSION_OPTIONS} from "../app/dialect_config_manager";
import {DIALECT_PRESETS} from "../app/dialect_manager";
import {HelpTooltip} from "./help_tooltip";
import {SegmentedSwitch} from "./switch";
import "./header.css";

const THEME_SWITCH_OPTIONS = [
  {id: "light", label: "Light"},
  {id: "dark", label: "Dark"},
] as const;

export class Header implements m.ClassComponent<Attrs> {
  private customPopoverOpen = false;
  private customSymbol = "syntaqlite_dialect";
  private customFile: File | null = null;
  private customError: string | null = null;
  private customLoading = false;
  private configPopoverOpen = false;

  view(vnode: m.Vnode<Attrs>) {
    const {app} = vnode.attrs;
    const activeId = app.dialect.activePresetId;

    return m("header.sq-toolbar", [
      m("div.sq-toolbar__left", [
        m("span.sq-toolbar__brand", [m("span.sq-toolbar__kicker", "syntaqlite"), " Playground"]),
      ]),
      m("div.sq-toolbar__right", [
        m("div.sq-dialect-controls", [
          m("span.sq-dialect-controls__label", "Dialect"),
          m(HelpTooltip, {
            ariaLabel: "Dialect requirements help",
            text: "Dialects must be SQLite-based.",
            linkHref: "https://github.com/LalitMaganti/syntaqlite/tree/main/docs",
            linkLabel: "TODO: docs",
          }),
          m("div.sq-dialect-switcher", [
            ...DIALECT_PRESETS.map((preset) =>
              preset.id === "sqlite"
                ? m(
                    "div.sq-config-popover",
                    {class: this.configPopoverOpen ? "sq-config-popover--open" : ""},
                    [
                      m(
                        "button.sq-dialect-switcher__btn.sq-dialect-switcher__btn--with-chevron",
                        {
                          class: activeId === preset.id ? "sq-dialect-switcher__btn--active" : "",
                          onclick: () => app.dialect.selectPreset(app.runtime, preset),
                        },
                        [
                          m("span", preset.label),
                          activeId === "sqlite"
                            ? m("span.sq-config-chevron", {
                                onclick: (e: Event) => {
                                  e.stopPropagation();
                                  this.configPopoverOpen = !this.configPopoverOpen;
                                },
                              }, "\u25BE")
                            : null,
                        ],
                      ),
                      m("div.sq-config-popover__backdrop", {
                        onclick: () => {
                          this.configPopoverOpen = false;
                        },
                      }),
                      m("div.sq-config-popover__panel", {onclick: (e: Event) => e.stopPropagation()}, [
                        m("div.sq-config-popover__section", [
                          m("span.sq-config-popover__label", "SQLite Version"),
                          m(
                            "select.sq-config-popover__select",
                            {
                              value: app.dialectConfig.version,
                              onchange: (e: Event) => {
                                app.dialectConfig.version = (e.target as HTMLSelectElement).value;
                                const visible = new Set(app.dialectConfig.visibleCflags);
                                for (const flag of app.dialectConfig.enabledCflags) {
                                  if (!visible.has(flag)) app.dialectConfig.enabledCflags.delete(flag);
                                }
                                app.dialectConfig.apply(app.runtime);
                                m.redraw();
                              },
                            },
                            VERSION_OPTIONS.map((v) => m("option", {value: v}, v)),
                          ),
                        ]),
                        m("div.sq-config-popover__section", [
                          m("span.sq-config-popover__label", "Compile Flags"),
                          m(
                            "div.sq-config-popover__cflag-list",
                            (() => {
                              const entries = app.dialectConfig.visibleCflagEntries;
                              const groups: Record<string, string[]> = {};
                              for (const e of entries) {
                                (groups[e.category] ??= []).push(e.name);
                              }
                              const categoryOrder = ["parser", "functions", "extensions", "vtable"];
                              const categoryLabels: Record<string, string> = {
                                parser: "Parser",
                                functions: "Functions",
                                extensions: "Extensions",
                                vtable: "Virtual Tables",
                              };
                              const renderGroup = (label: string, items: string[]) =>
                                items.length === 0
                                  ? null
                                  : [
                                      m("div.sq-config-popover__group-label", label),
                                      ...items.map((suffix) =>
                                        m("label.sq-config-popover__cflag-item", [
                                          m("input[type=checkbox]", {
                                            checked: app.dialectConfig.enabledCflags.has(suffix),
                                            onchange: () => {
                                              if (app.dialectConfig.enabledCflags.has(suffix)) {
                                                app.dialectConfig.enabledCflags.delete(suffix);
                                              } else {
                                                app.dialectConfig.enabledCflags.add(suffix);
                                              }
                                              app.dialectConfig.apply(app.runtime);
                                              m.redraw();
                                            },
                                          }),
                                          m("span", suffix),
                                        ]),
                                      ),
                                    ];
                              return categoryOrder.map((cat) =>
                                renderGroup(categoryLabels[cat] ?? cat, groups[cat] ?? []),
                              );
                            })(),
                          ),
                        ]),
                        m("div.sq-config-popover__section", [
                          m("span.sq-config-popover__label", "Schema Context"),
                          m("textarea.sq-config-popover__textarea", {
                            placeholder: "table_name: col1, col2\nusers: id, name, email",
                            rows: 4,
                            value: app.schemaContext.rawText,
                            oninput: (e: Event) => {
                              app.schemaContext.rawText = (e.target as HTMLTextAreaElement).value;
                              app.schemaContext.apply(app.runtime);
                              m.redraw();
                            },
                          }),
                          m("span.sq-config-popover__help-text", "One table per line: table_name: col1, col2"),
                        ]),
                        m("div.sq-config-popover__section", [
                          m(
                            "button.sq-config-popover__reset-btn",
                            {
                              onclick: () => {
                                app.dialectConfig.reset(app.runtime);
                                app.schemaContext.reset(app.runtime);
                                m.redraw();
                              },
                            },
                            "Reset to Defaults",
                          ),
                        ]),
                      ]),
                    ],
                  )
                : m(
                    "button.sq-dialect-switcher__btn",
                    {
                      class: activeId === preset.id ? "sq-dialect-switcher__btn--active" : "",
                      onclick: () => app.dialect.selectPreset(app.runtime, preset),
                    },
                    preset.label,
                  ),
            ),
            m(
              "div.sq-dialect-popover",
              {class: this.customPopoverOpen ? "sq-dialect-popover--open" : ""},
              [
                m(
                  "button.sq-dialect-switcher__btn",
                  {
                    class: activeId === "custom" ? "sq-dialect-switcher__btn--active" : "",
                    onclick: (e: Event) => {
                      e.stopPropagation();
                      this.customPopoverOpen = !this.customPopoverOpen;
                      if (this.customPopoverOpen) this.customError = null;
                    },
                  },
                  activeId === "custom" && app.dialect.customLabel
                    ? app.dialect.customLabel
                    : "Custom",
                ),
                m("div.sq-dialect-popover__backdrop", {onclick: () => this.closePopover()}),
                m("div.sq-dialect-popover__panel", {onclick: (e: Event) => e.stopPropagation()}, [
                  m("div.sq-dialect-popover__row", [
                    m("span.sq-dialect-popover__label", "File"),
                    m(
                      "div.sq-dialect-popover__file-btn",
                      {
                        onclick: () => {
                          const input = document.getElementById(
                            "dialect-file-input",
                          ) as HTMLInputElement;
                          input?.click();
                        },
                      },
                      this.customFile ? this.customFile.name : "Choose .wasm file...",
                    ),
                    m("input.sq-dialect-popover__file-input#dialect-file-input[type=file]", {
                      accept: ".wasm,application/wasm",
                      onchange: (e: Event) => {
                        const input = e.target as HTMLInputElement;
                        const file = input.files?.[0];
                        if (file) {
                          this.customFile = file;
                          this.customError = null;
                        }
                      },
                    }),
                  ]),
                  m("div.sq-dialect-popover__row", [
                    m("span.sq-dialect-popover__label", "Symbol"),
                    m("input.sq-dialect-popover__name[type=text]", {
                      placeholder: "syntaqlite_xyz_dialect",
                      value: this.customSymbol,
                      oninput: (e: Event) => {
                        this.customSymbol = (e.target as HTMLInputElement).value;
                        this.customError = null;
                      },
                    }),
                  ]),
                  m("div.sq-dialect-popover__row.sq-dialect-popover__row--help", [
                    m("span.sq-dialect-popover__label", "Help"),
                    m(HelpTooltip, {
                      className: "sq-dialect-popover__help-tooltip",
                      ariaLabel: "Custom dialect generation help",
                      text: "Custom dialect modules must be generated from a SQLite-based dialect build.",
                      linkHref: "https://github.com/LalitMaganti/syntaqlite/tree/main/docs",
                      linkLabel: "TODO: how to generate these",
                    }),
                  ]),
                  this.customError
                    ? m("div.sq-dialect-popover__error", this.customError)
                    : null,
                  m("div.sq-dialect-popover__row", [
                    m(
                      "button.sq-dialect-popover__load-btn",
                      {
                        disabled: !this.customFile || this.customLoading,
                        onclick: () => this.loadCustom(app),
                      },
                      this.customLoading ? "Loading..." : "Load",
                    ),
                  ]),
                ]),
              ],
            ),
          ]),
        ]),
        m("div.sq-theme-controls", [
          m("span.sq-theme-controls__label", "Theme"),
          m(SegmentedSwitch, {
            options: THEME_SWITCH_OPTIONS,
            value: app.theme.current,
            ariaLabel: "Theme",
            onChange: (value) => {
              app.theme.set(value as "light" | "dark");
              m.redraw();
            },
          }),
        ]),
      ]),
    ]);
  }

  private async loadCustom(app: InstanceType<typeof import("../app/app").App>) {
    if (!this.customFile) return;
    this.customLoading = true;
    this.customError = null;
    m.redraw();
    const error = await app.dialect.loadFromFile(app.runtime, this.customFile, this.customSymbol);
    this.customLoading = false;
    if (error) {
      this.customError = error;
    } else {
      this.closePopover();
    }
    m.redraw();
  }

  private closePopover() {
    this.customPopoverOpen = false;
  }

}
