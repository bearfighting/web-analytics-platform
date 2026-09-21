import { readFile } from "node:fs/promises";
import { readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const protocolRoot = resolve(repositoryRoot, "protocol");
const schemaRoot = resolve(protocolRoot, "events/v1/schemas");
const fixtureRoot = resolve(protocolRoot, "events/v1/fixtures");

const pageViewSchema = JSON.parse(
  await readFile(resolve(schemaRoot, "page-view-event.schema.json"), "utf8"),
);
const eventBatchSchema = JSON.parse(
  await readFile(resolve(schemaRoot, "event-batch.schema.json"), "utf8"),
);
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);

ajv.addSchema(pageViewSchema);
const validators = {
  pageView: ajv.compile(pageViewSchema),
  batch: ajv.compile(eventBatchSchema),
};

const fixtureSchemaByFilename = new Map([
  ["page-view.json", "pageView"],
  ["page-view-with-unknown-fields.json", "pageView"],
  ["event-batch.json", "batch"],
  ["batch-invalid-event.json", "batch"],
  ["empty-batch.json", "batch"],
  ["oversized-batch.json", "batch"],
  ["invalid-event-type.json", "pageView"],
  ["invalid-occurred-at.json", "pageView"],
  ["missing-event-id.json", "pageView"],
]);

async function validateFixtures(directory, expectedValid) {
  const filenames = readdirSync(directory)
    .filter((filename) => filename.endsWith(".json"))
    .sort();
  let failures = 0;

  for (const filename of filenames) {
    const path = resolve(directory, filename);
    const fixture = JSON.parse(await readFile(path, "utf8"));
    const schemaName = fixtureSchemaByFilename.get(filename);

    if (!schemaName) {
      throw new Error(`No Protocol schema mapping found for fixture: ${filename}`);
    }

    const validator = validators[schemaName];
    const actualValid = validator(fixture);

    if (actualValid !== expectedValid) {
      console.error(`Protocol validation mismatch: ${path}`);
      console.error(validator.errors ?? "no validation details");
      failures += 1;
      continue;
    }

    console.log(`${expectedValid ? "PASS" : "EXPECTED FAIL"} ${path}`);
  }

  return failures;
}

const failures =
  (await validateFixtures(resolve(fixtureRoot, "valid"), true)) +
  (await validateFixtures(resolve(fixtureRoot, "invalid"), false));

if (failures > 0) {
  process.exitCode = 1;
} else {
  console.log("Protocol fixtures validated successfully.");
}
