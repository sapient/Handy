#!/usr/bin/env bash

set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script currently supports macOS only."
  exit 1
fi

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1"
    exit 1
  fi
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
MODEL_DIR="${REPO_ROOT}/src-tauri/resources/models"
MODEL_PATH="${MODEL_DIR}/silero_vad_v4.onnx"
MODEL_URL="https://blob.handy.computer/silero_vad_v4.onnx"
APP_NAME="Handy.app"
APP_SOURCE="${REPO_ROOT}/src-tauri/target/release/bundle/macos/${APP_NAME}"
INSTALL_DIR="${HANDY_INSTALL_DIR:-/Applications}"
INSTALL_PATH="${INSTALL_DIR}/${APP_NAME}"
BUILD_LOG="$(mktemp -t handy-build.XXXXXX.log)"

cleanup() {
  rm -f "${BUILD_LOG}"
}

trap cleanup EXIT

require_cmd bun
require_cmd cargo
require_cmd curl
require_cmd rsync

can_write_install_path() {
  [[ -d "${INSTALL_PATH}" && -w "${INSTALL_PATH}" ]] ||
    [[ -d "${INSTALL_DIR}" && -w "${INSTALL_DIR}" ]] ||
    [[ ! -e "${INSTALL_DIR}" && -w "$(dirname "${INSTALL_DIR}")" ]]
}

if ! xcode-select -p >/dev/null 2>&1; then
  echo "Xcode Command Line Tools are required. Run: xcode-select --install"
  exit 1
fi

echo "Installing JavaScript dependencies with Bun..."
cd "${REPO_ROOT}"
bun install

if [[ ! -f "${MODEL_PATH}" ]]; then
  echo "Downloading Silero VAD model..."
  mkdir -p "${MODEL_DIR}"
  curl -fL "${MODEL_URL}" -o "${MODEL_PATH}"
else
  echo "Silero VAD model already present."
fi

echo "Building Handy.app..."
BUILD_STARTED_AT="$(date +%s)"
set +e
bun run tauri build 2>&1 | tee "${BUILD_LOG}"
BUILD_STATUS=${PIPESTATUS[0]}
set -e

if [[ "${BUILD_STATUS}" -ne 0 ]]; then
  if [[ -d "${APP_SOURCE}" ]] &&
    [[ "$(stat -f %m "${APP_SOURCE}")" -ge "${BUILD_STARTED_AT}" ]] &&
    grep -q "TAURI_SIGNING_PRIVATE_KEY" "${BUILD_LOG}"; then
    echo "Updater signing key is not configured, but Handy.app was built successfully."
    echo "Continuing with local installation."
  else
    echo "Build failed."
    exit "${BUILD_STATUS}"
  fi
fi

if [[ ! -d "${APP_SOURCE}" ]]; then
  echo "Build completed, but ${APP_SOURCE} was not found."
  exit 1
fi

echo "Installing ${APP_NAME} to ${INSTALL_DIR}..."
if can_write_install_path; then
  mkdir -p "${INSTALL_PATH}"
  rsync -a --delete "${APP_SOURCE}/" "${INSTALL_PATH}/"
else
  sudo mkdir -p "${INSTALL_PATH}"
  sudo rsync -a --delete "${APP_SOURCE}/" "${INSTALL_PATH}/"
fi

echo "Installed to ${INSTALL_PATH}"
echo "Binary path: ${INSTALL_PATH}/Contents/MacOS/Handy"
