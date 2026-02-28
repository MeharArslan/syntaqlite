// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";

export interface HelpTooltipAttrs {
  ariaLabel: string;
  text: string;
  linkHref?: string;
  linkLabel?: string;
  className?: string;
}

export class HelpTooltip implements m.ClassComponent<HelpTooltipAttrs> {
  view(vnode: m.Vnode<HelpTooltipAttrs>) {
    const {ariaLabel, text, linkHref, linkLabel, className} = vnode.attrs;
    return m("div", {class: `sq-help-tooltip ${className ?? ""}`.trim()}, [
      m(
        "button.sq-help-tooltip__icon",
        {
          type: "button",
          "aria-label": ariaLabel,
        },
        "?",
      ),
      m("div.sq-help-tooltip__panel", [
        m("span", text),
        linkHref && linkLabel
          ? m(
              "a",
              {
                href: linkHref,
                target: "_blank",
                rel: "noopener noreferrer",
              },
              linkLabel,
            )
          : undefined,
      ]),
    ]);
  }
}
