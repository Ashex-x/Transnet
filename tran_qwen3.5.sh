#!/bin/bash

# Script to launch llama-server with Qwen3.5 27B Q8 model
# Model path: /mnt/newland/model_parms/qwen3_27b_q8/Qwen_Qwen3.5-27B-Q8_0.gguf

MODEL_PATH="/mnt/newland/model_parms/qwen3_27b_q8/Qwen_Qwen3.5-27B-Q8_0.gguf"

# Server configuration
PORT=35794
HOST="127.0.0.1"
CTX_SIZE=8192
N_GPU_LAYERS=-1  # -1 means use all available GPU layers

echo "Starting llama-server..."
echo "Model: $MODEL_PATH"
echo "Host: $HOST"
echo "Port: $PORT"
echo "Context size: $CTX_SIZE"

../tools/llama.cpp/build/bin/llama-server \
  -m "$MODEL_PATH" \
  --host "$HOST" \
  --port "$PORT" \
  --ctx-size "$CTX_SIZE" \
  --n-gpu-layers "$N_GPU_LAYERS"