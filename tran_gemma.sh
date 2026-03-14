#!/bin/bash

# Script to launch llama-server with TranslateGemma 27B Q8 model
# Model path: /mnt/newland/model_parms/translategemma_27b_q8/translategemma-27b-it-Q8_0.gguf

MODEL_PATH="/mnt/newland/model_parms/translategemma_27b_q8/translategemma-27b-it-Q8_0.gguf"

# Server configuration
PORT=35793
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

# ---- Text Translation ----
# messages = [
#   {
#     "role": "user",
#     "content": [
#       {
#         "type": "text",
#         "source_lang_code": "cs",
#         "target_lang_code": "de-DE",
#         "text": "V nejhorším případě i k prasknutí čočky.",
#       }
#     ],
#   }
# ]
# 
# output = pipe(text=messages, max_new_tokens=200)
# print(output[0]["generated_text"][-1]["content"])
# 
# ---- Text Extraction and Translation ----
# messages = [
#   {
#     "role": "user",
#     "content": [
#       {
#         "type": "image",
#         "source_lang_code": "cs",
#         "target_lang_code": "de-DE",
#         "url": "https://c7.alamy.com/comp/2YAX36N/traffic-signs-in-czech-republic-pedestrian-zone-2YAX36N.jpg",
#       },
#     ],
#   }
# ]
