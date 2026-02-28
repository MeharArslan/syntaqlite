// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import "./switch.css";

export interface SwitchOption {
  id: string;
  label: string;
}

export interface SwitchAttrs {
  value: string;
  options: readonly SwitchOption[];
  onChange: (value: string) => void;
  ariaLabel?: string;
  className?: string;
}

export const SegmentedSwitch: m.Component<SwitchAttrs> = {
  view(vnode) {
    const {value, options, onChange, ariaLabel, className} = vnode.attrs;
    const activeIndex = Math.max(
      0,
      options.findIndex((option) => option.id === value),
    );

    return m(
      "div.sq-switch",
      {
        class: className ?? "",
        "data-active-index": String(activeIndex),
        style: {
          "--sq-switch-count": String(Math.max(1, options.length)),
          "--sq-switch-index": String(activeIndex),
        },
        role: "group",
        "aria-label": ariaLabel ?? "Switch",
      },
      options.map((option) =>
        m(
          "button.sq-switch__btn",
          {
            type: "button",
            class: value === option.id ? "sq-switch__btn--active" : "",
            "aria-pressed": value === option.id,
            onclick: () => onChange(option.id),
          },
          option.label,
        ),
      ),
    );
  },
};
