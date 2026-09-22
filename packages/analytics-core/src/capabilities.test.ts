import { readFileSync } from "node:fs";

import { describe, expect, it } from "vitest";

import {
  CAPABILITY_IDS,
  createCapabilityRegistry,
  type CapabilityId,
  type CapabilityManifest,
} from "./capabilities";

const manifest = JSON.parse(
  readFileSync(
    new URL("../../../protocol/capabilities/v1/capabilities.json", import.meta.url),
    "utf8",
  ),
) as CapabilityManifest;

describe("capability registry", () => {
  it("exposes all capability IDs and implementation status", () => {
    const registry = createCapabilityRegistry(manifest);

    expect(CAPABILITY_IDS).toHaveLength(10);
    expect(CAPABILITY_IDS).toEqual(manifest.capabilities.map((capability) => capability.id));
    for (const capability of manifest.capabilities) {
      expect(registry.get(capability.id)?.status).toBe(capability.status);
      expect(registry.dependencies(capability.id)).toEqual(capability.depends_on);
      expect(registry.isImplemented(capability.id)).toBe(capability.status === "implemented");
    }
  });

  it("returns an immutable contract snapshot", () => {
    const registry = createCapabilityRegistry(manifest);
    const sessions = registry.get("sessions");

    expect(registry.dependencies("sessions")).toEqual(["anonymous_visitors"]);
    expect(sessions?.depends_on).toEqual(["anonymous_visitors"]);
    expect(() => (sessions?.depends_on as unknown as CapabilityId[]).push("geo")).toThrow();
    expect(registry.dependencies("sessions")).toEqual(["anonymous_visitors"]);

    manifest.capabilities[3].depends_on = ["geo"];
    expect(registry.dependencies("sessions")).toEqual(["anonymous_visitors"]);
  });
});
