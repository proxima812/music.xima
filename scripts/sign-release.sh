#!/usr/bin/env bash
#
# Подпись release-APK ключом разработчика.
#
# Почему отдельным шагом, а не через `signingConfig` в Gradle: конфиг пришлось бы
# держать в `src-tauri/gen/android`, а эта папка в `.gitignore` и перезаписывается
# каждым `tauri android init` (docs/BUGS.md, B1) — подпись там не переживёт
# регенерацию. Скрипт лежит в репозитории и переживает.
#
#   MX_KEYSTORE_PASS='…' scripts/sign-release.sh [путь-к-unsigned.apk]
#
# Переменные окружения:
#   MX_KEYSTORE       путь к хранилищу (по умолчанию ~/.android/music-xima-release.jks)
#   MX_KEY_ALIAS      имя ключа       (по умолчанию music-xima)
#   MX_KEYSTORE_PASS  пароль; если не задан — скрипт спросит его, не показывая ввод
#
set -euo pipefail

KEYSTORE="${MX_KEYSTORE:-$HOME/.android/music-xima-release.jks}"
ALIAS="${MX_KEY_ALIAS:-music-xima}"

DEFAULT_APK='src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk'
UNSIGNED="${1:-$DEFAULT_APK}"

[ -f "$KEYSTORE" ] || { echo "нет хранилища ключей: $KEYSTORE" >&2; exit 1; }
[ -f "$UNSIGNED" ] || { echo "нет APK: $UNSIGNED — сначала npm run android:build -- --apk" >&2; exit 1; }

# Инструменты берём из самой свежей версии build-tools.
SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Library/Android/sdk}}"
TOOLS="$(find "$SDK/build-tools" -maxdepth 1 -mindepth 1 -type d 2>/dev/null | sort -V | tail -1)"
[ -n "$TOOLS" ] || { echo "не нашёл build-tools в $SDK" >&2; exit 1; }

if [ -z "${MX_KEYSTORE_PASS:-}" ]; then
  read -r -s -p "Пароль от $KEYSTORE: " MX_KEYSTORE_PASS
  echo
  export MX_KEYSTORE_PASS
fi

SIGNED="${UNSIGNED%-unsigned.apk}-signed.apk"
ALIGNED="$(mktemp -t mx-aligned).apk"
trap 'rm -f "$ALIGNED"' EXIT

# `-P 16` кладёт нативные библиотеки на границу 16 КБ: новые Android работают на
# 16-килобайтных страницах памяти и невыровненную .so грузить отказываются (B3).
"$TOOLS/zipalign" -P 16 -f 4 "$UNSIGNED" "$ALIGNED"

"$TOOLS/apksigner" sign \
  --ks "$KEYSTORE" \
  --ks-key-alias "$ALIAS" \
  --ks-pass "env:MX_KEYSTORE_PASS" \
  --key-pass "env:MX_KEYSTORE_PASS" \
  --out "$SIGNED" \
  "$ALIGNED"

"$TOOLS/apksigner" verify --print-certs "$SIGNED"
"$TOOLS/zipalign" -c -P 16 4 "$SIGNED" && echo "выравнивание: ок"

echo
echo "подписано: $SIGNED"
