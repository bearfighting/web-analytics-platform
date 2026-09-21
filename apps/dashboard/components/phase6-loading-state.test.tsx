import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";

import { Phase6LoadingState } from "./phase6-loading-state";

describe("Phase6LoadingState", () => {
  it("identifies the Phase 6 section while loading", () => {
    const markup = renderToStaticMarkup(<Phase6LoadingState heading="Dimension Report" />);

    expect(markup).toContain("Dimension Report");
    expect(markup).toContain("Loading Phase 6 analytics");
    expect(markup).toContain('role="status"');
  });
});
