#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
KEEP_TEMP="${KEEP_TEMP:-0}"
PACK_DIR="${TMP_DIR}/pack"
PACK_ID="ai.greentic.component-slot-extractor-test"
FLOW_ID="ai.greentic.component-slot-extractor.test"
NODE_ID="slot_extractor_step"
COMPONENT_ID="ai.greentic.component-slot-extractor"
FLOW_FILE="${PACK_DIR}/flows/slot-extractor.ygtc"
WASM_SRC="${ROOT_DIR}/target/wasm32-wasip2/release/component_slot_extractor.wasm"

cleanup() {
  if [[ "${KEEP_TEMP}" == "1" ]]; then
    return
  fi
  rm -rf "${TMP_DIR}"
}
trap cleanup EXIT

echo "Temp dir: ${TMP_DIR}"

make -C "${ROOT_DIR}" build

if [[ ! -f "${WASM_SRC}" ]]; then
  echo "Missing wasm artifact at ${WASM_SRC}" >&2
  echo "Smoke test failed." >&2
  exit -1
fi

greentic-pack new --dir "${PACK_DIR}" "${PACK_ID}"
mkdir -p "${PACK_DIR}/components"
cp "${WASM_SRC}" "${PACK_DIR}/components/${COMPONENT_ID}.wasm"
cp "${ROOT_DIR}/component.manifest.json" "${PACK_DIR}/components/component.manifest.json"
sed -i \
  "s|\"component_wasm\": \"target/wasm32-wasip2/release/component_slot_extractor.wasm\"|\"component_wasm\": \"${COMPONENT_ID}.wasm\"|" \
  "${PACK_DIR}/components/component.manifest.json"

greentic-flow new \
  --flow "${FLOW_FILE}" \
  --id "${FLOW_ID}" \
  --type messaging \
  --name "Slot extractor smoke" \
  --description "Smoke test for component-slot-extractor"

cat > "${TMP_DIR}/payload.json" <<'JSON'
{
  "utterance": "I want to refund order 42",
  "slot_definitions": [
    {
      "name": "order_id",
      "slot_type": "number",
      "pattern": "order (\\d+)",
      "required": true
    }
  ]
}
JSON

greentic-flow add-step \
  --flow "${FLOW_FILE}" \
  --node-id "${NODE_ID}" \
  --operation extract_slots \
  --payload "$(cat "${TMP_DIR}/payload.json")" \
  --local-wasm "${PACK_DIR}/components/${COMPONENT_ID}.wasm" \
  --routing-out

greentic-pack update --in "${PACK_DIR}"
greentic-pack build --in "${PACK_DIR}"

echo "Smoke test passed."
