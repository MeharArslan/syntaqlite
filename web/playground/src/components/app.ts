// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

import m from "mithril";
import type {Attrs} from "../app/app";
import {Header} from "./header";
import {DetailsPanel} from "./details_panel";
import {Workspace} from "./workspace";

export class AppComponent implements m.ClassComponent<Attrs> {
  view(vnode: m.Vnode<Attrs>) {
    const {app} = vnode.attrs;
    return m("main.sq-app", [
      m(Header, {app}),
      app.runtime.statusError
        ? m("div.sq-error-banner", app.runtime.status)
        : undefined,
      m(Workspace, {app}),
      m(DetailsPanel, {app}),
    ]);
  }
}
