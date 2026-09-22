import { readFile } from "node:fs/promises";
import { readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const protocolRoot = resolve(repositoryRoot, "protocol");
const schemaRoot = resolve(protocolRoot, "events/schemas");
const fixtureRoot = resolve(protocolRoot, "events/fixtures");

const readJson = async (path) => JSON.parse(await readFile(path, "utf8"));
const pageViewSchema = await readJson(resolve(schemaRoot, "page-view-event.schema.json"));
const eventBatchSchema = await readJson(resolve(schemaRoot, "event-batch.schema.json"));
const contextSchema = await readJson(resolve(protocolRoot, "contexts/browser-context.schema.json"));
const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormats(ajv);

ajv.addSchema(pageViewSchema);
ajv.addSchema(contextSchema);
const validators = {
  pageView: ajv.compile(pageViewSchema),
  batch: ajv.compile(eventBatchSchema),
};

async function validateFixtures(directory, expectedValid) {
  const filenames = readdirSync(directory)
    .filter((filename) => filename.endsWith(".json"))
    .sort();
  let failures = 0;

  for (const filename of filenames) {
    const path = resolve(directory, filename);
    const fixture = await readJson(path);
    const validator =
      filename.startsWith("event-batch") || filename === "oversized-batch.json"
        ? validators.batch
        : validators.pageView;
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
