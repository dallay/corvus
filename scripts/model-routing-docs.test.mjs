import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

function readText(path) {
  return fs.readFileSync(path, "utf8");
}

const docs = [
  ["EN", readText("clients/web/apps/docs/src/content/docs/guides/model-routing.md")],
  ["ES", readText("clients/web/apps/docs/src/content/docs/es/guides/model-routing.md")],
];

for (const [label, doc] of docs) {
  test(`${label} model routing guide lets operators configure routing using docs only`, () => {
    assert.match(doc, /\[\[model_routes\]\]/);
    assert.match(doc, /\[query_classification\]/);
    assert.match(doc, /corvus doctor/);
    assert.match(doc, /fast/);
    assert.match(doc, /reasoning/);
  });

  test(`${label} model routing guide covers required config fields`, () => {
    for (const field of [
      "hint",
      "provider",
      "model",
      "api_key",
      "allow_image_input",
      "enabled",
      "keywords",
      "patterns",
      "min_length",
      "max_length",
      "priority",
    ]) {
      assert.match(doc, new RegExp(`\\\`${field}\\\``));
    }
  });

  test(`${label} model routing guide explains hint flow and default-model behavior`, () => {
    assert.match(doc, /classification|clasificación/i);
    assert.match(doc, /router/i);
    assert.match(doc, /default model|modelo por defecto/i);
    assert.match(doc, /default provider|provider por defecto/i);
  });

  test(`${label} model routing guide includes troubleshooting for common misconfigurations`, () => {
    assert.match(doc, /orphaned hint|hint huérfano/i);
    assert.match(doc, /no rules are configured|sin reglas/i);
    assert.match(doc, /never match|nunca va a coincidir/i);
    assert.match(doc, /corvus doctor/);
  });
}
