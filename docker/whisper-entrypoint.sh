#!/bin/sh
set -e

MODEL_NAME="${WHISPER_MODEL:-small}"
MODEL_PATH="/models/ggml-${MODEL_NAME}.bin"

if [ ! -f "${MODEL_PATH}" ]; then
  echo "Downloading Whisper model '${MODEL_NAME}' to /models..."
  ./models/download-ggml-model.sh "${MODEL_NAME}" /models
fi

echo "Starting whisper-server (${MODEL_NAME})..."
exec ./build/bin/whisper-server \
  -m "${MODEL_PATH}" \
  --host 0.0.0.0 \
  --port 9000
