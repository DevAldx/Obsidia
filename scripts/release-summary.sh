#!/bin/bash
# Imprime el resumen de unas notas de versión para el aviso de actualización
# del launcher (campo `notes` de latest.json): el primer párrafo bajo
# «## Cambios», en una sola línea y sin marcas de Markdown.
#
# Uso: ./scripts/release-summary.sh PATCH_NOTES_X.Y.Z.md [texto por defecto]
set -euo pipefail

FILE="${1:?Uso: release-summary.sh PATCH_NOTES_X.Y.Z.md [texto por defecto]}"
FALLBACK="${2:-Actualización de Obsidia}"

SUMMARY=""
if [[ -f "$FILE" ]]; then
  SUMMARY=$(awk '
    /^## Cambios/ { inside = 1; next }
    inside && /^#/ { if (started) exit; next }
    inside && /^[[:space:]]*$/ { if (started) exit; next }
    inside { started = 1; printf "%s ", $0 }
  ' "$FILE" | sed -E 's/\*\*//g; s/`//g; s/[[:space:]]+/ /g; s/^ //; s/ $//')
fi

if [[ -n "$SUMMARY" ]]; then
  printf '%s\n' "$SUMMARY"
else
  printf '%s\n' "$FALLBACK"
fi
