@echo off
set "TARGET=%APPDATA%\.hayride\ai\models"
mkdir "%TARGET%" 2>nul

if exist "%TARGET%\Meta-Llama-3.1-8B-Instruct-Q5_K_M.gguf" (
  echo File already exists, skipping download.
  exit /b 0
)

curl -L "https://huggingface.co/bartowski/Meta-Llama-3.1-8B-Instruct-GGUF/resolve/main/Meta-Llama-3.1-8B-Instruct-Q5_K_M.gguf" -o "%TARGET%\Meta-Llama-3.1-8B-Instruct-Q5_K_M.gguf"
